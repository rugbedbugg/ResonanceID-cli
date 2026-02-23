use shazam::{
    config::AppConfig,
    db::create_db::Database,
    pipeline::{fingerprint_wav, fingerprint_wav_with_report},
};

const DEFAULT_DB_PATH: &str = "shazam.db";
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
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let command = parse_cli(&args)?;

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
            let (fingerprints, report) = fingerprint_wav_with_report(
                &wav_path,
                cfg.fingerprint.threshold_db,
                cfg.fingerprint.window_size,
                cfg.fingerprint.hop_size,
                cfg.fingerprint.anchor_window,
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
            // shazam store <wav_path> <title> <artist> [options]
            if args.len() < 5 {
                print_usage();
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
            // shazam recognize <wav_path> [options]
            if args.len() < 3 {
                print_usage();
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
            // shazam list-top-matches <wav_path> [options]
            if args.len() < 3 {
                print_usage();
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

fn print_usage() {
    println!("Usage:");
    println!("  shazam store <wav_path> <title> <artist> [--db <db_path>] [--config <path>] [--no-config] [--window-size <n>] [--hop-size <n>] [--anchor-window <n>] [--threshold-db <f32>]");
    println!("  shazam remember <wav_path> <title> <artist> ...   (alias for store)");
    println!("  shazam recognize <wav_path> [--db <db_path>] [--config <path>] [--no-config] [--window-size <n>] [--hop-size <n>] [--anchor-window <n>] [--threshold-db <f32>] [--min-match-score <n>] [--dynamic-gate-scale <f32>] [--small-query-threshold <n>] [--max-results <n>]");
    println!("  shazam list-top-matches <wav_path> [same options as recognize]");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_store_command() {
        let args = vec![
            "shazam".to_string(),
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
            "shazam".to_string(),
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
        let args = vec!["shazam".to_string()];
        assert!(parse_cli(&args).is_err());
    }

    #[test]
    fn warn_for_short_index_duration() {
        assert!(should_warn_short_index(5.0));
        assert!(!should_warn_short_index(25.0));
    }

    #[test]
    fn parse_store_command_with_config_flags() {
        let args = vec![
            "shazam".to_string(),
            "remember".to_string(),
            "songs/output.wav".to_string(),
            "Test Song".to_string(),
            "Test Artist".to_string(),
            "--config".to_string(),
            "custom.toml".to_string(),
            "--no-config".to_string(),
        ];

        let command = parse_cli(&args).unwrap();
        match command {
            Command::Store {
                config_path,
                no_config,
                ..
            } => {
                assert_eq!(config_path.unwrap(), "custom.toml");
                assert!(no_config);
            }
            _ => panic!("expected store command"),
        }
    }

    #[test]
    fn parse_recognize_with_override_flags() {
        let args = vec![
            "shazam".to_string(),
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
            "shazam".to_string(),
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
}
