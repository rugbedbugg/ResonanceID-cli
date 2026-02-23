use rusqlite::{Connection, Result};

/// Database abstraction.
/// Owns a SQLite connection and provides:
/// 1. Song registration    (indexing)
/// 2. Song recognition     (querying)
pub struct Database {
    pub(crate) conn: Connection,
}

impl Database {
    ///////////////////////////////////////////////
    // Open Database (schema + performance setup)//
    ///////////////////////////////////////////////
    // Opens/creates the database file and ensures required tables exist.
    // i.   Open db connection
    // ii.  Apply PRAGMAs
    // iii. Create tables
    // iv.  Create indexes
    pub fn open(path: &str) -> Result<Self> {
        //------------------------------//
        //-- OPEN DATABASE CONNECTION --//
        //------------------------------//
        let conn = Connection::open(path)?;

        //----------------------------------------//
        //-- PRAGMAs for performance and safety --//
        //----------------------------------------//
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;\
             PRAGMA journal_mode = WAL;\
             PRAGMA synchronous = NORMAL;\
             PRAGMA temp_store = MEMORY;",
        )?;

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
        //--     - stores audio fingerprints for matching
        //--
        //--     Attributes: (hash, song_id, anchor_time_ms)
        //--
        //--     anchor_time_ms = time position of anchor peak in song
        //---------------------------------------------------------------//
        conn.execute(
            "CREATE TABLE IF NOT EXISTS fingerprints ( \
                hash INTEGER NOT NULL, \
                song_id INTEGER NOT NULL, \
                anchor_time_ms INTEGER NOT NULL, \
                PRIMARY KEY (hash, song_id, anchor_time_ms), \
                FOREIGN KEY (song_id) REFERENCES songs(id) ON DELETE CASCADE \
            )",
            [],
        )?;

        //------------------------------------------//
        //-- Indexes for fast recognition lookups --//
        //------------------------------------------//
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_fingerprints_hash \
             ON fingerprints(hash)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_fingerprints_song \
             ON fingerprints(song_id)",
            [],
        )?;

        //--------------------------------//
        //-- RETURN DATABASE CONNECTION --//
        //--------------------------------//
        Ok(Database { conn })
    }
}
