use crate::db::create_db::Database;

fn temp_db_path() -> std::path::PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    path.push(format!("resonanceid_cli_test_{}_{}.db", std::process::id(), nanos));
    path
}

#[test]
fn register_and_recognize_by_offset_votes() {
    let db_path = temp_db_path();
    let mut db = Database::open(db_path.to_str().unwrap()).unwrap();

    let hashes = vec![(123, 100), (456, 200), (789, 300)];
    db.register_song("songs/output.wav", "Test Song", "Test Artist", &hashes)
        .unwrap();

    let query_hashes = vec![(123, 50), (456, 150), (789, 250)];
    let matches = db.recognize_song(&query_hashes).unwrap();

    assert!(!matches.is_empty());
    let (title, artist, score) = &matches[0];
    assert_eq!(title, "Test Song");
    assert_eq!(artist, "Test Artist");
    assert_eq!(*score as u32, 3);

    drop(db);
    let _ = std::fs::remove_file(db_path);
}

#[test]
fn reindex_replaces_existing_fingerprints() {
    let db_path = temp_db_path();
    let mut db = Database::open(db_path.to_str().unwrap()).unwrap();

    let hashes_v1 = vec![(111, 10), (222, 20)];
    db.register_song("songs/output.wav", "Test Song", "Test Artist", &hashes_v1)
        .unwrap();

    let hashes_v2 = vec![(333, 30), (444, 40)];
    db.register_song("songs/output.wav", "Test Song", "Test Artist", &hashes_v2)
        .unwrap();

    let matches_old = db.recognize_song(&hashes_v1).unwrap();
    assert!(matches_old.is_empty());

    let matches_new = db.recognize_song(&hashes_v2).unwrap();
    assert!(!matches_new.is_empty());

    drop(db);
    let _ = std::fs::remove_file(db_path);
}

#[test]
fn recognize_returns_empty_for_unknown_hashes() {
    let db_path = temp_db_path();
    let mut db = Database::open(db_path.to_str().unwrap()).unwrap();

    let hashes = vec![(123, 100), (456, 200), (789, 300)];
    db.register_song("songs/output.wav", "Test Song", "Test Artist", &hashes)
        .unwrap();

    let query_hashes = vec![(999_001, 50), (999_002, 150)];
    let matches = db.recognize_song(&query_hashes).unwrap();
    assert!(matches.is_empty());

    drop(db);
    let _ = std::fs::remove_file(db_path);
}

#[test]
fn weak_match_below_threshold_is_filtered() {
    let db_path = temp_db_path();
    let mut db = Database::open(db_path.to_str().unwrap()).unwrap();

    let hashes = vec![(123, 100), (456, 200), (789, 300)];
    db.register_song("songs/output.wav", "Test Song", "Test Artist", &hashes)
        .unwrap();

    // only one matching offset vote => below minimum score gate
    let weak_query = vec![(123, 50)];
    let matches = db.recognize_song(&weak_query).unwrap();
    assert!(matches.is_empty());

    drop(db);
    let _ = std::fs::remove_file(db_path);
}

#[test]
fn sparse_query_match_with_inconsistent_offsets_is_filtered() {
    let db_path = temp_db_path();
    let mut db = Database::open(db_path.to_str().unwrap()).unwrap();

    let hashes = vec![(1001, 100), (1002, 200), (1003, 300), (1004, 400)];
    db.register_song("songs/output.wav", "Test Song", "Test Artist", &hashes)
        .unwrap();

    // 30 query hashes, only 2 can match and they disagree on offset
    let mut query_hashes = Vec::new();
    for i in 0..28u32 {
        query_hashes.push((900_000 + i, 10 + i));
    }
    query_hashes.push((1001, 50));  // offset = 50
    query_hashes.push((1002, 151)); // offset = 49

    let matches = db.recognize_song(&query_hashes).unwrap();
    assert!(matches.is_empty());

    drop(db);
    let _ = std::fs::remove_file(db_path);
}
