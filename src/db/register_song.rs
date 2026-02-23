use crate::db::create_db::Database;
use rusqlite::{Result, params};

impl Database {
    //////////////////////
    // Register a Song  //
    //////////////////////
    // Registers a song uniquely and stores its fingerprints.
    // i.   Upsert song metadata
    // ii.  Remove existing fingerprints (reindex)
    // iii. Insert fingerprints
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
        //--      Each fingerprint: (hash, song_id, anchor_time_ms) --//
        //------------------------------------------------------------//
        {
            let mut stmt = tx.prepare(
                "INSERT INTO \
                fingerprints (hash, song_id, anchor_time_ms) \
                VALUES (?, ?, ?) \
                ON CONFLICT(hash, song_id, anchor_time_ms) DO NOTHING",
            )?;

            for &(hash, anchor_time_ms) in hashes {
                stmt.execute(params![hash as i64, song_id, anchor_time_ms as i64])?;
            }
        }

        //---------------------------//
        //-- COMMIT DB TRANSACTION --//
        //---------------------------//
        tx.commit()
    }
}
