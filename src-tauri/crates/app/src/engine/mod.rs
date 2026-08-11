//! PipeWire audio engine.
//!
//! All PipeWire objects are created and driven on a single dedicated thread
//! running the PipeWire main loop. The control plane talks to it through a
//! [`pipewire::channel`] command channel; real-time process callbacks stay
//! allocation-free (scratch buffers are pre-allocated and reused).

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use atcs_soundboard_core::audio::clip_player::{ClipPlayer, ClipPlayerHandle};
use atcs_soundboard_core::audio::mixer;
use atcs_soundboard_core::models::{
    AudioDevice, AudioStatus, MicStatus, SoundStoppedPayload, VirtualMicStatus,
};
use atcs_soundboard_core::db::Repository;
use atcs_soundboard_core::settings as settings_core;
use atcs_soundboard_core::{TARGET_CHANNELS, TARGET_SAMPLE_RATE};

use pipewire as pw;
use pw::properties::properties;
use pw::stream::StreamFlags;
use pw::spa;
use spa::param::audio::{AudioFormat, AudioInfoRaw};
use spa::param::ParamType;
use spa::pod::{Pod, Value};
use spa::utils::SpaTypes;
use tauri::{AppHandle, Emitter};

const VIRTUAL_MIC_NAME: &str = "ATCS Soundboard Virtual Mic";
const VIRTUAL_MIC_NODE_NAME: &str = "atcs-soundboard-virtual-mic";
const MIC_NODE_NAME: &str = "atcs-soundboard-mic";
const MONITOR_NODE_NAME: &str = "atcs-soundboard-monitor";
const APP_ID: &str = "atcs.soundboard";

/// One second of stereo f32 samples, used as the mic ring-buffer capacity.
const RING_CAPACITY: usize = 1 << 18; // 2^18 samples ≈ 1.36s of stereo @48k

pub enum EngineCommand {
    Stop,
    SetMicVolume(f32),
    SetSoundboardVolume(f32),
    SetMasterVolume(f32),
    SetMicMuted(bool),
    SetMonitorEnabled(bool),
    SelectMicrophone(Option<u32>),
    PlayClip {
        clip_id: i64,
        name: String,
        samples: Arc<Vec<f32>>,
        frames: usize,
        gain: f32,
    },
    StopClip(i64),
    StopAll,
}

fn f32_bits(v: f32) -> u32 {
    v.to_bits()
}
fn f32_of(bits: u32) -> f32 {
    f32::from_bits(bits)
}

/// State shared between the control plane and the real-time callbacks.
pub struct EngineShared {
    ring: Mutex<(rtrb::Producer<f32>, rtrb::Consumer<f32>)>,
    last_mix: Mutex<Vec<f32>>,
    mic_volume: AtomicU32,
    soundboard_volume: AtomicU32,
    master_volume: AtomicU32,
    mic_muted: AtomicBool,
    monitor_enabled: AtomicBool,
    mic_node_id: AtomicU32,
    mic_connected: AtomicBool,
    mic_error: Mutex<Option<String>>,
    vmic_node_id: AtomicU32,
    vmic_error: Mutex<Option<String>>,
    request_stop: AtomicBool,
}

impl EngineShared {
    fn new(settings: &atcs_soundboard_core::models::AudioSettings) -> Arc<Self> {
        Arc::new(Self {
            ring: Mutex::new(rtrb::RingBuffer::new(RING_CAPACITY)),
            last_mix: Mutex::new(Vec::new()),
            mic_volume: AtomicU32::new(f32_bits(settings.microphone_volume as f32)),
            soundboard_volume: AtomicU32::new(f32_bits(settings.soundboard_volume as f32)),
            master_volume: AtomicU32::new(f32_bits(settings.master_volume as f32)),
            mic_muted: AtomicBool::new(false),
            monitor_enabled: AtomicBool::new(settings.monitor_enabled),
            mic_node_id: AtomicU32::new(0),
            mic_connected: AtomicBool::new(false),
            mic_error: Mutex::new(None),
            vmic_node_id: AtomicU32::new(0),
            vmic_error: Mutex::new(None),
            request_stop: AtomicBool::new(false),
        })
    }

    pub fn set_mic_volume(&self, v: f32) {
        self.mic_volume.store(f32_bits(v.clamp(0.0, 2.0)), Ordering::Relaxed);
    }
    pub fn mic_volume(&self) -> f32 {
        f32_of(self.mic_volume.load(Ordering::Relaxed))
    }
    pub fn set_soundboard_volume(&self, v: f32) {
        self.soundboard_volume
            .store(f32_bits(v.clamp(0.0, 2.0)), Ordering::Relaxed);
    }
    pub fn soundboard_volume(&self) -> f32 {
        f32_of(self.soundboard_volume.load(Ordering::Relaxed))
    }
    pub fn set_master_volume(&self, v: f32) {
        self.master_volume.store(f32_bits(v.clamp(0.0, 2.0)), Ordering::Relaxed);
    }
    pub fn master_volume(&self) -> f32 {
        f32_of(self.master_volume.load(Ordering::Relaxed))
    }
    pub fn set_mic_muted(&self, muted: bool) {
        self.mic_muted.store(muted, Ordering::Relaxed);
    }
    pub fn set_monitor_enabled(&self, on: bool) {
        self.monitor_enabled.store(on, Ordering::Relaxed);
    }
    pub fn monitor_enabled(&self) -> bool {
        self.monitor_enabled.load(Ordering::Relaxed)
    }

    pub fn mic_status(&self) -> MicStatus {
        let node_id = self.mic_node_id.load(Ordering::Relaxed);
        MicStatus {
            connected: self.mic_connected.load(Ordering::Relaxed),
            muted: self.mic_muted.load(Ordering::Relaxed),
            device_id: (node_id != 0).then_some(node_id),
            device_name: None,
            error: self.mic_error.lock().unwrap().clone(),
        }
    }

    pub fn vmic_status(&self) -> VirtualMicStatus {
        let node_id = self.vmic_node_id.load(Ordering::Relaxed);
        VirtualMicStatus {
            running: node_id != 0,
            node_id: (node_id != 0).then_some(node_id),
            name: VIRTUAL_MIC_NAME.to_string(),
            error: self.vmic_error.lock().unwrap().clone(),
        }
    }
}

/// RT-side state for the virtual microphone stream.
struct VmicData {
    shared: Arc<EngineShared>,
    player: ClipPlayer,
    mix_buf: Vec<f32>,
    mic_buf: Vec<f32>,
    connected: bool,
}

impl VmicData {
    fn new(shared: Arc<EngineShared>) -> Self {
        Self {
            shared,
            player: ClipPlayer::new(),
            mix_buf: Vec::new(),
            mic_buf: Vec::new(),
            connected: false,
        }
    }

    fn status(&self) -> VirtualMicStatus {
        let mut status = self.shared.vmic_status();
        status.running = self.connected;
        status.node_id = self.connected.then_some(self.shared.vmic_node_id.load(Ordering::Relaxed));
        status
    }

    fn process(&mut self, stream: &pw::stream::StreamRef) {
        let Some(mut buffer) = stream.dequeue_buffer() else {
            return;
        };
        let datas = buffer.datas_mut();
        if datas.is_empty() {
            return;
        }
        let data = &mut datas[0];
        let Some(slice) = data.data() else {
            return;
        };

        let stride = std::mem::size_of::<f32>() * usize::from(TARGET_CHANNELS);
        let (_, samples, _) = unsafe { slice.align_to_mut::<f32>() };
        let out_frames = samples.len() / usize::from(TARGET_CHANNELS);
        if out_frames == 0 {
            return;
        }

        self.player.set_soundboard_gain(self.shared.soundboard_volume());
        self.mix_buf.resize(samples.len(), 0.0);
        self.player.process(&mut self.mix_buf, out_frames);

        // Microphone input.
        let mic_gain = if self.shared.mic_muted.load(Ordering::Relaxed) {
            0.0
        } else {
            self.shared.mic_volume()
        };
        self.mic_buf.resize(samples.len(), 0.0);
        if let Ok(mut ring) = self.shared.ring.lock() {
            let _ = ring.1.pop_partial_slice(&mut self.mic_buf[..]);
        }
        mixer::mix_in(&mut self.mix_buf, &self.mic_buf, mic_gain);

        let master = self.shared.master_volume();
        mixer::apply_gain(&mut self.mix_buf, master);
        mixer::soft_clip(&mut self.mix_buf);

        samples.copy_from_slice(&self.mix_buf);

        let chunk = data.chunk_mut();
        *chunk.offset_mut() = 0;
        *chunk.stride_mut() = stride as i32;
        *chunk.size_mut() = (out_frames * stride) as u32;

        // Snapshot the mix so the monitor stream can replay it.
        if let Ok(mut last) = self.shared.last_mix.lock() {
            last.clear();
            last.extend_from_slice(&self.mix_buf);
        }
    }
}

/// RT-side state for the physical microphone capture stream.
struct MicData {
    shared: Arc<EngineShared>,
}

impl MicData {
    fn process(&mut self, stream: &pw::stream::StreamRef) {
        let Some(mut buffer) = stream.dequeue_buffer() else {
            return;
        };
        let datas = buffer.datas_mut();
        if datas.is_empty() {
            return;
        }
        let data = &mut datas[0];
        let chunk = data.chunk();
        let chunk_size = chunk.size();
        if chunk_size == 0 {
            return;
        }
        let start = chunk.offset() as usize;
        let Some(slice) = data.data() else {
            return;
        };
        let end = (start + chunk_size as usize).min(slice.len());
        if start >= end {
            return;
        }
        let (_, samples, _) = unsafe { slice[start..end].align_to::<f32>() };
        if samples.is_empty() {
            return;
        }
        if let Ok(mut ring) = self.shared.ring.lock() {
            let _ = ring.0.push_partial_slice(samples);
        }
    }
}

/// RT-side state for the optional monitor (playback) stream.
struct MonitorData {
    shared: Arc<EngineShared>,
}

impl MonitorData {
    fn process(&mut self, stream: &pw::stream::StreamRef) {
        let Some(mut buffer) = stream.dequeue_buffer() else {
            return;
        };
        let datas = buffer.datas_mut();
        if datas.is_empty() {
            return;
        }
        let data = &mut datas[0];
        let Some(slice) = data.data() else {
            return;
        };
        let stride = std::mem::size_of::<f32>() * usize::from(TARGET_CHANNELS);
        let (_, samples, _) = unsafe { slice.align_to_mut::<f32>() };
        let out_frames = samples.len() / usize::from(TARGET_CHANNELS);
        if out_frames == 0 {
            return;
        }
        let copy;
        {
            let last = self.shared.last_mix.lock().unwrap();
            copy = samples.len().min(last.len());
            samples[..copy].copy_from_slice(&last[..copy]);
        }
        samples[copy..].fill(0.0);
        let chunk = data.chunk_mut();
        *chunk.offset_mut() = 0;
        *chunk.stride_mut() = stride as i32;
        *chunk.size_mut() = (out_frames * stride) as u32;
    }
}

struct MicStream {
    _stream: pw::stream::Stream,
    _listener: pw::stream::StreamListener<MicData>,
}

struct MonitorStream {
    stream: pw::stream::Stream,
    _listener: pw::stream::StreamListener<MonitorData>,
}

struct VmicStream {
    _stream: pw::stream::Stream,
    _listener: pw::stream::StreamListener<Rc<RefCell<VmicData>>>,
}

/// Serialized EnumFormat pod (F32LE / 48kHz / stereo) shared by all streams.
fn format_pod_bytes() -> Vec<u8> {
    let mut audio_info = AudioInfoRaw::new();
    audio_info.set_format(AudioFormat::F32LE);
    audio_info.set_rate(TARGET_SAMPLE_RATE);
    audio_info.set_channels(u32::from(TARGET_CHANNELS));
    let values: Vec<u8> = pw::spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &Value::Object(pw::spa::pod::Object {
            type_: SpaTypes::ObjectParamFormat.as_raw(),
            id: ParamType::EnumFormat.as_raw(),
            properties: audio_info.into(),
        }),
    )
    .map(|(cursor, _)| cursor.into_inner())
    .unwrap_or_default();
    values
}

fn connect_flags_rt() -> StreamFlags {
    StreamFlags::MAP_BUFFERS | StreamFlags::RT_PROCESS
}

fn create_mic_stream(
    core: &pw::core::Core,
    shared: &Arc<EngineShared>,
    app: &AppHandle,
    pod_bytes: &[u8],
    target: Option<u32>,
) -> Result<MicStream, pw::Error> {
    let mut props = properties! {
        *pw::keys::MEDIA_TYPE => "Audio",
        *pw::keys::MEDIA_CATEGORY => "Capture",
        *pw::keys::MEDIA_ROLE => "Communication",
        *pw::keys::NODE_NAME => MIC_NODE_NAME,
        *pw::keys::NODE_DESCRIPTION => "ATCS Soundboard Microphone",
        *pw::keys::APP_ID => APP_ID,
        *pw::keys::APP_NAME => "ATCS Soundboard",
    };
    if let Some(id) = target {
        props.insert(*pw::keys::TARGET_OBJECT, id.to_string());
    }

    let stream = pw::stream::Stream::new(core, "atcs-soundboard-mic", props)?;
    let app_for_events = app.clone();
    let _listener = stream
        .add_local_listener_with_user_data(MicData {
            shared: shared.clone(),
        })
        .state_changed(move |_, data: &mut MicData, _old, new| {
            match new {
                pw::stream::StreamState::Streaming => {
                    data.shared.mic_connected.store(true, Ordering::Relaxed);
                    data.shared.mic_error.lock().unwrap().take();
                }
                pw::stream::StreamState::Error(e) => {
                    data.shared.mic_connected.store(false, Ordering::Relaxed);
                    *data.shared.mic_error.lock().unwrap() = Some(e);
                }
                pw::stream::StreamState::Unconnected => {
                    data.shared.mic_connected.store(false, Ordering::Relaxed);
                }
                _ => {}
            }
            let _ = app_for_events.emit(
                "microphone-status-changed",
                data.shared.mic_status(),
            );
        })
        .process(|stream, data: &mut MicData| data.process(stream))
        .register()?;

    let pod = Pod::from_bytes(pod_bytes).ok_or(pw::Error::CreationFailed)?;
    let mut params = [pod];
    stream.connect(
        spa::utils::Direction::Input,
        target,
        connect_flags_rt() | StreamFlags::AUTOCONNECT,
        &mut params,
    )?;

    Ok(MicStream { _stream: stream, _listener })
}

fn create_monitor_stream(
    core: &pw::core::Core,
    shared: &Arc<EngineShared>,
) -> Result<MonitorStream, pw::Error> {
    let props = properties! {
        *pw::keys::MEDIA_TYPE => "Audio",
        *pw::keys::MEDIA_CATEGORY => "Playback",
        *pw::keys::MEDIA_ROLE => "Communication",
        *pw::keys::NODE_NAME => MONITOR_NODE_NAME,
        *pw::keys::NODE_DESCRIPTION => "ATCS Soundboard Monitor",
        *pw::keys::APP_ID => APP_ID,
        *pw::keys::APP_NAME => "ATCS Soundboard",
    };
    let stream = pw::stream::Stream::new(core, "atcs-soundboard-monitor", props)?;
    let _listener = stream
        .add_local_listener_with_user_data(MonitorData {
            shared: shared.clone(),
        })
        .process(|stream, data: &mut MonitorData| data.process(stream))
        .register()?;
    let pod_bytes = format_pod_bytes();
    let pod = Pod::from_bytes(pod_bytes.as_slice()).ok_or(pw::Error::CreationFailed)?;
    let mut params = [pod];
    stream.connect(
        spa::utils::Direction::Output,
        None,
        connect_flags_rt() | StreamFlags::AUTOCONNECT | StreamFlags::INACTIVE,
        &mut params,
    )?;
    Ok(MonitorStream { stream, _listener })
}

struct PwState {
    core: pw::core::Core,
    shared: Arc<EngineShared>,
    app: AppHandle,
    /// Send-side play/stop — safe while RT holds `vmic_data`.
    player: ClipPlayerHandle,
    /// Held alive for the lifetime of the engine: dropping the stream would
    /// tear down the PipeWire source even while applications capture it.
    _vmic: VmicStream,
    mic: Option<MicStream>,
    monitor: Option<MonitorStream>,
}

fn handle_command(mainloop: &pw::main_loop::MainLoop, state: &RefCell<PwState>, cmd: EngineCommand) {
    match cmd {
        EngineCommand::Stop => {
            state.borrow().shared.request_stop.store(true, Ordering::Relaxed);
            mainloop.quit();
        }
        EngineCommand::SetMicVolume(v) => state.borrow().shared.set_mic_volume(v),
        EngineCommand::SetSoundboardVolume(v) => state.borrow().shared.set_soundboard_volume(v),
        EngineCommand::SetMasterVolume(v) => state.borrow().shared.set_master_volume(v),
        EngineCommand::SetMicMuted(muted) => {
            state.borrow().shared.set_mic_muted(muted);
            let st = state.borrow();
            let _ = st.app.emit("microphone-status-changed", st.shared.mic_status());
        }
        EngineCommand::SetMonitorEnabled(on) => {
            let mut st = state.borrow_mut();
            st.shared.set_monitor_enabled(on);
            match (on, &mut st.monitor) {
                (true, Some(m)) => {
                    let _ = m.stream.set_active(true);
                }
                (true, None) => {
                    st.monitor = create_monitor_stream(&st.core, &st.shared).ok();
                    if let Some(m) = &st.monitor {
                        let _ = m.stream.set_active(true);
                    }
                }
                (false, Some(m)) => {
                    let _ = m.stream.set_active(false);
                }
                (false, None) => {}
            }
        }
        EngineCommand::SelectMicrophone(target) => {
            let mut st = state.borrow_mut();
            st.shared.mic_node_id.store(target.unwrap_or(0), Ordering::Relaxed);
            st.shared.mic_connected.store(false, Ordering::Relaxed);
            st.shared.mic_error.lock().unwrap().take();
            st.mic = None; // drop old stream on the loop thread
            match create_mic_stream(&st.core, &st.shared, &st.app, format_pod_bytes().as_slice(), target) {
                Ok(mic) => {
                    st.mic = Some(mic);
                }
                Err(e) => {
                    *st.shared.mic_error.lock().unwrap() = Some(e.to_string());
                }
            }
            let _ = st.app.emit("microphone-status-changed", st.shared.mic_status());
        }
        EngineCommand::PlayClip {
            clip_id,
            name,
            samples,
            frames,
            gain,
        } => {
            let st = state.borrow();
            st.player.play(clip_id, samples, frames, gain);
            let _ = st.app.emit(
                "sound-started",
                atcs_soundboard_core::models::SoundEventPayload { clip_id, name },
            );
        }
        EngineCommand::StopClip(clip_id) => {
            let st = state.borrow();
            st.player.stop(clip_id);
            let _ = st.app.emit(
                "sound-stopped",
                SoundStoppedPayload {
                    clip_id,
                    name: String::new(),
                    reason: Some("stopped".into()),
                },
            );
        }
        EngineCommand::StopAll => {
            let st = state.borrow();
            st.player.stop_all();
        }
    }
}

fn run_engine(
    app: AppHandle,
    shared: Arc<EngineShared>,
    devices: Arc<Mutex<Vec<AudioDevice>>>,
    settings: atcs_soundboard_core::models::AudioSettings,
    cmd_rx: pw::channel::Receiver<EngineCommand>,
) -> Result<(), pw::Error> {
    pw::init();
    let mainloop = pw::main_loop::MainLoop::new(None)?;
    let context = pw::context::Context::new(&mainloop)?;
    let core = context.connect(None)?;

    // Device discovery.
    let registry = core.get_registry()?;
    let _registry_listener = {
        let devices_global = devices.clone();
        let devices_remove = devices.clone();
        registry
            .add_listener_local()
            .global(move |obj| {
                if obj.type_ != pw::types::ObjectType::Node {
                    return;
                }
                let props = match obj.props {
                    Some(p) => p,
                    None => return,
                };
                if props.get(*pw::keys::APP_ID) == Some(APP_ID) {
                    return; // hide our own nodes
                }
                let media_class = props.get(*pw::keys::MEDIA_CLASS).unwrap_or("").to_string();
                if media_class != "Audio/Source" && media_class != "Audio/Sink" {
                    return;
                }
                let node_name = props.get(*pw::keys::NODE_NAME).unwrap_or("").to_string();
                let description = props
                    .get(*pw::keys::NODE_DESCRIPTION)
                    .or_else(|| props.get(*pw::keys::NODE_NAME))
                    .unwrap_or("Unknown")
                    .to_string();
                let entry = AudioDevice {
                    id: obj.id,
                    name: description.clone(),
                    node_name,
                    description,
                    media_class,
                };
                let mut list = devices_global.lock().unwrap();
                if !list.iter().any(|d| d.id == entry.id) {
                    list.push(entry);
                    list.sort_by_key(|d| d.id);
                }
            })
            .global_remove(move |id| {
                let mut list = devices_remove.lock().unwrap();
                list.retain(|d| d.id != id);
            })
            .register()
    };

    // Virtual microphone: an Audio/Source node that applications capture from.
    let vmic_inner = VmicData::new(shared.clone());
    let player = vmic_inner.player.handle();
    let vmic_data = Rc::new(RefCell::new(vmic_inner));
    let vmic_stream = pw::stream::Stream::new(
        &core,
        "atcs-soundboard-virtual-mic",
        properties! {
            *pw::keys::MEDIA_TYPE => "Audio",
            *pw::keys::MEDIA_CATEGORY => "Capture",
            *pw::keys::MEDIA_ROLE => "Communication",
            *pw::keys::MEDIA_CLASS => "Audio/Source",
            *pw::keys::NODE_NAME => VIRTUAL_MIC_NODE_NAME,
            *pw::keys::NODE_DESCRIPTION => VIRTUAL_MIC_NAME,
            *pw::keys::NODE_VIRTUAL => "true",
            *pw::keys::NODE_PAUSE_ON_IDLE => "false",
            *pw::keys::APP_ID => APP_ID,
            *pw::keys::APP_NAME => "ATCS Soundboard",
        },
    )?;
    let vmic_listener = {
        let shared_for_vmic = shared.clone();
        let app_for_vmic = app.clone();
        vmic_stream
            .add_local_listener_with_user_data(vmic_data.clone())
            .state_changed(move |_, data: &mut Rc<RefCell<VmicData>>, _old, new| {
                let mut d = data.borrow_mut();
                match new {
                    pw::stream::StreamState::Streaming => {
                        d.connected = true;
                        shared_for_vmic.vmic_error.lock().unwrap().take();
                    }
                    pw::stream::StreamState::Error(e) => {
                        d.connected = false;
                        *shared_for_vmic.vmic_error.lock().unwrap() = Some(e);
                    }
                    _ => {}
                }
                let _ = app_for_vmic.emit("virtual-mic-status-changed", d.status());
            })
            .process(|stream, data: &mut Rc<RefCell<VmicData>>| {
                let mut d = data.borrow_mut();
                d.process(stream);
            })
            .register()?
    };
    let pod_bytes = format_pod_bytes();
    let pod = Pod::from_bytes(pod_bytes.as_slice()).ok_or(pw::Error::CreationFailed)?;
    let mut params = [pod];
    vmic_stream.connect(
        spa::utils::Direction::Output,
        None,
        connect_flags_rt(), // no autoconnect: the virtual source waits for consumers
        &mut params,
    )?;
    let vmic_node_id = vmic_stream.node_id();
    shared.vmic_node_id.store(vmic_node_id, Ordering::Relaxed);
    let vmic = VmicStream {
        _stream: vmic_stream,
        _listener: vmic_listener,
    };

    // Physical microphone.
    let mic_target = settings.microphone.as_deref().and_then(|s| s.parse::<u32>().ok());
    if let Some(target) = mic_target {
        shared.mic_node_id.store(target, Ordering::Relaxed);
    }
    let mut mic = match create_mic_stream(&core, &shared, &app, format_pod_bytes().as_slice(), mic_target) {
        Ok(m) => Some(m),
        Err(e) => {
            *shared.mic_error.lock().unwrap() = Some(e.to_string());
            None
        }
    };

    let monitor: Option<MonitorStream> = if settings.monitor_enabled {
        create_monitor_stream(&core, &shared).ok()
    } else {
        None
    };

    let state = Rc::new(RefCell::new(PwState {
        core,
        shared: shared.clone(),
        app: app.clone(),
        player,
        _vmic: vmic,
        mic: mic.take(),
        monitor,
    }));

    // Report finished clips to the UI (drained on the loop thread, not the RT thread).
    // try_borrow_mut: RT process may already hold the RefCell; skip this tick if so.
    let vmic_for_timer = vmic_data.clone();
    let app_for_timer = app.clone();
    let timer = mainloop.loop_().add_timer(move |_expirations| {
        let Ok(mut d) = vmic_for_timer.try_borrow_mut() else {
            return;
        };
        let finished = d.player.take_finished().to_vec();
        if !finished.is_empty() {
            for clip_id in finished {
                let _ = app_for_timer.emit(
                    "sound-stopped",
                    SoundStoppedPayload {
                        clip_id,
                        name: String::new(),
                        reason: Some("finished".into()),
                    },
                );
            }
            d.player.reset_finished();
        }
    });
    timer.update_timer(Some(Duration::from_millis(50)), Some(Duration::from_millis(50))).into_result()?;

    // Command channel: control plane -> loop thread.
    let _cmd_receiver = cmd_rx.attach(mainloop.loop_(), {
        let mainloop = mainloop.clone();
        let state = state.clone();
        move |cmd: EngineCommand| handle_command(&mainloop, &state, cmd)
    });

    let _ = app.emit("virtual-mic-status-changed", shared.vmic_status());
    let _ = app.emit("microphone-status-changed", shared.mic_status());
    let _ = app.emit("audio-device-changed", ());

    mainloop.run();
    Ok(())
}

/// Handle to the running engine, owned by the control plane.
pub struct Engine {
    cmd_tx: pw::channel::Sender<EngineCommand>,
    join: Option<JoinHandle<()>>,
    shared: Arc<EngineShared>,
    devices: Arc<Mutex<Vec<AudioDevice>>>,
}

impl Engine {
    pub fn start(app: AppHandle, repo: &Repository) -> atcs_soundboard_core::Result<Self> {
        let settings = settings_core::load_audio_settings(repo)?;
        let shared = EngineShared::new(&settings);
        let devices = Arc::new(Mutex::new(Vec::<AudioDevice>::new()));
        let (tx_back, rx_back) = std::sync::mpsc::channel::<pw::channel::Sender<EngineCommand>>();

        let app_thread = app.clone();
        let shared_thread = shared.clone();
        let devices_thread = devices.clone();
        let join = std::thread::Builder::new()
            .name("atcs-pw-engine".into())
            .spawn(move || {
                let (cmd_tx, cmd_rx) = pw::channel::channel::<EngineCommand>();
                let _ = tx_back.send(cmd_tx);
                if let Err(e) = run_engine(app_thread.clone(), shared_thread, devices_thread, settings, cmd_rx) {
                    let _ = app_thread.emit(
                        "audio-error",
                        atcs_soundboard_core::models::AudioErrorPayload {
                            message: format!("audio engine error: {e}"),
                        },
                    );
                }
            })?;

        let cmd_tx = rx_back
            .recv()
            .map_err(|_| atcs_soundboard_core::Error::NotReady("engine thread failed to start".into()))?;

        Ok(Self {
            cmd_tx,
            join: Some(join),
            shared,
            devices,
        })
    }

    pub fn send(&self, cmd: EngineCommand) {
        let _ = self.cmd_tx.send(cmd);
    }

    pub fn stop(&mut self) {
        self.send(EngineCommand::Stop);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }

    pub fn devices(&self) -> Vec<AudioDevice> {
        self.devices.lock().unwrap().clone()
    }

    pub fn input_devices(&self) -> Vec<AudioDevice> {
        self.devices
            .lock()
            .unwrap()
            .iter()
            .filter(|d| d.media_class == "Audio/Source")
            .cloned()
            .collect()
    }

    pub fn status(&self) -> AudioStatus {
        AudioStatus {
            engine_running: true,
            microphone: self.shared.mic_status(),
            virtual_microphone: self.shared.vmic_status(),
            sample_rate: TARGET_SAMPLE_RATE,
            channels: TARGET_CHANNELS,
            master_volume: f64::from(self.shared.master_volume()),
            mic_volume: f64::from(self.shared.mic_volume()),
            soundboard_volume: f64::from(self.shared.soundboard_volume()),
            monitor_enabled: self.shared.monitor_enabled(),
        }
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        if self.join.is_some() {
            self.stop();
        }
    }
}
