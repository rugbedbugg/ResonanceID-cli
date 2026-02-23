use crate::db::create_db::Database;
use rusqlite::{Result, params};

impl Database {
    //////////////////////
    // Recognize a Song //
    //////////////////////
    // Identifies the best-matching song by offset consistency.
    // i.   Collect offset votes for matching hashes
    // ii.  Compute best offset score per song
    // iii. Rank and fetch metadata
    pub fn recognize_song(&self, hashes: &[(u32, u32)]) -> Result<Vec<(String, String, f32)>> {
        //---------------------------------------//
        //-- i. Candidate collection by offset --//
        //---------------------------------------//
        let mut offset_counts: std::collections::HashMap<(i64, i32), u32> =
            std::collections::HashMap::new();

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
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let mut results = Vec::new();
        for (song_id, score) in ranked.into_iter().take(5) {
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
