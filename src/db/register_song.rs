use crate::db::create_db::Database;
use crate::db::fingerprint_codec::pack_anchor_times;
use rusqlite::{Result, params};
use std::collections::HashMap;

impl Database {
    //////////////////////
    // Register a Song  //
    //////////////////////
    // Registers a song uniquely and stores its fingerprints.
    // i.   Upsert song metadata
    // ii.  Remove existing fingerprints (reindex)
    // iii. Insert fingerprints packed as (hash, song_id -> [anchor_time_ms])
    pub fn register_song(
        &mut self,
        path: &str,
        title: &str,
        artist: &str,
        hashes: &[(u32, u32)], // (hash, anchor_time_ms)
    ) -> Result<()> {
        //--------------------------//
        //-- BEGIN DB TRANSACTION --//
        //--------------------------//
        let tx = self.conn.transaction()?;

        //---------------------------------------//
        //-- i. Insert or Update song metadata --//
        //---------------------------------------//
        let song_id: i64 = tx.query_row(
            "INSERT INTO \
            songs (path, title, artist) \
            VALUES (?, ?, ?) \
            \
            ON CONFLICT(path) \
            DO UPDATE SET \
            title = excluded.title, artist = excluded.artist \
            RETURNING id",
            params![path, title, artist],
            |row: &rusqlite::Row| row.get(0),
        )?;
        //-----------------------------------------------//
        //-- ii. Clear existing fingerprints (reindex) --//
        //-----------------------------------------------//
        tx.execute(
            "DELETE FROM fingerprints WHERE song_id = ?",
            params![song_id],
        )?;
        //------------------------------------------------------------//
        //-- iii. Insert fingerprints                               --//
        //--       Grouped per hash; each row packs the anchor      --//
        //--       times of one (hash, song_id) pair into a BLOB.   --//
        //------------------------------------------------------------//
        {
            let mut grouped: HashMap<u32, Vec<u32>> = HashMap::new();
            for &(hash, anchor_time_ms) in hashes {
                grouped.entry(hash).or_default().push(anchor_time_ms);
            }

            let mut grouped: Vec<(u32, Vec<u32>)> = grouped.into_iter().collect();
            grouped.sort_unstable_by_key(|&(hash, _)| hash);

            let mut stmt = tx.prepare(
                "INSERT INTO \
                fingerprints (hash, song_id, anchor_times) \
                VALUES (?, ?, ?)",
            )?;

            for (hash, mut times) in grouped {
                times.sort_unstable();
                times.dedup();
                stmt.execute(params![
                    hash as i64,
                    song_id,
                    pack_anchor_times(&times)
                ])?;
            }
        }

        //---------------------------//
        //-- COMMIT DB TRANSACTION --//
        //---------------------------//
        tx.commit()
    }
}
