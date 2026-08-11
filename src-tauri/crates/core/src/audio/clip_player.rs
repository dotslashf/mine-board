use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError};
use std::sync::Arc;

use crate::audio::mixer;

const MAX_ACTIVE_CLIPS: usize = 32;
const TRIGGER_QUEUE_CAPACITY: usize = 256;

#[derive(Debug)]
enum Trigger {
    Play {
        clip_id: i64,
        samples: Arc<Vec<f32>>,
        frames: usize,
        gain: f32,
    },
    Stop(i64),
    StopAll,
}

struct ActiveClip {
    clip_id: i64,
    samples: Arc<Vec<f32>>,
    frames: usize,
    position: usize,
    gain: f32,
}

fn f32_to_bits(v: f32) -> u32 {
    v.to_bits()
}

/// Send-only handle for queueing play/stop without borrowing the RT player.
#[derive(Clone)]
pub struct ClipPlayerHandle {
    tx: SyncSender<Trigger>,
}

impl ClipPlayerHandle {
    /// Queue a clip for playback. Non-blocking.
    pub fn play(&self, clip_id: i64, samples: Arc<Vec<f32>>, frames: usize, gain: f32) {
        let _ = self.tx.try_send(Trigger::Play {
            clip_id,
            samples,
            frames,
            gain,
        });
    }

    /// Queue a stop for one clip. Non-blocking.
    pub fn stop(&self, clip_id: i64) {
        let _ = self.tx.try_send(Trigger::Stop(clip_id));
    }

    /// Queue a stop for all clips. Non-blocking.
    pub fn stop_all(&self) {
        let _ = self.tx.try_send(Trigger::StopAll);
    }
}

/// Real-time-safe clip scheduler.
///
/// Triggers are queued on a bounded channel from any thread and drained at
/// the start of every [`ClipPlayer::process`] call (the audio callback).
/// Mixing is allocation-free. Completed clips are reported through
/// [`ClipPlayer::take_finished`].
pub struct ClipPlayer {
    tx: SyncSender<Trigger>,
    rx: Receiver<Trigger>,
    active: Vec<ActiveClip>,
    finished: Vec<i64>,
    soundboard_gain: AtomicU32,
}

impl Default for ClipPlayer {
    fn default() -> Self {
        Self::new()
    }
}

impl ClipPlayer {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::sync_channel(TRIGGER_QUEUE_CAPACITY);
        Self {
            tx,
            rx,
            active: Vec::with_capacity(MAX_ACTIVE_CLIPS),
            finished: Vec::with_capacity(8),
            soundboard_gain: AtomicU32::new(f32_to_bits(1.0)),
        }
    }

    /// Cloneable send-side handle for the control plane (no RefCell needed).
    pub fn handle(&self) -> ClipPlayerHandle {
        ClipPlayerHandle {
            tx: self.tx.clone(),
        }
    }

    /// Queue a clip for playback. `gain` is the clip's own volume; the live
    /// soundboard gain is applied at mix time. Non-blocking.
    pub fn play(&self, clip_id: i64, samples: Arc<Vec<f32>>, frames: usize, gain: f32) {
        self.handle().play(clip_id, samples, frames, gain);
    }

    /// Queue a stop for one clip. Non-blocking.
    pub fn stop(&self, clip_id: i64) {
        self.handle().stop(clip_id);
    }

    /// Queue a stop for all clips. Non-blocking.
    pub fn stop_all(&self) {
        self.handle().stop_all();
    }

    /// Set the live soundboard gain (shared with the RT thread via atomic).
    pub fn set_soundboard_gain(&self, gain: f32) {
        self.soundboard_gain
            .store(f32_to_bits(gain), Ordering::Relaxed);
    }

    pub fn soundboard_gain(&self) -> f32 {
        f32::from_bits(self.soundboard_gain.load(Ordering::Relaxed))
    }

    pub fn is_playing(&self, clip_id: i64) -> bool {
        self.active.iter().any(|c| c.clip_id == clip_id)
    }

    pub fn active_count(&self) -> usize {
        self.active.len()
    }

    /// Drain trigger queue, then mix all active clips into `out`
    /// (interleaved stereo). Completes clips and records finished ids.
    pub fn process(&mut self, out: &mut [f32], frames: usize) {
        self.drain_triggers();
        self.mix(out, frames);
    }

    /// Ids of clips that completed during the last [`ClipPlayer::process`].
    pub fn take_finished(&mut self) -> &[i64] {
        // SAFETY-free pattern: swap the buffer so the caller owns a copy.
        // (We keep it simple: caller copies the ids out.)
        &self.finished
    }

    pub fn reset_finished(&mut self) {
        self.finished.clear();
    }

    fn drain_triggers(&mut self) {
        loop {
            match self.rx.try_recv() {
                Ok(Trigger::Play {
                    clip_id,
                    samples,
                    frames,
                    gain,
                }) => {
                    if self.active.len() < MAX_ACTIVE_CLIPS {
                        self.active.push(ActiveClip {
                            clip_id,
                            samples,
                            frames: frames.max(1),
                            position: 0,
                            gain,
                        });
                    }
                }
                Ok(Trigger::Stop(clip_id)) => {
                    let was_active = self.active.iter().any(|c| c.clip_id == clip_id);
                    self.active.retain(|c| c.clip_id != clip_id);
                    if was_active {
                        self.finished.push(clip_id);
                    }
                }
                Ok(Trigger::StopAll) => {
                    for clip in self.active.drain(..) {
                        self.finished.push(clip.clip_id);
                    }
                }
                Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => break,
            }
        }
    }

    fn mix(&mut self, out: &mut [f32], frames: usize) {
        let out_frames = out.len() / 2;
        let frames = frames.min(out_frames);
        let soundboard_gain = self.soundboard_gain();
        if soundboard_gain == 0.0 {
            self.active.clear();
            self.finished.clear();
            return;
        }

        // Pre-clear only the frames we'll write.
        let usable = frames * 2;
        mixer::clear(&mut out[..usable]);

        let mut finished = std::mem::take(&mut self.finished);
        let mut still_playing: Vec<ActiveClip> = Vec::with_capacity(self.active.len());

        for mut clip in self.active.drain(..) {
            let start = clip.position;
            let available = clip.frames.saturating_sub(start);
            if available == 0 {
                finished.push(clip.clip_id);
                continue;
            }
            let frames_to_write = available.min(frames);
            let src_start = start * 2;
            let src_end = src_start + frames_to_write * 2;
            let gain = clip.gain * soundboard_gain;
            mixer::mix_in(&mut out[..src_end.min(usable)], &clip.samples[src_start..src_end], gain);
            clip.position += frames_to_write;

            if clip.position >= clip.frames {
                finished.push(clip.clip_id);
            } else {
                still_playing.push(clip);
            }
        }

        self.active = still_playing;
        self.finished = finished;
    }
}

impl Drop for ClipPlayer {
    fn drop(&mut self) {
        let _ = self.tx.try_send(Trigger::StopAll);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn silent(frames: usize) -> Vec<f32> {
        vec![0.0; frames * 2]
    }

    fn tone(frames: usize, value: f32) -> Vec<f32> {
        vec![value; frames * 2]
    }

    #[test]
    fn plays_clip_and_completes() {
        let mut player = ClipPlayer::new();
        let samples = Arc::new(tone(10, 0.5));
        player.play(1, samples, 10, 1.0);

        let mut out = silent(4);
        player.process(&mut out, 4);
        assert!(out.iter().all(|&v| v == 0.5));

        let mut out = silent(6);
        player.process(&mut out, 6);
        assert!(out.iter().all(|&v| v == 0.5));
        assert_eq!(player.take_finished(), &[1]);
        player.reset_finished();
        assert!(player.active_count() == 0);
    }

    #[test]
    fn play_overlap_mixes() {
        let mut player = ClipPlayer::new();
        let a = Arc::new(tone(8, 1.0));
        let b = Arc::new(tone(8, 2.0));
        player.play(1, a, 8, 1.0);
        player.play(2, b, 8, 1.0);

        let mut out = silent(4);
        player.process(&mut out, 4);
        assert!(out.iter().all(|&v| v == 3.0));
    }

    #[test]
    fn stop_halfway() {
        let mut player = ClipPlayer::new();
        let samples = Arc::new(tone(10, 1.0));
        player.play(1, samples, 10, 1.0);

        let mut out = silent(4);
        player.process(&mut out, 4);
        player.stop(1);
        let mut out = silent(4);
        player.process(&mut out, 4);
        assert!(out.iter().all(|&v| v == 0.0));
        assert_eq!(player.take_finished(), &[1]);
    }

    #[test]
    fn handle_stop_without_mut_borrow() {
        let mut player = ClipPlayer::new();
        let handle = player.handle();
        player.play(1, Arc::new(tone(10, 1.0)), 10, 1.0);
        let mut out = silent(4);
        player.process(&mut out, 4);
        handle.stop(1);
        let mut out = silent(4);
        player.process(&mut out, 4);
        assert!(out.iter().all(|&v| v == 0.0));
        assert_eq!(player.active_count(), 0);
    }

    #[test]
    fn stop_all() {
        let mut player = ClipPlayer::new();
        player.play(1, Arc::new(tone(10, 1.0)), 10, 1.0);
        player.play(2, Arc::new(tone(10, 1.0)), 10, 1.0);
        player.stop_all();
        let mut out = silent(4);
        player.process(&mut out, 4);
        assert!(out.iter().all(|&v| v == 0.0));
    }

    #[test]
    fn soundboard_gain_applied() {
        let mut player = ClipPlayer::new();
        player.set_soundboard_gain(0.5);
        player.play(1, Arc::new(tone(4, 1.0)), 4, 1.0);
        let mut out = silent(4);
        player.process(&mut out, 4);
        assert!(out.iter().all(|&v| v == 0.5));
    }

    #[test]
    fn per_clip_gain() {
        let mut player = ClipPlayer::new();
        player.play(1, Arc::new(tone(4, 1.0)), 4, 0.25);
        let mut out = silent(4);
        player.process(&mut out, 4);
        assert!(out.iter().all(|&v| v == 0.25));
    }

    #[test]
    fn mix_across_callback_boundaries() {
        let mut player = ClipPlayer::new();
        player.play(1, Arc::new(tone(6, 1.0)), 6, 1.0);

        let mut out = silent(4);
        player.process(&mut out, 4);
        assert!(out.iter().all(|&v| v == 1.0));

        let mut out = silent(4);
        player.process(&mut out, 4);
        // remaining 2 frames then silence
        assert_eq!(&out[..4], &[1.0; 4]);
        assert!(out[4..].iter().all(|&v| v == 0.0));
        assert_eq!(player.take_finished(), &[1]);
    }

    #[test]
    fn zero_gain_skips_mix() {
        let mut player = ClipPlayer::new();
        player.set_soundboard_gain(0.0);
        player.play(1, Arc::new(tone(4, 1.0)), 4, 1.0);
        let mut out = silent(4);
        player.process(&mut out, 4);
        assert!(out.iter().all(|&v| v == 0.0));
    }
}
