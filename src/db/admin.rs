use crate::db::create_db::Database;
use rusqlite::{Result, params};

impl Database {
    pub fn list_songs(&self) -> Result<Vec<(i64, String, String, String, u64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT s.id, s.title, s.artist, s.path, COUNT(f.hash) as fp_count
             FROM songs s
             LEFT JOIN fingerprints f ON s.id = f.song_id
             GROUP BY s.id, s.title, s.artist, s.path
             ORDER BY s.id ASC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)? as u64,
            ))
        })?;

        let mut songs = Vec::new();
        for row in rows {
            songs.push(row?);
        }

        Ok(songs)
    }

    pub fn remove_song_by_id(&self, song_id: i64) -> Result<usize> {
        self.conn
            .execute("DELETE FROM songs WHERE id = ?", params![song_id])
    }

    pub fn db_stats(&self) -> Result<(u64, u64)> {
        let song_count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM songs", [], |row| row.get(0))?;

        let fingerprint_count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM fingerprints", [], |row| row.get(0))?;

        Ok((song_count as u64, fingerprint_count as u64))
    }
}
