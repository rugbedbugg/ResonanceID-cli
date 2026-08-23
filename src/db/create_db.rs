use rusqlite::{Connection, Result};

/// Schema version tracked via PRAGMA user_version.
///
/// 1: one row per fingerprint (hash, song_id, anchor_time_ms)
/// 2: one row per (hash, song_id) with anchor times packed into a BLOB
const SCHEMA_VERSION: i64 = 2;

/// Database abstraction.
/// Owns a SQLite connection and provides:
/// 1. Song registration    (indexing)
/// 2. Song recognition     (querying)
pub struct Database {
    pub(crate) conn: Connection,
    /// Path retained so recognition can open extra read-only connections
    /// for parallel hash lookups.
    pub(crate) path: String,
}

impl Database {
    ///////////////////////////////////////////////
    // Open Database (schema + performance setup)//
    ///////////////////////////////////////////////
    // Opens/creates the database file and ensures required tables exist.
    // i.   Open db connection
    // ii.  Apply PRAGMAs
    // iii. Migrate legacy schemas if needed
    // iv.  Create tables + indexes
    pub fn open(path: &str) -> Result<Self> {
        //------------------------------//
        //-- OPEN DATABASE CONNECTION --//
        //------------------------------//
        let mut conn = Connection::open(path)?;

        //----------------------------------------//
        //-- PRAGMAs for performance and safety --//
        //----------------------------------------//
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;\
             PRAGMA journal_mode = WAL;\
             PRAGMA synchronous = NORMAL;\
             PRAGMA temp_store = MEMORY;",
        )?;

        //---------------------------------------------------//
        //-- Migrate legacy row-per-fingerprint databases  --//
        //---------------------------------------------------//
        let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version < SCHEMA_VERSION {
            migrate_legacy_fingerprints(&mut conn)?;
            conn.execute_batch(&format!("PRAGMA user_version = {SCHEMA_VERSION};"))?;
        }

        //----------------------------------------------//
        //-- i. Create 'songs' table
        //--    - stores song metadata
        //--
        //--    Attributes: (id, path, title, artist)
        //----------------------------------------------//
        conn.execute(
            "CREATE TABLE IF NOT EXISTS songs ( \
                id INTEGER PRIMARY KEY, \
                path TEXT UNIQUE, \
                title TEXT, \
                artist TEXT \
            )",
            [],
        )?;
        //---------------------------------------------------------------//
        //-- ii. Create 'fingerprints' table
        //--     - compact layout: one row per (hash, song_id) pair whose
        //--       BLOB holds the anchor times as little-endian u32 ms values
        //---------------------------------------------------------------//
        conn.execute(
            "CREATE TABLE IF NOT EXISTS fingerprints ( \
                hash INTEGER NOT NULL, \
                song_id INTEGER NOT NULL, \
                anchor_times BLOB NOT NULL, \
                PRIMARY KEY (hash, song_id), \
                FOREIGN KEY (song_id) REFERENCES songs(id) ON DELETE CASCADE \
            )",
            [],
        )?;

        //------------------------------------------//
        //-- Index for song-based lookups         --//
        //-- (hash lookups are covered by the     --//
        //--  leading column of the primary key)  --//
        //------------------------------------------//
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_fingerprints_song \
             ON fingerprints(song_id)",
            [],
        )?;

        //--------------------------------//
        //-- RETURN DATABASE CONNECTION --//
        //--------------------------------//
        Ok(Database { conn, path: path.to_string() })
    }
}

//-------------------------------------------------------------------------//
//-- Legacy migration                                                    --//
//-- Converts row-per-fingerprint tables into the packed BLOB layout.    --//
//-- Runs inside a transaction so an interrupted migration rolls back.   --//
//-------------------------------------------------------------------------//
fn migrate_legacy_fingerprints(conn: &mut Connection) -> Result<()> {
    let has_legacy_column = {
        let mut stmt = conn.prepare("PRAGMA table_info(fingerprints)")?;
        let names = stmt.query_map([], |row| row.get::<_, String>(1))?;
        let mut found = false;
        for name in names {
            if name? == "anchor_time_ms" {
                found = true;
            }
        }
        found
    };

    if !has_legacy_column {
        return Ok(());
    }

    eprintln!(
        "Migrating fingerprints to packed storage format (one-time, may take a moment)..."
    );
    let tx = conn.transaction()?;

    tx.execute(
        "CREATE TABLE fingerprints_packed ( \
            hash INTEGER NOT NULL, \
            song_id INTEGER NOT NULL, \
            anchor_times BLOB NOT NULL, \
            PRIMARY KEY (hash, song_id), \
            FOREIGN KEY (song_id) REFERENCES songs(id) ON DELETE CASCADE \
         )",
        [],
    )?;

    {
        use crate::db::fingerprint_codec::pack_anchor_times;
        use std::collections::HashMap;

        let mut grouped: HashMap<(i64, i64), Vec<u32>> = HashMap::new();
        {
            let mut stmt = tx.prepare(
                "SELECT hash, song_id, anchor_time_ms FROM fingerprints \
                 ORDER BY hash, song_id, anchor_time_ms",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })?;

            for row in rows {
                let (hash, song_id, time_ms) = row?;
                grouped
                    .entry((hash, song_id))
                    .or_default()
                    .push(time_ms as u32);
            }
        }

        let mut insert = tx.prepare(
            "INSERT INTO fingerprints_packed (hash, song_id, anchor_times) \
             VALUES (?, ?, ?)",
        )?;
        for ((hash, song_id), mut times) in grouped {
            times.sort_unstable();
            times.dedup();
            insert.execute(rusqlite::params![
                hash,
                song_id,
                pack_anchor_times(&times)
            ])?;
        }
        drop(insert);
    }

    tx.execute("DROP TABLE fingerprints", [])?;
    tx.execute(
        "ALTER TABLE fingerprints_packed RENAME TO fingerprints",
        [],
    )?;
    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_fingerprints_song ON fingerprints(song_id)",
        [],
    )?;

    tx.commit()?;

    // DROP TABLE leaves the freed pages in the file; reclaim them so the
    // packed layout actually shrinks the database on disk.
    conn.execute("VACUUM", [])?;

    Ok(())
}
