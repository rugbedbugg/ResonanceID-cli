use crate::{config::RecognitionConfig, db::create_db::Database};
use rusqlite::{Result, params};

fn dynamic_min_match_score(query_hash_count: usize, cfg: &RecognitionConfig) -> u32 {
    if query_hash_count < cfg.small_query_threshold {
        return cfg.min_match_score;
    }

    // scale score gate for large queries to suppress accidental collisions
    ((query_hash_count as f32).sqrt() * cfg.dynamic_gate_scale) as u32
}

impl Database {
    //////////////////////
    // Recognize a Song //
    //////////////////////
    // Identifies the best-matching song by offset consistency.
    // i.   Collect offset votes for matching hashes
    // ii.  Compute best offset score per song
    // iii. Rank and fetch metadata
    pub fn recognize_song(&self, hashes: &[(u32, u32)]) -> Result<Vec<(String, String, f32)>> {
        self.recognize_song_with_config(hashes, &RecognitionConfig::default())
    }

    pub fn recognize_song_with_config(
        &self,
        hashes: &[(u32, u32)],
        cfg: &RecognitionConfig,
    ) -> Result<Vec<(String, String, f32)>> {
        //---------------------------------------//
        //-- i. Candidate collection by offset --//
        //---------------------------------------//
        let mut offset_counts: std::collections::HashMap<(i64, i32), u32> =
            std::collections::HashMap::new();
        let min_score_gate = dynamic_min_match_score(hashes.len(), cfg);

        // Prepare fingerprint lookup statement
        let mut stmt = self.conn.prepare(
            "SELECT song_id, anchor_time_ms \
            FROM fingerprints \
            WHERE hash=?",
        )?;

        for &(hash, query_anchor_time_ms) in hashes {
            let rows = stmt.query_map(params![hash as i64], |row: &rusqlite::Row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
            })?;

            for row in rows {
                let (song_id, db_time_ms) = row?;
                let offset = db_time_ms as i32 - query_anchor_time_ms as i32;
                *offset_counts.entry((song_id, offset)).or_insert(0) += 1;
            }
        }
        //--------------------------------------------------------//
        //-- ii. Compute best offset score per song             --//
        //--------------------------------------------------------//
        let mut scores: std::collections::HashMap<i64, u32> = std::collections::HashMap::new();
        for ((song_id, _offset), count) in offset_counts {
            let entry = scores.entry(song_id).or_insert(0);
            if count > *entry {
                *entry = count;
            }
        }
        //------------------------------------------//
        //-- iii. Sort by score and fetch metadata --//
        //------------------------------------------//
        let mut ranked: Vec<_> = scores.into_iter().collect();
        ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

        let mut results = Vec::new();
        for (song_id, score) in ranked.into_iter().take(cfg.max_results) {
            if score < min_score_gate {
                continue;
            }

            let (title, artist) = self
                .conn
                .query_row(
                    "SELECT title, artist FROM songs WHERE id=?",
                    params![song_id],
                    |row: &rusqlite::Row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                )
                .unwrap_or_else(|_| ("Unknown".to_string(), "Unknown".to_string()));

            results.push((title, artist, score as f32));
        }

        //--------------------//
        //-- RETURN RESULTS --//
        //--------------------//
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dynamic_gate_for_small_queries() {
        let cfg = RecognitionConfig::default();
        assert_eq!(dynamic_min_match_score(10, &cfg), 2);
        assert_eq!(dynamic_min_match_score(999, &cfg), 2);
    }

    #[test]
    fn dynamic_gate_scales_for_large_queries() {
        let cfg = RecognitionConfig::default();
        let gate = dynamic_min_match_score(1_000_000, &cfg);
        assert!(gate > 2);
        assert_eq!(gate, 30000);
    }
}
