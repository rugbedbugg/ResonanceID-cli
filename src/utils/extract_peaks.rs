pub fn extract_peaks(spectrogram: &[Vec<f32>], threshold_db: f32) -> Vec<(usize, usize, f32)> {
    let mut peaks = Vec::new();

    // convert DB threshold to linear
    // fallback for invalid threshold values
    let threshold_db = if threshold_db.is_nan() { -20.0 } else { threshold_db };
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_peak_in_flat_spectrogram() {
        let spectrogram = vec![vec![0.0; 10]; 5];
        let peaks = extract_peaks(&spectrogram, -20.0);
        assert!(peaks.is_empty());
    }

    #[test]
    fn find_peak_at_bin_5() {
        let mut frame = vec![0.0; 10];
        frame[5] = 1.0;

        let spectrogram = vec![frame; 3];
        let peaks = extract_peaks(&spectrogram, -20.0);

        assert!(!peaks.is_empty());
        for &(_frame_idx, bin_idx, mag) in &peaks {
            assert_eq!(bin_idx, 5);
            assert!(mag > 0.0);
        }
    }

    #[test]
    fn handle_non_finite_threshold() {
        let mut frame = vec![0.0; 10];
        frame[5] = 1.0;
        let spectrogram = vec![frame; 1];

        let peaks_inf = extract_peaks(&spectrogram, f32::INFINITY);
        let peaks_nan = extract_peaks(&spectrogram, f32::NAN);

        assert!(peaks_inf.is_empty());
        assert!(!peaks_nan.is_empty());
    }
}
