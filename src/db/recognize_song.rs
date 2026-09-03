use crate::{config::RecognitionConfig, db::create_db::Database};
use rusqlite::{Connection, Result, params};
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

/// Offset votes are keyed by (song_id, offset) packed into a single u64 so
/// each vote costs one hash lookup instead of hashing a tuple. FNV-1a is
/// noticeably faster than the default SipHash for this workload and the key
/// distribution is fine for a non-cryptographic hash.
const FNV_PRIME: u64 = 0x100000001b3;

#[derive(Default)]
struct FnvHasher(u64);

impl Hasher for FnvHasher {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        let mut hash = std::mem::take(self).0;
        for &b in bytes {
            hash ^= b as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        *self = FnvHasher(hash);
    }
}

/// Packs (song_id, offset) into one u64 key. song_id is truncated to its
/// lower 32 bits; SQLite rowids stay far below that in practice, but this
/// is a deliberate narrowing versus the old full-i64 tuple key.
fn pack_vote_key(song_id: i64, offset: i32) -> u64 {
    ((offset as u32 as u64) << 32) | (song_id as u64 & 0xFFFF_FFFF)
}

fn unpack_vote_key(key: u64) -> (i64, i32) {
    ((key & 0xFFFF_FFFF) as i64, (key >> 32) as u32 as i32)
}

type VoteMap = HashMap<u64, u32, BuildHasherDefault<FnvHasher>>;

/// Safe batch size for SQLite parameter binding.
const BATCH_SIZE: usize = 500;

fn dynamic_min_match_score(query_hash_count: usize, cfg: &RecognitionConfig) -> u32 {
    if query_hash_count < cfg.small_query_threshold {
        return cfg.min_match_score;
    }

    // Cap at 500 to prevent gate from becoming huge on large queries
    let score = ((query_hash_count as f32).sqrt() * cfg.dynamic_gate_scale) as u32;
    score.min(500)
}

/// Builds the `hash IN (...)` lookup for a given batch size.
fn lookup_sql(placeholders: usize) -> String {
    format!(
        "SELECT hash, song_id, anchor_times FROM fingerprints WHERE hash IN ({})",
        vec!["?"; placeholders].join(",")
    )
}

/// Runs one hash-batch lookup, accumulating offset votes into `votes`.
fn query_batch(
    stmt: &mut rusqlite::Statement<'_>,
    batch: &[i64],
    hash_to_query_time: &HashMap<u32, Vec<u32>>,
    votes: &mut VoteMap,
) -> Result<()> {
    let params: Vec<&dyn rusqlite::ToSql> =
        batch.iter().map(|h| h as &dyn rusqlite::ToSql).collect();
    let rows = stmt.query_map(params.as_slice(), |row: &rusqlite::Row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, Vec<u8>>(2)?,
        ))
    })?;

    for row in rows {
        let (hash, song_id, blob) = row?;
        let Some(query_times) = hash_to_query_time.get(&(hash as u32)) else {
            continue;
        };

        // Iterate packed times directly; sorted ascending.
        for &chunk in blob.as_chunks::<4>().0 {
            let db_time = i32::from_le_bytes(chunk);
            for &query_time in query_times.iter() {
                let offset = db_time.wrapping_sub(query_time as i32);
                *votes.entry(pack_vote_key(song_id, offset)).or_insert(0) += 1;
            }
        }
    }

    Ok(())
}

/// Processes one worker's share of the batches against `conn`,
/// accumulating offset votes.
fn collect_votes(
    conn: &Connection,
    batches: &[&[i64]],
    hash_to_query_time: &HashMap<u32, Vec<u32>>,
) -> Result<VoteMap> {
    let mut votes: VoteMap = HashMap::default();
    let mut stmt_full = conn.prepare(&lookup_sql(BATCH_SIZE))?;

    for batch in batches {
        if batch.len() == BATCH_SIZE {
            query_batch(&mut stmt_full, batch, hash_to_query_time, &mut votes)?;
        } else {
            let mut stmt = conn.prepare(&lookup_sql(batch.len()))?;
            query_batch(&mut stmt, batch, hash_to_query_time, &mut votes)?;
        }
    }

    Ok(votes)
}

impl Database {
    //////////////////////
    // Recognize a Song //
    //////////////////////
    // Identifies the best-matching song by offset consistency.
    // i.   Collect offset votes for matching hashes
    // ii.  Compute best offset score per song
    // iii. Rank and fetch metadata
    pub fn recognize_song(&self, hashes: &[(u32, u32)]) -> Result<Vec<(String, f32)>> {
        self.recognize_song_with_config(hashes, &RecognitionConfig::default())
    }

    pub fn recognize_song_with_config(
        &self,
        hashes: &[(u32, u32)],
        cfg: &RecognitionConfig,
    ) -> Result<Vec<(String, f32)>> {
        //---------------------------------------//
        //-- i. Candidate collection by offset --//
        //---------------------------------------//
        let mut offset_counts: VoteMap = HashMap::default();
        let min_score_gate = dynamic_min_match_score(hashes.len(), cfg);

        // Build a map of hash -> query_anchor_time for quick lookup.
        // Duplicates are preserved to keep vote counts identical.
        let mut hash_to_query_time: HashMap<u32, Vec<u32>> = HashMap::new();
        for &(hash, query_time) in hashes {
            hash_to_query_time.entry(hash).or_default().push(query_time);
        }
        for times in hash_to_query_time.values_mut() {
            times.sort_unstable();
        }
        let hash_to_query_time = std::sync::Arc::new(hash_to_query_time);

        let unique_hashes: Vec<i64> =
            hash_to_query_time.keys().map(|&h| h as i64).collect();

        // Nothing to look up (e.g. silence produced zero fingerprints);
        // avoids a zero-sized chunk panic below.
        if unique_hashes.is_empty() {
            return Ok(Vec::new());
        }

        //---------------------------------------------------------------//
        //-- Parallel lookups: one read-only connection per worker,    --//
        //-- each draining its share of the batches into a partial     --//
        //-- vote map. Merged afterwards; results are order-independent.--
        //-- In-memory databases can't be shared across connections,   --//
        //-- so those fall back to a sequential pass on the main one.  --
        //---------------------------------------------------------------//
        let batches: Vec<&[i64]> = unique_hashes.chunks(BATCH_SIZE).collect();
        let joined: Vec<Result<VoteMap>> = if self.path == ":memory:" {
            Vec::new()
        } else {
            let workers = std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1)
                .min(batches.len());

            std::thread::scope(|scope| {
                let handles: Vec<_> = batches
                    .chunks(batches.len().div_ceil(workers))
                    .map(|worker_batches| {
                        let path = self.path.clone();
                        let hash_to_query_time = std::sync::Arc::clone(&hash_to_query_time);
                        scope.spawn(move || -> Result<VoteMap> {
                            let conn = Connection::open_with_flags(
                                &path,
                                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
                                    | rusqlite::OpenFlags::SQLITE_OPEN_URI,
                            )?;
                            conn.execute_batch(
                                "PRAGMA mmap_size = 268435456; \
                                 PRAGMA cache_size = -65536;",
                            )?;

                            collect_votes(&conn, worker_batches, &hash_to_query_time)
                        })
                    })
                    .collect();

                handles
                    .into_iter()
                    .map(|handle| handle.join().expect("recognition worker panicked"))
                    .collect()
            })
        };

        for partial in joined.into_iter().collect::<Result<Vec<_>>>()? {
            for (key, count) in partial {
                *offset_counts.entry(key).or_insert(0) += count;
            }
        }

        // Fallback path for in-memory databases (single connection).
        if self.path == ":memory:" {
            let votes = collect_votes(&self.conn, &batches, &hash_to_query_time)?;
            for (key, count) in votes {
                *offset_counts.entry(key).or_insert(0) += count;
            }
        }

        //--------------------------------------------------------//
        //-- ii. Compute best offset score per song             --//
        //--------------------------------------------------------//

        let mut scores: std::collections::HashMap<i64, u32> = std::collections::HashMap::new();
        for (key, count) in offset_counts {
            let (song_id, _offset) = unpack_vote_key(key);
            let entry = scores.entry(song_id).or_insert(0);
            if count > *entry {
                *entry = count;
            }
        }

        //-------------------------------------------//
        //-- iii. Sort by score and fetch metadata --//
        //-------------------------------------------//

        let mut ranked: Vec<_> = scores.into_iter().collect();
        ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

        // Confidence margin: the winner must clearly beat the runner-up.
        // Near-tied top scores mean the query aligned with noise, so report
        // nothing rather than a guess. A lone candidate skips this check and
        // still has to clear min_score_gate below.
        if cfg.min_margin_ratio > 1.0
            && ranked.len() > 1
            && (ranked[0].1 as f32) < ranked[1].1 as f32 * cfg.min_margin_ratio
        {
            return Ok(Vec::new());
        }

        let mut results = Vec::new();
        for (song_id, score) in ranked.into_iter().take(cfg.max_results) {
            if score < min_score_gate {
                continue;
            }

            let name = self
                .conn
                .query_row(
                    "SELECT title FROM songs WHERE id=?",
                    params![song_id],
                    |row: &rusqlite::Row| row.get::<_, String>(0),
                )
                .unwrap_or_else(|_| "Unknown".to_string());

            results.push((name, score as f32));
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
        assert!(gate > cfg.min_match_score);
        assert!(gate <= 500);
    }

    #[test]
    fn vote_keys_round_trip() {
        let cases = [(1i64, 0i32), (42, -123_456), (130, 3_600_000), (u32::MAX as i64, i32::MIN)];
        for (song, offset) in cases {
            let (s, o) = unpack_vote_key(pack_vote_key(song, offset));
            assert_eq!((s, o), (song, offset));
        }
    }
}
