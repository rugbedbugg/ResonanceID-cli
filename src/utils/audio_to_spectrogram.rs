use rustfft::{FftPlanner, num_complex::Complex};

type Sample = i16;

pub fn audio_to_spectrogram(
        samples: &[Sample],
        sample_rate: u32,
        frame_size: usize,      // If       1024
        hop_size: usize,        // then     512 = 50% overlap
    ) -> Vec<Vec<f32>> {
    // Guard invalid pipeline configuration
    if samples.is_empty() || sample_rate == 0 || frame_size < 2 || hop_size == 0 {
        return Vec::new();
    }

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(frame_size);

    let mut spectrogram = Vec::new();
    let mut frame_f32: Vec<Complex<f32>> = vec![Complex::new(0.0, 0.0); frame_size];

    for start in (0..samples.len()).step_by(hop_size) {
        let end = (start + frame_size).min(samples.len());
        let frame_len = end - start;

        // Zero-pad if frame is shorter than frame_size
        for i in 0..frame_size {
            let sample = if i < frame_len {
                samples[start + i] as f32 / sample_rate as f32
            } else { 0.0 };
            frame_f32[i] = Complex::new(sample, 0.0);
        }

        // Hamming window
        for i in 0..frame_size {
            let w = 0.54 - 0.46 
                * (2.0 * std::f32::consts::PI*i as f32 / (frame_size-1) as f32).cos();
            frame_f32[i] *= w;
        }

        // Fast Fourier Transform
        fft.process(&mut frame_f32);

        // Conver to magnitude spectrum
        let mut spectrum = Vec::with_capacity(frame_size/2);
        for i in 0..(frame_size/2) {
            let mag = frame_f32[i].norm();
            spectrum.push(mag);
        }

        spectrogram.push(spectrum);
    }

    spectrogram
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_zero_samples() {
        let samples = vec![0i16; 2048];
        let spectrogram = audio_to_spectrogram(&samples, 44100, 1024, 512);
        assert!(!spectrogram.is_empty());
        assert!(spectrogram.len() > 1);
    }

    #[test]
    fn handle_short_samples() {
        let samples = vec![0i16, 512];
        let spectrogram = audio_to_spectrogram(&samples, 44100, 1024, 512);
        assert!(!spectrogram.is_empty());
    }

    #[test]
    fn handle_empty_samples() {
        let samples = vec![];
        let spectrogram = audio_to_spectrogram(&samples, 44100, 1024, 512);
        assert!(spectrogram.is_empty());
    }

    #[test]
    fn handle_invalid_sample_rate() {
        let samples = vec![1i16, 2, 3, 4];
        let spectrogram = audio_to_spectrogram(&samples, 0, 1024, 512);
        assert!(spectrogram.is_empty());
    }

    #[test]
    fn handle_invalid_fft_params() {
        let samples = vec![1i16, 2, 3, 4];

        let spectrogram_frame = audio_to_spectrogram(&samples, 44100, 1, 512);
        assert!(spectrogram_frame.is_empty());

        let spectrogram_hop = audio_to_spectrogram(&samples, 44100, 1024, 0);
        assert!(spectrogram_hop.is_empty());
    }
}
