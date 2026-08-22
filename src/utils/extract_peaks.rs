use crate::utils::audio_to_spectrogram::Spectrogram;

/// Log-spaced band edges as fractions of the spectrum. Each frame contributes
/// at most one peak per band: its strongest bin above the threshold. This is
/// the classic Shazam-style filter — it caps peaks at ~bands-per-frame and
/// keeps them concentrated where real musical energy lives instead of noise.
const BAND_EDGE_FRACTIONS: [f32; 7] = [0.0, 0.02, 0.04, 0.08, 0.16, 0.31, 1.0];

pub fn extract_peaks(spectrogram: &Spectrogram, threshold_db: f32) -> Vec<(usize, usize, f32)> {
    let mut peaks = Vec::new();
    if spectrogram.frames == 0 || spectrogram.bins == 0 {
        return peaks;
    }

    // convert DB threshold to linear
    // fallback for invalid threshold values
    let threshold_db = if threshold_db.is_nan() { -20.0 } else { threshold_db };
    let threshold_linear = 10.0f32.powf(threshold_db / 20.0);

    let bands: Vec<usize> = BAND_EDGE_FRACTIONS
        .iter()
        .map(|&f| ((spectrogram.bins as f32 * f) as usize).min(spectrogram.bins))
        .collect();

    for frame_idx in 0..spectrogram.frames {
        let frame = spectrogram.row(frame_idx);

        for edge in bands.windows(2) {
            let (band_start, band_end) = (edge[0], edge[1].max(edge[0] + 1));
            if band_start >= spectrogram.bins {
                continue;
            }

            // Strongest bin within this band for this frame
            let mut best_bin = band_start;
            let mut best_mag = frame[band_start];
            for (bin_idx, &mag) in frame.iter().enumerate().skip(band_start).take(band_end - band_start) {
                if mag > best_mag {
                    best_bin = bin_idx;
                    best_mag = mag;
                }
            }

            if best_mag >= threshold_linear {
                peaks.push((frame_idx, best_bin, best_mag));
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
        let mut frame = vec![0.0; 100];
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
    fn one_peak_per_band() {
        // Two strong bins in different bands survive; a weak third does not.
        let mut frame = vec![0.0; 100];
        frame[5] = 1.0;   // band [0..2]
        frame[50] = 0.9;  // band [16..100]
        frame[60] = 0.3;  // same band as 50 but weaker

        let spectrogram = spectrogram(&[frame]);
        let peaks = extract_peaks(&spectrogram, -20.0);

        assert_eq!(peaks.len(), 2);
        let bins: Vec<usize> = peaks.iter().map(|&(_, b, _)| b).collect();
        assert!(bins.contains(&5));
        assert!(bins.contains(&50));
    }

    #[test]
    fn handle_non_finite_threshold() {
        let mut frame = vec![0.0; 100];
        frame[5] = 1.0;
        let spectrogram = spectrogram(&[frame]);

        let peaks_inf = extract_peaks(&spectrogram, f32::INFINITY);
        let peaks_nan = extract_peaks(&spectrogram, f32::NAN);

        assert!(peaks_inf.is_empty());
        assert!(!peaks_nan.is_empty());
    }
}
