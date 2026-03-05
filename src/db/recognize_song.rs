use crate::{config::RecognitionConfig, db::create_db::Database};
use rusqlite::{Result, params};

fn dynamic_min_match_score(query_hash_count: usize, cfg: &RecognitionConfig) -> u32 {
    if query_hash_count < cfg.small_query_threshold {
        return cfg.min_match_score;
    }

    // Cap at 500 to prevent gate from becoming huge on large queries
    let score = ((query_hash_count as f32).sqrt() * cfg.dynamic_gate_scale) as u32;
    score.min(500)
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
        let t0 = std::time::Instant::now();      // DEBUG

        let mut offset_counts: std::collections::HashMap<(i64, i32), u32> =
            std::collections::HashMap::new();
        let min_score_gate = dynamic_min_match_score(hashes.len(), cfg);

        // Build a map of hash -> query_anchor_time for quick lookup
        let hash_to_query_time: std::collections::HashMap<u32, Vec<u32>> = {
            let mut map: std::collections::HashMap<u32, Vec<u32>> = std::collections::HashMap::new();
            for &(hash, query_time) in hashes {
                map.entry(hash).or_insert_with(Vec::new).push(query_time);
            }
            map
        };


        // Batch query: use IN clause for better performance
        // SQLite can handle up to 999 parameters, so we batch in chunks
        let unique_hashes: Vec<i64> = hash_to_query_time.keys().map(|&h| h as i64).collect();
        const BATCH_SIZE: usize = 500; // Safe batch size for SQLite
        
        eprintln!("[DEBUG]: unique hashes: {}, batches: {}", unique_hashes.len(), unique_hashes.len().div_ceil(500));

        for chunk in unique_hashes.chunks(BATCH_SIZE) {
            let t_chunk = std::time::Instant::now();     // DEBUG

            let placeholders = chunk.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            let query = format!(
                "SELECT hash, song_id, anchor_time_ms FROM fingerprints WHERE hash IN ({}) LIMIT 90000",
                placeholders
            );

            let mut stmt = self.conn.prepare(&query)?;
            eprintln!("[DEBUG]: Prepare took {}ms", t_chunk.elapsed().as_millis());

            let params: Vec<&dyn rusqlite::ToSql> = chunk.iter().map(|h| h as &dyn rusqlite::ToSql).collect();
            let rows = stmt.query_map(params.as_slice(), |row: &rusqlite::Row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?))
            })?;

            for row in rows {
                let (hash, song_id, db_time_ms) = row?;
                
                // For each query time that had this hash
                if let Some(query_times) = hash_to_query_time.get(&(hash as u32)) {
                    for &query_time in query_times {
                        let offset = db_time_ms as i32 - query_time as i32;
                        *offset_counts.entry((song_id, offset)).or_insert(0) += 1;
                    }
                }
            }
            eprintln!("[DEBUG]: full chunk took {}ms", t_chunk.elapsed().as_millis());
        }
        eprintln!("[DEBUG]: DB lookup took {}ms", t0.elapsed().as_millis());
        
        //--------------------------------------------------------//
        //-- ii. Compute best offset score per song             --//
        //--------------------------------------------------------//
        let t1 = std::time::Instant::now();      // DEBUG

        let mut scores: std::collections::HashMap<i64, u32> = std::collections::HashMap::new();
        for ((song_id, _offset), count) in offset_counts {
            let entry = scores.entry(song_id).or_insert(0);
            if count > *entry {
                *entry = count;
            }
        }
        eprintln!("[DEBUG]: Offset voting took {}ms", t1.elapsed().as_millis());
        
        //-------------------------------------------//
        //-- iii. Sort by score and fetch metadata --//
        //-------------------------------------------//
        let t2 = std::time::Instant::now();      // DEBUG

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
        eprintln!("[DEBUG]: Scoring took {}ms", t2.elapsed().as_millis());
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
        assert_eq!(gate, 500);  // Cap at 500
    }
}
