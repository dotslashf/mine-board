use std::path::Path;
use std::sync::Arc;

use hound::{SampleFormat, WavReader};

use crate::errors::{Error, Result};
use crate::{TARGET_CHANNELS, TARGET_SAMPLE_RATE};

/// Decoded audio stored as interleaved stereo `f32` samples at the target
/// sample rate, ready to be mixed on the real-time thread.
#[derive(Debug, Clone)]
pub struct DecodedAudio {
    pub samples: Arc<Vec<f32>>,
    pub sample_rate: u32,
    pub channels: u16,
    pub frames: usize,
    pub duration_ms: i64,
}

/// Read a WAV file, normalise to interleaved stereo at [`TARGET_SAMPLE_RATE`].
pub fn decode_wav_file(path: impl AsRef<Path>) -> Result<DecodedAudio> {
    let path = path.as_ref();
    let reader = WavReader::open(path).map_err(|e| Error::AudioDecode(e.to_string()))?;
    let spec = reader.spec();
    let src_rate = spec.sample_rate;
    let src_channels = spec.channels;
    let src_frames = reader.duration();

    if src_rate == 0 || src_channels == 0 {
        return Err(Error::AudioDecode("invalid WAV spec".into()));
    }

    let samples = match (spec.sample_format, spec.bits_per_sample) {
        (SampleFormat::Float, 32) => reader
            .into_samples::<f32>()
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| Error::AudioDecode(e.to_string()))?,
        (SampleFormat::Int, 16) => reader
            .into_samples::<i16>()
            .map(|s| s.map(|v| f32::from(v) / 32768.0))
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| Error::AudioDecode(e.to_string()))?,
        (SampleFormat::Int, 24) => reader
            .into_samples::<i32>()
            .map(|s| s.map(|v| v as f32 / 8_388_608.0))
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| Error::AudioDecode(e.to_string()))?,
        (SampleFormat::Int, 32) => reader
            .into_samples::<i32>()
            .map(|s| s.map(|v| v as f32 / 2_147_483_648.0))
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| Error::AudioDecode(e.to_string()))?,
        (fmt, bits) => {
            return Err(Error::AudioDecode(format!(
                "unsupported WAV format {fmt:?} {bits}-bit"
            )))
        }
    };

    Ok(resample_to_target(samples, src_rate, src_channels, src_frames as u64))
}

/// Convert arbitrary-rate interleaved audio to interleaved stereo at the
/// target rate using linear interpolation.
fn resample_to_target(
    samples: Vec<f32>,
    src_rate: u32,
    src_channels: u16,
    src_frames: u64,
) -> DecodedAudio {
    let src_channels = src_channels as usize;
    let src_frames = src_frames as usize;
    let src = &samples[..src_frames * src_channels];

    let out_frames = ((src_frames as f64) * f64::from(TARGET_SAMPLE_RATE) / f64::from(src_rate))
        .floor() as usize;
    let mut out = Vec::with_capacity(out_frames * TARGET_CHANNELS as usize);
    let step = f64::from(src_rate) / f64::from(TARGET_SAMPLE_RATE);

    for frame in 0..out_frames {
        let pos = frame as f64 * step;
        let i0 = pos.floor() as usize;
        let frac = (pos - i0 as f64) as f32;
        let i1 = (i0 + 1).min(src_frames - 1);

        for channel in 0..TARGET_CHANNELS as usize {
            let src_ch = channel.min(src_channels - 1);
            let a = src[i0 * src_channels + src_ch];
            let b = src[i1 * src_channels + src_ch];
            out.push(a + (b - a) * frac);
        }
    }

    let duration_ms = (out_frames as i64)
        .saturating_mul(1000)
        .checked_div(i64::from(TARGET_SAMPLE_RATE))
        .unwrap_or(0);

    DecodedAudio {
        samples: Arc::new(out),
        sample_rate: TARGET_SAMPLE_RATE,
        channels: TARGET_CHANNELS,
        frames: out_frames,
        duration_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_wav(path: &Path, rate: u32, channels: u16, samples: Vec<f32>) {
        let spec = hound::WavSpec {
            channels,
            sample_rate: rate,
            bits_per_sample: 32,
            sample_format: SampleFormat::Float,
        };
        let mut writer = hound::WavWriter::create(path, spec).unwrap();
        for s in samples {
            writer.write_sample(s).unwrap();
        }
        writer.finalize().unwrap();
    }

    #[test]
    fn decodes_stereo_48k() {
        let dir = std::env::temp_dir();
        let path = dir.join("atcs_test_48k.wav");
        let mut samples = Vec::new();
        for i in 0..4800 {
            let v = (i as f32 / 100.0).sin();
            samples.push(v);
            samples.push(-v);
        }
        write_wav(&path, 48000, 2, samples);

        let decoded = decode_wav_file(&path).unwrap();
        assert_eq!(decoded.channels, 2);
        assert_eq!(decoded.sample_rate, 48000);
        assert_eq!(decoded.frames, 4800);
        assert_eq!(decoded.samples.len(), 4800 * 2);
        assert!(decoded.duration_ms >= 99 && decoded.duration_ms <= 100);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn decodes_mono_and_resamples() {
        let dir = std::env::temp_dir();
        let path = dir.join("atcs_test_44k_mono.wav");
        let mut samples = Vec::new();
        for _ in 0..22050 {
            samples.push(0.5);
        }
        write_wav(&path, 44100, 1, samples);

        let decoded = decode_wav_file(&path).unwrap();
        assert_eq!(decoded.channels, 2);
        assert_eq!(decoded.sample_rate, 48000);
        // 0.5s at 44.1k -> 0.5s at 48k
        assert_eq!(decoded.frames, 24000);
        assert_eq!(decoded.samples.len(), 48000);
        assert_eq!(decoded.samples[0], 0.5);
        assert_eq!(decoded.samples[1], 0.5);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn rejects_non_wav() {
        let dir = std::env::temp_dir();
        let path = dir.join("atcs_test_not_wav.txt");
        std::fs::write(&path, b"not a wav").unwrap();
        assert!(decode_wav_file(&path).is_err());
        let _ = std::fs::remove_file(&path);
    }
}
