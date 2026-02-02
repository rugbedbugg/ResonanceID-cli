use rustfft::{FftPlanner, num_complex::Complex};

type Sample = i16;

pub fn audio_to_spectrogram(
        samples: &[Sample],
        sample_rate: u32,
        frame_size: usize,      // If       1024
        hop_size: usize,        // then     512 = 50% overlap
    ) -> Vec<Vec<f32>> {
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
