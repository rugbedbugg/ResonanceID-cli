pub fn extract_peaks(spectrogram: &[Vec<f32>], threshold_db: f32) -> Vec<(usize, usize, f32)> {
    let mut peaks = Vec::new();

    // convert DB threshold to linear
    let threshold_linear = 10.0f32.powf(threshold_db / 20.0);

    for (frame_idx, frame) in spectrogram.iter().enumerate() {
        for (bin_idx, &mag) in frame.iter().enumerate() {
            if mag < threshold_linear {
                continue;
            }

            // Check if this is a local max in frequency bin
            let left_ok = bin_idx==0 || frame[bin_idx-1] < mag;
            let right_ok = bin_idx==frame.len()-1 || frame[bin_idx+1] < mag;

            if left_ok && right_ok {
                peaks.push((frame_idx, bin_idx, mag));
            }
        }
    }

    peaks
}
