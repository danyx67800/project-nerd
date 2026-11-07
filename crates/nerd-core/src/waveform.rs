//! Waveform peak extraction, pure and allocation-light.

/// Number of buckets used by the UI waveforms.
pub const DEFAULT_BUCKETS: usize = 900;

/// Reduces interleaved samples to `buckets` peak values in `0.0..=1.0`.
///
/// The function tolerates empty input and non-finite samples, which makes it
/// safe to call on partially decoded files.
pub fn peaks(samples: &[f32], channels: usize, buckets: usize) -> Vec<f32> {
    if samples.is_empty() || buckets == 0 {
        return vec![0.0; buckets];
    }
    let channels = channels.max(1);
    let frames = samples.len() / channels;
    if frames == 0 {
        return vec![0.0; buckets];
    }
    let mut out = vec![0.0f32; buckets];
    for (bucket, slot) in out.iter_mut().enumerate() {
        let start = bucket * frames / buckets;
        let end = ((bucket + 1) * frames / buckets).max(start + 1).min(frames);
        let mut peak = 0.0f32;
        for frame in start..end {
            for ch in 0..channels {
                let value = samples[frame * channels + ch].abs();
                if value.is_finite() && value > peak {
                    peak = value;
                }
            }
        }
        *slot = peak.min(1.0);
    }
    out
}

/// Computes a short RMS value used by the library inspector.
pub fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f64 = samples
        .iter()
        .map(|s| {
            let v = if s.is_finite() { *s as f64 } else { 0.0 };
            v * v
        })
        .sum();
    ((sum / samples.len() as f64).sqrt()) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peaks_are_normalized_and_bucketed() {
        let samples = [0.0, 1.0, -0.5, 0.25, 0.0, -0.75];
        let peaks = peaks(&samples, 2, 3);
        assert_eq!(peaks.len(), 3);
        assert!((peaks[0] - 1.0).abs() < f32::EPSILON);
        assert!((peaks[1] - 0.5).abs() < f32::EPSILON);
        assert!((peaks[2] - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn empty_input_returns_zeros() {
        assert_eq!(peaks(&[], 2, 4), vec![0.0; 4]);
        assert_eq!(rms(&[]), 0.0);
    }

    #[test]
    fn non_finite_samples_do_not_poison_peaks() {
        let samples = [f32::NAN, 0.5, f32::INFINITY, -0.2];
        let peaks = peaks(&samples, 2, 1);
        assert_eq!(peaks, vec![0.5]);
    }
}
