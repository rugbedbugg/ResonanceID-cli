use resonanceid_cli::{
    config::AppConfig,
    db::create_db::Database,
    pipeline::{fingerprint_wav, fingerprint_wav_with_report_and_clip, ClipOptions},
};

const DEFAULT_DB_PATH: &str = "resonanceid-cli.db";
const MIN_RECOMMENDED_INDEX_DURATION_SECONDS: f32 = 15.0;

enum Command {
    Store {
        wav_path: String,
        title: String,
        artist: String,
        db_path: String,
        config_path: Option<String>,
        no_config: bool,
        overrides: Overrides,
    },
    Recognize {
        wav_path: String,
        db_path: String,
        config_path: Option<String>,
        no_config: bool,
        overrides: Overrides,
    },
    ListTopMatches {
        wav_path: String,
        db_path: String,
        config_path: Option<String>,
        no_config: bool,
        overrides: Overrides,
    },
    ListSongs {
        db_path: String,
    },
    RemoveSong {
        song_id: i64,
        db_path: String,
    },
    DbStats {
        db_path: String,
    },
}

#[derive(Default, Clone)]
struct Overrides {
    window_size: Option<usize>,
    hop_size: Option<usize>,
    anchor_window: Option<usize>,
    threshold_db: Option<f32>,
    min_match_score: Option<u32>,
    dynamic_gate_scale: Option<f32>,
    small_query_threshold: Option<usize>,
    max_results: Option<usize>,
    clip_start_seconds: Option<f32>,
    clip_duration_seconds: Option<f32>,
    auto_clip: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: Vec<String> = std::env::args().collect();

    // tolerate accidental `cargo run src/main.rs ...` invocation
    if args.len() > 1 && args[1].ends_with(".rs") {
        args.remove(1);
    }

    let command = match parse_cli(&args) {
        Ok(cmd) => cmd,
        Err(err) => {
            if err.to_string() == "help requested" {
                return Ok(());
            }
            return Err(err);
        }
    };

    match command {
        Command::Store {
            wav_path,
            title,
            artist,
            db_path,
            config_path,
            no_config,
            overrides,
        } => {
            let run_start = std::time::Instant::now();
            let (mut cfg, config_report) = AppConfig::load_with_report(config_path.as_deref(), no_config)?;
            apply_overrides(&mut cfg, &overrides);

            let mut db = Database::open(&db_path)?;
            let clip_options = ClipOptions {
                clip_start_seconds: overrides.clip_start_seconds,
                clip_duration_seconds: overrides.clip_duration_seconds,
                auto_clip: overrides.auto_clip,
            };
            let (fingerprints, report) = fingerprint_wav_with_report_and_clip(
                &wav_path,
                cfg.fingerprint.threshold_db,
                cfg.fingerprint.window_size,
                cfg.fingerprint.hop_size,
                cfg.fingerprint.anchor_window,
                clip_options,
            )?;

            db.register_song(&wav_path, &title, &artist, &fingerprints)?;

            println!("✅ Stored '{}' by '{}'", title, artist);
            if !config_report.loaded_paths.is_empty() {
                println!("Config: loaded from {}", config_report.loaded_paths.join(", "));
            }
            println!("Path: {}", wav_path);
            println!("Database: {}", db_path);
            println!("Sample Rate: {} Hz", report.sample_rate);
            println!("Duration: {:.2} s", report.duration_seconds);
            println!(
                "Clip Used: start={:.2}s, duration={:.2}s",
                report.clip_start_seconds,
                report.clip_duration_seconds
            );
            println!("Samples: {}", report.sample_count);
            println!("Frames: {}", report.frame_count);
            println!("Peaks: {}", report.peak_count);
            println!("Fingerprints: {}", report.fingerprint_count);
            println!(
                "Params: window_size={}, hop_size={}, anchor_window={}, threshold_db={}",
                cfg.fingerprint.window_size,
                cfg.fingerprint.hop_size,
                cfg.fingerprint.anchor_window,
                cfg.fingerprint.threshold_db
            );
            println!("Index Time: {} ms", run_start.elapsed().as_millis());

            if should_warn_short_index(report.duration_seconds) {
                println!(
                    "⚠️ Warning: indexed audio is {:.2}s (recommended >= {:.0}s for stable identification)",
                    report.duration_seconds,
                    MIN_RECOMMENDED_INDEX_DURATION_SECONDS
                );
                println!("   Tip: use 'recognize' for snippets and 'store/remember' for reference tracks.");
            }
        }
        Command::Recognize {
            wav_path,
            db_path,
            config_path,
            no_config,
            overrides,
        } => {
            let (mut cfg, config_report) = AppConfig::load_with_report(config_path.as_deref(), no_config)?;
            apply_overrides(&mut cfg, &overrides);

            let db = Database::open(&db_path)?;
            let fingerprints = fingerprint_wav(
                &wav_path,
                cfg.fingerprint.threshold_db,
                cfg.fingerprint.window_size,
                cfg.fingerprint.hop_size,
                cfg.fingerprint.anchor_window,
            )?;

            let matches = db.recognize_song_with_config(&fingerprints, &cfg.recognition)?;
            if !config_report.loaded_paths.is_empty() {
                println!("Config: loaded from {}", config_report.loaded_paths.join(", "));
            }
            if let Some((title, artist, score)) = matches.first() {
                println!(
                    "✅ Match found\nTop Match: {} - {} (match score: {})",
                    title, artist, score
                );
            } else {
                println!("❌ No matches found");
            }

            for (idx, (title, artist, score)) in matches.iter().enumerate() {
                println!("{}. {} - {} (score: {})", idx + 1, title, artist, score);
            }
        }
        Command::ListTopMatches {
            wav_path,
            db_path,
            config_path,
            no_config,
            overrides,
        } => {
            let (mut cfg, config_report) = AppConfig::load_with_report(config_path.as_deref(), no_config)?;
            apply_overrides(&mut cfg, &overrides);

            let db = Database::open(&db_path)?;
            let fingerprints = fingerprint_wav(
                &wav_path,
                cfg.fingerprint.threshold_db,
                cfg.fingerprint.window_size,
                cfg.fingerprint.hop_size,
                cfg.fingerprint.anchor_window,
            )?;

            let matches = db.recognize_song_with_config(&fingerprints, &cfg.recognition)?;
            if !config_report.loaded_paths.is_empty() {
                println!("Config: loaded from {}", config_report.loaded_paths.join(", "));
            }
            if matches.is_empty() {
                println!("❌ No matches found");
            } else {
                println!("Top matches:");
                for (idx, (title, artist, score)) in matches.iter().enumerate() {
                    println!("{}. {} - {} (score: {})", idx + 1, title, artist, score);
                }
            }
        }
        Command::ListSongs { db_path } => {
            let db = Database::open(&db_path)?;
            let songs = db.list_songs()?;
            if songs.is_empty() {
                println!("No songs stored.");
            } else {
                for (id, title, artist, path, fp_count) in songs {
                    println!("{} | {} - {} | fingerprints={} | {}", id, title, artist, fp_count, path);
                }
            }
        }
        Command::RemoveSong { song_id, db_path } => {
            let db = Database::open(&db_path)?;
            let removed = db.remove_song_by_id(song_id)?;
            if removed > 0 {
                println!("✅ Removed song id {}", song_id);
            } else {
                println!("⚠️ No song found for id {}", song_id);
            }
        }
        Command::DbStats { db_path } => {
            let db = Database::open(&db_path)?;
            let (song_count, fingerprint_count) = db.db_stats()?;
            println!("Songs: {}", song_count);
            println!("Fingerprints: {}", fingerprint_count);
        }
    }

    Ok(())
}

fn parse_cli(args: &[String]) -> Result<Command, Box<dyn std::error::Error>> {
    if args.len() < 2 {
        print_usage();
        return Err("missing command".into());
    }

    match args[1].as_str() {
        "store" | "remember" | "index" => {
            // resonanceid-cli store <wav_path> <title> <artist> [options]
            if has_help_flag(args, 2) {
                print_store_usage();
                return Err("help requested".into());
            }
            if args.len() < 5 {
                print_store_usage();
                return Err("store requires <wav_path> <title> <artist>".into());
            }

            let wav_path = args[2].clone();
            let title = args[3].clone();
            let artist = args[4].clone();
            let (db_path, config_path, no_config, overrides) = parse_common_options(args, 5)?;

            Ok(Command::Store {
                wav_path,
                title,
                artist,
                db_path,
                config_path,
                no_config,
                overrides,
            })
        }
        "recognize" => {
            // resonanceid-cli recognize <wav_path> [options]
            if has_help_flag(args, 2) {
                print_recognize_usage();
                return Err("help requested".into());
            }
            if args.len() < 3 {
                print_recognize_usage();
                return Err("recognize requires <wav_path>".into());
            }

            let wav_path = args[2].clone();
            let (db_path, config_path, no_config, overrides) = parse_common_options(args, 3)?;

            Ok(Command::Recognize {
                wav_path,
                db_path,
                config_path,
                no_config,
                overrides,
            })
        }
        "list-top-matches" => {
            // resonanceid-cli list-top-matches <wav_path> [options]
            if has_help_flag(args, 2) {
                print_list_top_matches_usage();
                return Err("help requested".into());
            }
            if args.len() < 3 {
                print_list_top_matches_usage();
                return Err("list-top-matches requires <wav_path>".into());
            }

            let wav_path = args[2].clone();
            let (db_path, config_path, no_config, overrides) = parse_common_options(args, 3)?;

            Ok(Command::ListTopMatches {
                wav_path,
                db_path,
                config_path,
                no_config,
                overrides,
            })
        }
        "list-songs" => {
            if has_help_flag(args, 2) {
                print_list_songs_usage();
                return Err("help requested".into());
            }
            let db_path = parse_db_only_option(args, 2)?;
            Ok(Command::ListSongs { db_path })
        }
        "remove-song" => {
            if has_help_flag(args, 2) {
                print_remove_song_usage();
                return Err("help requested".into());
            }
            if args.len() < 3 {
                print_remove_song_usage();
                return Err("remove-song requires <song_id>".into());
            }
            let song_id: i64 = args[2].parse()?;
            let db_path = parse_db_only_option(args, 3)?;
            Ok(Command::RemoveSong { song_id, db_path })
        }
        "db-stats" => {
            if has_help_flag(args, 2) {
                print_db_stats_usage();
                return Err("help requested".into());
            }
            let db_path = parse_db_only_option(args, 2)?;
            Ok(Command::DbStats { db_path })
        }
        "help" | "--help" | "-h" => {
            print_usage();
            Err("help requested".into())
        }
        _ => {
            print_usage();
            Err("unknown command".into())
        }
    }
}

fn parse_db_only_option(args: &[String], offset: usize) -> Result<String, Box<dyn std::error::Error>> {
    if args.len() == offset {
        return Ok(DEFAULT_DB_PATH.to_string());
    }

    if args.len() == offset + 2 && args[offset] == "--db" {
        return Ok(args[offset + 1].clone());
    }

    Err("invalid db options".into())
}

fn parse_common_options(
    args: &[String],
    offset: usize,
) -> Result<(String, Option<String>, bool, Overrides), Box<dyn std::error::Error>> {
    let mut db_path = DEFAULT_DB_PATH.to_string();
    let mut config_path: Option<String> = None;
    let mut no_config = false;
    let mut overrides = Overrides::default();

    let mut i = offset;
    while i < args.len() {
        match args[i].as_str() {
            "--db" => {
                if i + 1 >= args.len() {
                    return Err("--db requires a value".into());
                }
                db_path = args[i + 1].clone();
                i += 2;
            }
            "--config" => {
                if i + 1 >= args.len() {
                    return Err("--config requires a value".into());
                }
                config_path = Some(args[i + 1].clone());
                i += 2;
            }
            "--no-config" => {
                no_config = true;
                i += 1;
            }
            "--window-size" => {
                if i + 1 >= args.len() {
                    return Err("--window-size requires a value".into());
                }
                overrides.window_size = Some(args[i + 1].parse()?);
                i += 2;
            }
            "--hop-size" => {
                if i + 1 >= args.len() {
                    return Err("--hop-size requires a value".into());
                }
                overrides.hop_size = Some(args[i + 1].parse()?);
                i += 2;
            }
            "--anchor-window" => {
                if i + 1 >= args.len() {
                    return Err("--anchor-window requires a value".into());
                }
                overrides.anchor_window = Some(args[i + 1].parse()?);
                i += 2;
            }
            "--threshold-db" => {
                if i + 1 >= args.len() {
                    return Err("--threshold-db requires a value".into());
                }
                overrides.threshold_db = Some(args[i + 1].parse()?);
                i += 2;
            }
            "--min-match-score" => {
                if i + 1 >= args.len() {
                    return Err("--min-match-score requires a value".into());
                }
                overrides.min_match_score = Some(args[i + 1].parse()?);
                i += 2;
            }
            "--dynamic-gate-scale" => {
                if i + 1 >= args.len() {
                    return Err("--dynamic-gate-scale requires a value".into());
                }
                overrides.dynamic_gate_scale = Some(args[i + 1].parse()?);
                i += 2;
            }
            "--small-query-threshold" => {
                if i + 1 >= args.len() {
                    return Err("--small-query-threshold requires a value".into());
                }
                overrides.small_query_threshold = Some(args[i + 1].parse()?);
                i += 2;
            }
            "--max-results" => {
                if i + 1 >= args.len() {
                    return Err("--max-results requires a value".into());
                }
                overrides.max_results = Some(args[i + 1].parse()?);
                i += 2;
            }
            "--clip-start" => {
                if i + 1 >= args.len() {
                    return Err("--clip-start requires a value".into());
                }
                overrides.clip_start_seconds = Some(args[i + 1].parse()?);
                i += 2;
            }
            "--clip-duration" => {
                if i + 1 >= args.len() {
                    return Err("--clip-duration requires a value".into());
                }
                overrides.clip_duration_seconds = Some(args[i + 1].parse()?);
                i += 2;
            }
            "--auto-clip" => {
                overrides.auto_clip = true;
                i += 1;
            }
            _ => {
                return Err("invalid arguments after required positional values".into());
            }
        }
    }

    Ok((db_path, config_path, no_config, overrides))
}

fn apply_overrides(cfg: &mut AppConfig, overrides: &Overrides) {
    if let Some(v) = overrides.window_size {
        cfg.fingerprint.window_size = v;
    }
    if let Some(v) = overrides.hop_size {
        cfg.fingerprint.hop_size = v;
    }
    if let Some(v) = overrides.anchor_window {
        cfg.fingerprint.anchor_window = v;
    }
    if let Some(v) = overrides.threshold_db {
        cfg.fingerprint.threshold_db = v;
    }
    if let Some(v) = overrides.min_match_score {
        cfg.recognition.min_match_score = v;
    }
    if let Some(v) = overrides.dynamic_gate_scale {
        cfg.recognition.dynamic_gate_scale = v;
    }
    if let Some(v) = overrides.small_query_threshold {
        cfg.recognition.small_query_threshold = v;
    }
    if let Some(v) = overrides.max_results {
        cfg.recognition.max_results = v;
    }
}

fn should_warn_short_index(duration_seconds: f32) -> bool {
    duration_seconds < MIN_RECOMMENDED_INDEX_DURATION_SECONDS
}

fn has_help_flag(args: &[String], offset: usize) -> bool {
    args.iter().skip(offset).any(|arg| arg == "--help" || arg == "-h")
}

fn print_usage() {
    println!("A chemistry-inspired audio fingerprint CLI.");
    println!();
    println!("USAGE");
    println!("  resonanceid-cli <command> [subcommand] [flags]");
    println!();
    println!("CORE COMMANDS");
    println!("  store             Save a reference track into the fingerprint database");
    println!("  remember          Alias for 'store'");
    println!("  recognize         Identify the best match for an input clip");
    println!("  list-top-matches  Show ranked candidates for a clip");
    println!("  list-songs        List songs currently stored in the database");
    println!("  remove-song       Remove a song by id");
    println!("  db-stats          Show song/fingerprint totals");
    println!();
    println!("FLAGS");
    println!("  --help            Show help for command");
    println!();
    println!("EXAMPLES");
    println!("  $ resonanceid-cli store song.wav \"Song\" \"Artist\"");
    println!("  $ resonanceid-cli recognize clip.wav");
    println!("  $ resonanceid-cli db-stats");
    println!();
    println!("LEARN MORE");
    println!("  Use 'resonanceid-cli <command> --help' for command-specific usage.");
    println!("  Version: v{}", env!("CARGO_PKG_VERSION"));
}

fn print_store_usage() {
    println!("USAGE");
    println!("  resonanceid-cli store <wav_path> <title> <artist> [options]");
    println!();
    println!("OPTIONS");
    println!("  --db <db_path>");
    println!("  --config <path>");
    println!("  --no-config");
    println!("  --window-size <n>");
    println!("  --hop-size <n>");
    println!("  --anchor-window <n>");
    println!("  --threshold-db <f32>");
    println!("  --clip-start <seconds>");
    println!("  --clip-duration <seconds>");
    println!("  --auto-clip");
}

fn print_recognize_usage() {
    println!("USAGE");
    println!("  resonanceid-cli recognize <wav_path> [options]");
    println!();
    println!("OPTIONS");
    println!("  --db <db_path>");
    println!("  --config <path>");
    println!("  --no-config");
    println!("  --window-size <n>");
    println!("  --hop-size <n>");
    println!("  --anchor-window <n>");
    println!("  --threshold-db <f32>");
    println!("  --min-match-score <n>");
    println!("  --dynamic-gate-scale <f32>");
    println!("  --small-query-threshold <n>");
    println!("  --max-results <n>");
}

fn print_list_top_matches_usage() {
    println!("USAGE");
    println!("  resonanceid-cli list-top-matches <wav_path> [options]");
    println!();
    println!("TIP");
    println!("  Uses same options as 'recognize'.");
}

fn print_list_songs_usage() {
    println!("USAGE");
    println!("  resonanceid-cli list-songs [--db <db_path>]");
}

fn print_remove_song_usage() {
    println!("USAGE");
    println!("  resonanceid-cli remove-song <song_id> [--db <db_path>]");
}

fn print_db_stats_usage() {
    println!("USAGE");
    println!("  resonanceid-cli db-stats [--db <db_path>]");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_store_command() {
        let args = vec![
            "resonanceid-cli".to_string(),
            "store".to_string(),
            "songs/output.wav".to_string(),
            "Test Song".to_string(),
            "Test Artist".to_string(),
        ];

        let command = parse_cli(&args).unwrap();
        match command {
            Command::Store {
                wav_path,
                title,
                artist,
                db_path,
                config_path,
                no_config,
                ..
            } => {
                assert_eq!(wav_path, "songs/output.wav");
                assert_eq!(title, "Test Song");
                assert_eq!(artist, "Test Artist");
                assert_eq!(db_path, DEFAULT_DB_PATH);
                assert!(config_path.is_none());
                assert!(!no_config);
            }
            _ => panic!("expected store command"),
        }
    }

    #[test]
    fn parse_recognize_command_with_custom_db() {
        let args = vec![
            "resonanceid-cli".to_string(),
            "recognize".to_string(),
            "snippet/clip.wav".to_string(),
            "--db".to_string(),
            "custom.db".to_string(),
        ];

        let command = parse_cli(&args).unwrap();
        match command {
            Command::Recognize {
                wav_path,
                db_path,
                config_path,
                no_config,
                ..
            } => {
                assert_eq!(wav_path, "snippet/clip.wav");
                assert_eq!(db_path, "custom.db");
                assert!(config_path.is_none());
                assert!(!no_config);
            }
            _ => panic!("expected recognize command"),
        }
    }

    #[test]
    fn fail_on_missing_command() {
        let args = vec!["resonanceid-cli".to_string()];
        assert!(parse_cli(&args).is_err());
    }

    #[test]
    fn warn_for_short_index_duration() {
        assert!(should_warn_short_index(5.0));
        assert!(!should_warn_short_index(25.0));
    }

    #[test]
    fn parse_store_command_with_config_and_clip_flags() {
        let args = vec![
            "resonanceid-cli".to_string(),
            "remember".to_string(),
            "songs/output.wav".to_string(),
            "Test Song".to_string(),
            "Test Artist".to_string(),
            "--config".to_string(),
            "custom.toml".to_string(),
            "--no-config".to_string(),
            "--clip-start".to_string(),
            "12.5".to_string(),
            "--clip-duration".to_string(),
            "20".to_string(),
            "--auto-clip".to_string(),
        ];

        let command = parse_cli(&args).unwrap();
        match command {
            Command::Store {
                config_path,
                no_config,
                overrides,
                ..
            } => {
                assert_eq!(config_path.unwrap(), "custom.toml");
                assert!(no_config);
                assert_eq!(overrides.clip_start_seconds, Some(12.5));
                assert_eq!(overrides.clip_duration_seconds, Some(20.0));
                assert!(overrides.auto_clip);
            }
            _ => panic!("expected store command"),
        }
    }

    #[test]
    fn parse_recognize_with_override_flags() {
        let args = vec![
            "resonanceid-cli".to_string(),
            "recognize".to_string(),
            "snippet/clip.wav".to_string(),
            "--window-size".to_string(),
            "2048".to_string(),
            "--threshold-db".to_string(),
            "-30".to_string(),
            "--min-match-score".to_string(),
            "10".to_string(),
        ];

        let command = parse_cli(&args).unwrap();
        match command {
            Command::Recognize { overrides, .. } => {
                assert_eq!(overrides.window_size, Some(2048));
                assert_eq!(overrides.threshold_db, Some(-30.0));
                assert_eq!(overrides.min_match_score, Some(10));
            }
            _ => panic!("expected recognize command"),
        }
    }

    #[test]
    fn parse_list_top_matches_command() {
        let args = vec![
            "resonanceid-cli".to_string(),
            "list-top-matches".to_string(),
            "snippet/clip.wav".to_string(),
            "--max-results".to_string(),
            "3".to_string(),
        ];

        let command = parse_cli(&args).unwrap();
        match command {
            Command::ListTopMatches { overrides, .. } => {
                assert_eq!(overrides.max_results, Some(3));
            }
            _ => panic!("expected list-top-matches command"),
        }
    }

    #[test]
    fn parse_list_songs_command() {
        let args = vec!["resonanceid-cli".to_string(), "list-songs".to_string()];
        let command = parse_cli(&args).unwrap();
        match command {
            Command::ListSongs { db_path } => assert_eq!(db_path, DEFAULT_DB_PATH),
            _ => panic!("expected list-songs command"),
        }
    }

    #[test]
    fn parse_remove_song_command() {
        let args = vec![
            "resonanceid-cli".to_string(),
            "remove-song".to_string(),
            "7".to_string(),
            "--db".to_string(),
            "x.db".to_string(),
        ];
        let command = parse_cli(&args).unwrap();
        match command {
            Command::RemoveSong { song_id, db_path } => {
                assert_eq!(song_id, 7);
                assert_eq!(db_path, "x.db");
            }
            _ => panic!("expected remove-song command"),
        }
    }

    #[test]
    fn parse_db_stats_command() {
        let args = vec!["resonanceid-cli".to_string(), "db-stats".to_string()];
        let command = parse_cli(&args).unwrap();
        match command {
            Command::DbStats { db_path } => assert_eq!(db_path, DEFAULT_DB_PATH),
            _ => panic!("expected db-stats command"),
        }
    }

    #[test]
    fn help_flag_for_store_command() {
        let args = vec![
            "resonanceid-cli".to_string(),
            "store".to_string(),
            "--help".to_string(),
        ];
        assert!(parse_cli(&args).is_err());
    }
}
