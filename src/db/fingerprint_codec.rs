/// Packing helpers for the compact fingerprints schema.
///
/// Legacy layout stored one SQLite row per fingerprint
/// (hash, song_id, anchor_time_ms), wasting roughly half of each record on
/// padding and row headers. The packed layout stores one row per
/// (hash, song_id) pair whose BLOB holds the anchor times as a flat array of
/// little-endian u32 milliseconds.

pub fn pack_anchor_times(times: &[u32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(times.len() * 4);
    for t in times {
        out.extend_from_slice(&t.to_le_bytes());
    }
    out
}

pub fn unpack_anchor_times(blob: &[u8]) -> Vec<u32> {
    blob.chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_preserves_values() {
        let times = [0u32, 23, 116_000, u32::MAX];
        let blob = pack_anchor_times(&times);
        assert_eq!(blob.len(), 16);
        assert_eq!(unpack_anchor_times(&blob), times.to_vec());
    }

    #[test]
    fn empty_round_trip() {
        assert!(pack_anchor_times(&[]).is_empty());
        assert!(unpack_anchor_times(&[]).is_empty());
    }

    #[test]
    fn unpack_ignores_trailing_partial_chunk() {
        let times = [42u32, 1337];
        let mut blob = pack_anchor_times(&times);
        blob.push(0xAB); // corrupt tail byte
        assert_eq!(unpack_anchor_times(&blob), times.to_vec());
    }
}
