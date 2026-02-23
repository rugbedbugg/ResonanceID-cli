pub fn peaks_to_hashes(peaks: &[(usize, usize, f32)], anchor_window:usize) -> Vec<u32> {
    let mut hashes = Vec::new();

    if anchor_window == 0 {
        return hashes;
    }

    for (anchor_idx, &(anchor_frame, anchor_bin, _)) in peaks.iter().enumerate() {
        let target_start = anchor_idx + 1;
        let target_end = (anchor_idx + anchor_window).min(peaks.len());

        for &(target_frame, target_bin, _) in &peaks[target_start..target_end] {
            let delta_t = target_frame as i32 - anchor_frame as i32;
            if delta_t <= 0 || delta_t > 1023 {
                continue;
            }

            // Pack into a 32-bit hash
            // You can tweak the bit layout later
            let hash = ((anchor_bin as u32) << 20)
                    | ((target_bin as u32) << 10)
                    | ((delta_t as u32) & 0x3ff);

            hashes.push(hash);
        }
    }

    hashes
}

pub fn peaks_to_fingerprints(
    peaks: &[(usize, usize, f32)],
    anchor_window: usize,
    sample_rate: u32,
    hop_size: usize,
) -> Vec<(u32, u32)> {
    let mut fingerprints = Vec::new();

    if anchor_window == 0 || sample_rate == 0 || hop_size == 0 {
        return fingerprints;
    }

    for (anchor_idx, &(anchor_frame, anchor_bin, _)) in peaks.iter().enumerate() {
        let target_start = anchor_idx + 1;
        let target_end = (anchor_idx + anchor_window).min(peaks.len());
        let anchor_time_ms =
            (anchor_frame as f32 * hop_size as f32 / sample_rate as f32 * 1000.0) as u32;

        for &(target_frame, target_bin, _) in &peaks[target_start..target_end] {
            let delta_t = target_frame as i32 - anchor_frame as i32;
            if delta_t <= 0 || delta_t > 1023 {
                continue;
            }

            let hash = ((anchor_bin as u32) << 20)
                    | ((target_bin as u32) << 10)
                    | ((delta_t as u32) & 0x3ff);

            fingerprints.push((hash, anchor_time_ms));
        }
    }

    fingerprints
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn generate_hashes_from_peaks() {
        let peaks = vec![
            (0, 100, 1.0),
            (1, 150, 1.0),
            (2, 200, 1.0),
        ];

        let hashes = peaks_to_hashes(&peaks, 50);
        assert!(!hashes.is_empty());
        assert!(hashes.len() >= 2);
    }

    #[test]
    fn handle_empty_peaks() {
        let peaks = vec![];
        let hashes = peaks_to_hashes(&peaks, 50);
        assert!(hashes.is_empty());
    }

    #[test]
    fn generate_fingerprints_with_anchor_time() {
        let peaks = vec![
            (10, 100, 1.0),
            (12, 150, 1.0),
            (15, 200, 1.0),
        ];

        let fingerprints = peaks_to_fingerprints(&peaks, 50, 1000, 100);
        assert!(!fingerprints.is_empty());

        // anchor frame=10, hop=100, rate=1000 => 1000 ms
        let (_hash, anchor_time_ms) = fingerprints[0];
        assert_eq!(anchor_time_ms, 1000);
    }

    #[test]
    fn ignore_invalid_delta_t_and_zero_window() {
        let peaks = vec![
            (0, 100, 1.0),
            (2000, 150, 1.0), // delta_t too large
            (2001, 200, 1.0),
        ];

        let hashes = peaks_to_hashes(&peaks, 50);
        assert!(!hashes.is_empty());

        let empty_hashes = peaks_to_hashes(&peaks, 0);
        assert!(empty_hashes.is_empty());
    }

    #[test]
    fn handle_invalid_fingerprint_params() {
        let peaks = vec![(1, 100, 1.0), (2, 150, 1.0)];

        let zero_rate = peaks_to_fingerprints(&peaks, 50, 0, 100);
        assert!(zero_rate.is_empty());

        let zero_hop = peaks_to_fingerprints(&peaks, 50, 1000, 0);
        assert!(zero_hop.is_empty());

        let zero_window = peaks_to_fingerprints(&peaks, 0, 1000, 100);
        assert!(zero_window.is_empty());
    }
}
