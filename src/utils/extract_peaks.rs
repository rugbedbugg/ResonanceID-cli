use crate::utils::audio_to_spectrogram::Spectrogram;

pub fn extract_peaks(spectrogram: &Spectrogram, threshold_db: f32) -> Vec<(usize, usize, f32)> {
    let mut peaks = Vec::new();
    if spectrogram.frames == 0 || spectrogram.bins == 0 {
        return peaks;
    }

    // convert DB threshold to linear
    // fallback for invalid threshold values
    let threshold_db = if threshold_db.is_nan() { -20.0 } else { threshold_db };
    let threshold_linear = 10.0f32.powf(threshold_db / 20.0);

    for frame_idx in 0..spectrogram.frames {
        let frame = spectrogram.row(frame_idx);
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

    fn spectrogram(frames: &[Vec<f32>]) -> Spectrogram {
        let bins = frames.first().map_or(0, |f| f.len());
        Spectrogram {
            data: frames.iter().flatten().copied().collect(),
            frames: frames.len(),
            bins,
        }
    }

    #[test]
    fn find_peak_in_flat_spectrogram() {
        let spectrogram = spectrogram(&core::iter::repeat_n(vec![0.0; 10], 5).collect::<Vec<_>>());
        let peaks = extract_peaks(&spectrogram, -20.0);
        assert!(peaks.is_empty());
    }

    #[test]
    fn find_peak_at_bin_5() {
        let mut frame = vec![0.0; 10];
        frame[5] = 1.0;

        let spectrogram = spectrogram(&[frame.clone(), frame.clone(), frame]);
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
        let spectrogram = spectrogram(&[frame]);

        let peaks_inf = extract_peaks(&spectrogram, f32::INFINITY);
        let peaks_nan = extract_peaks(&spectrogram, f32::NAN);

        assert!(peaks_inf.is_empty());
        assert!(!peaks_nan.is_empty());
    }
}
