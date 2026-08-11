//! Pure mixing math. All functions are allocation-free and safe to call from
//! a real-time audio callback.

/// Interleaved-stereo frame count for a sample slice.
pub fn frames_of(samples: &[f32]) -> usize {
    samples.len() / 2
}

/// Add `src` (interleaved stereo) into `dst`, scaled by `gain`, clipping to
/// avoid unbounded accumulation. `src` is truncated if `dst` is shorter.
pub fn mix_in(dst: &mut [f32], src: &[f32], gain: f32) {
    let n = dst.len().min(src.len());
    for (d, s) in dst[..n].iter_mut().zip(&src[..n]) {
        let mixed = *d + s * gain;
        *d = mixed.clamp(-8.0, 8.0);
    }
}

/// Copy `src` (interleaved stereo) into `dst` scaled by `gain`, replacing
/// contents (used to copy the mic or a single source into the mix bus).
pub fn write_in(dst: &mut [f32], src: &[f32], gain: f32) {
    let n = dst.len().min(src.len());
    for (d, s) in dst[..n].iter_mut().zip(&src[..n]) {
        *d = (s * gain).clamp(-8.0, 8.0);
    }
}

/// Silence a slice (set to zero).
pub fn clear(samples: &mut [f32]) {
    samples.fill(0.0);
}

/// Apply a single constant gain to every sample, clamping to `[-8, 8]`.
pub fn apply_gain(samples: &mut [f32], gain: f32) {
    for s in samples.iter_mut() {
        *s = (*s * gain).clamp(-8.0, 8.0);
    }
}

/// Soft-clip a buffer to `[-1, 1]` with a gentle tanh-like curve to avoid
/// hard digital clipping on the output.
pub fn soft_clip(samples: &mut [f32]) {
    for s in samples.iter_mut() {
        *s = (*s / (1.0 + s.abs())).clamp(-1.0, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mix_scales_and_adds() {
        let mut dst = [0.0, 0.0, 1.0, 1.0];
        mix_in(&mut dst, &[0.5, -0.5, 1.0, -1.0], 2.0);
        assert_eq!(dst, [1.0, -1.0, 3.0, -1.0]);
    }

    #[test]
    fn mix_truncates_to_dst_len() {
        let mut dst = [0.0; 4];
        mix_in(&mut dst, &[1.0; 8], 1.0);
        assert_eq!(dst, [1.0; 4]);
    }

    #[test]
    fn mix_clips_to_avoid_runaway() {
        let mut dst = [10.0, 10.0];
        mix_in(&mut dst, &[10.0, 10.0], 2.0);
        assert!(dst.iter().all(|v| *v <= 8.0));
    }

    #[test]
    fn soft_clip_bounds() {
        let mut s = [100.0, -100.0, 0.0];
        soft_clip(&mut s);
        assert!(s[0] <= 1.0);
        assert!(s[1] >= -1.0);
        assert_eq!(s[2], 0.0);
    }
}
