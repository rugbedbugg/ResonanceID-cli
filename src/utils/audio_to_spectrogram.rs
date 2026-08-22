use rustfft::{FftPlanner, num_complex::Complex};

type Sample = i16;

/// Time-frequency representation stored as a flat, frame-major buffer.
///
/// Sample at (frame f, bin b) lives at `data[f * bins + b]`.
pub struct Spectrogram {
    pub data: Vec<f32>,
    pub frames: usize,
    pub bins: usize,
}

impl Spectrogram {
    pub fn empty() -> Self {
        Self { data: Vec::new(), frames: 0, bins: 0 }
    }

    pub fn is_empty(&self) -> bool {
        self.frames == 0
    }

    #[inline]
    pub fn row(&self, frame: usize) -> &[f32] {
        &self.data[frame * self.bins..(frame + 1) * self.bins]
    }
}

pub fn audio_to_spectrogram(
        samples: &[Sample],
        sample_rate: u32,
        frame_size: usize,      // If       1024
        hop_size: usize,        // then     512 = 50% overlap
    ) -> Spectrogram {
    // Guard invalid pipeline configuration
    if samples.is_empty() || sample_rate == 0 || frame_size < 2 || hop_size == 0 {
        return Spectrogram::empty();
    }

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(frame_size);

    let bins = frame_size / 2;
    let frame_count = samples.len().div_ceil(hop_size);
    let mut spectrogram_data = vec![0.0f32; frame_count * bins];

    // Hamming window computed once instead of per-frame
    let window: Vec<f32> = (0..frame_size)
        .map(|i| {
            0.54 - 0.46
                * (2.0 * std::f32::consts::PI * i as f32 / (frame_size - 1) as f32).cos()
        })
        .collect();

    let mut frame_f32: Vec<Complex<f32>> = vec![Complex::new(0.0, 0.0); frame_size];

    for (frame_idx, start) in (0..samples.len()).step_by(hop_size).enumerate() {
        let end = (start + frame_size).min(samples.len());
        let frame_len = end - start;

        // Zero-pad if frame is shorter than frame_size
        for i in 0..frame_len {
            frame_f32[i] = Complex::new(samples[start + i] as f32 / i16::MAX as f32 * window[i], 0.0);
        }
        for i in frame_len..frame_size {
            frame_f32[i] = Complex::new(0.0, 0.0);
        }

        // Fast Fourier Transform
        fft.process(&mut frame_f32);

        // Convert to magnitude spectrum
        let out_start = frame_idx * bins;
        for i in 0..bins {
            spectrogram_data[out_start + i] = frame_f32[i].norm();
        }
    }

    Spectrogram {
        data: spectrogram_data,
        frames: frame_count,
        bins,
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_zero_samples() {
        let samples = vec![0i16; 2048];
        let spectrogram = audio_to_spectrogram(&samples, 44100, 1024, 512);
        assert!(spectrogram.frames > 1);
    }

    #[test]
    fn handle_short_samples() {
        let samples = vec![0i16, 512];
        let spectrogram = audio_to_spectrogram(&samples, 44100, 1024, 512);
        assert!(!spectrogram.data.is_empty());
    }

    #[test]
    fn handle_empty_samples() {
        let samples = vec![];
        let spectrogram = audio_to_spectrogram(&samples, 44100, 1024, 512);
        assert_eq!(spectrogram.frames, 0);
    }

    #[test]
    fn handle_invalid_sample_rate() {
        let samples = vec![1i16, 2, 3, 4];
        let spectrogram = audio_to_spectrogram(&samples, 0, 1024, 512);
        assert_eq!(spectrogram.frames, 0);
    }

    #[test]
    fn handle_invalid_fft_params() {
        let samples = vec![1i16, 2, 3, 4];

        let spectrogram_frame = audio_to_spectrogram(&samples, 44100, 1, 512);
        assert_eq!(spectrogram_frame.frames, 0);

        let spectrogram_hop = audio_to_spectrogram(&samples, 44100, 1024, 0);
        assert_eq!(spectrogram_hop.frames, 0);
    }
}
