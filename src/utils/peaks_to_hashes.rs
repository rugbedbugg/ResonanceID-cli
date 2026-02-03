pub fn peaks_to_hashes(peaks: &[(usize, usize, f32)], anchor_window:usize) -> Vec<u32> {
    let mut hashes = Vec::new();

    for (anchor_idx, &(anchor_frame, anchor_bin, _)) in peaks.iter().enumerate() {
        let target_start = anchor_idx + 1;
        let target_end = (anchor_idx + anchor_window).min(peaks.len());

        for &(target_frame, target_bin, _) in &peaks[target_start..target_end] {
            let delta_t = target_frame as i32 - anchor_frame as i32;

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
        assert!(!hashes.len() >= 3);
    }

    #[test]
    fn handle_empty_peaks() {
        let peaks = vec![];
        let hashes = peaks_to_hashes(&peaks, 50);
        assert!(hashes.is_empty());
    }
}
