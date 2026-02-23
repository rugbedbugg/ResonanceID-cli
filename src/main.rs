use shazam::{
    config::AppConfig,
    db::create_db::Database,
    pipeline::{fingerprint_wav, fingerprint_wav_with_report},
};

const DEFAULT_DB_PATH: &str = "shazam.db";
const MIN_RECOMMENDED_INDEX_DURATION_SECONDS: f32 = 15.0;

enum Command {
    Index {
        wav_path: String,
        title: String,
        artist: String,
        db_path: String,
        config_path: Option<String>,
        no_config: bool,
    },
    Recognize {
        wav_path: String,
        db_path: String,
        config_path: Option<String>,
        no_config: bool,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let command = parse_cli(&args)?;

    match command {
        Command::Index {
            wav_path,
            title,
            artist,
            db_path,
            config_path,
            no_config,
        } => {
            let run_start = std::time::Instant::now();
            let cfg = AppConfig::load(config_path.as_deref(), no_config)?;
            let mut db = Database::open(&db_path)?;
            let (fingerprints, report) = fingerprint_wav_with_report(
                &wav_path,
                cfg.fingerprint.threshold_db,
                cfg.fingerprint.window_size,
                cfg.fingerprint.hop_size,
                cfg.fingerprint.anchor_window,
            )?;

            db.register_song(&wav_path, &title, &artist, &fingerprints)?;

            println!("✅ Indexed '{}' by '{}'", title, artist);
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
                println!("   Tip: use 'recognize' for snippets and 'index' for reference tracks.");
            }
        }
        Command::Recognize {
            wav_path,
            db_path,
            config_path,
            no_config,
        } => {
            let cfg = AppConfig::load(config_path.as_deref(), no_config)?;
            let db = Database::open(&db_path)?;
            let fingerprints = fingerprint_wav(
                &wav_path,
                cfg.fingerprint.threshold_db,
                cfg.fingerprint.window_size,
                cfg.fingerprint.hop_size,
                cfg.fingerprint.anchor_window,
            )?;

            let matches = db.recognize_song_with_config(&fingerprints, &cfg.recognition)?;
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
    }

    Ok(())
}

fn parse_cli(args: &[String]) -> Result<Command, Box<dyn std::error::Error>> {
    if args.len() < 2 {
        print_usage();
        return Err("missing command".into());
    }

    match args[1].as_str() {
        "index" => {
            // shazam index <wav_path> <title> <artist> [--db <path>]
            if args.len() < 5 {
                print_usage();
                return Err("index requires <wav_path> <title> <artist>".into());
            }

            let wav_path = args[2].clone();
            let title = args[3].clone();
            let artist = args[4].clone();
            let (db_path, config_path, no_config) = parse_common_options(args, 5)?;

            Ok(Command::Index {
                wav_path,
                title,
                artist,
                db_path,
                config_path,
                no_config,
            })
        }
        "recognize" => {
            // shazam recognize <wav_path> [--db <path>]
            if args.len() < 3 {
                print_usage();
                return Err("recognize requires <wav_path>".into());
            }

            let wav_path = args[2].clone();
            let (db_path, config_path, no_config) = parse_common_options(args, 3)?;

            Ok(Command::Recognize {
                wav_path,
                db_path,
                config_path,
                no_config,
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
) -> Result<(String, Option<String>, bool), Box<dyn std::error::Error>> {
    let mut db_path = DEFAULT_DB_PATH.to_string();
    let mut config_path: Option<String> = None;
    let mut no_config = false;

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
            _ => {
                return Err("invalid arguments after required positional values".into());
            }
        }
    }

    Ok((db_path, config_path, no_config))
}

fn should_warn_short_index(duration_seconds: f32) -> bool {
    duration_seconds < MIN_RECOMMENDED_INDEX_DURATION_SECONDS
}

fn print_usage() {
    println!("Usage:");
    println!("  shazam index <wav_path> <title> <artist> [--db <db_path>] [--config <path>] [--no-config]");
    println!("  shazam recognize <wav_path> [--db <db_path>] [--config <path>] [--no-config]");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_index_command() {
        let args = vec![
            "shazam".to_string(),
            "index".to_string(),
            "songs/output.wav".to_string(),
            "Test Song".to_string(),
            "Test Artist".to_string(),
        ];

        let command = parse_cli(&args).unwrap();
        match command {
            Command::Index {
                wav_path,
                title,
                artist,
                db_path,
                config_path,
                no_config,
            } => {
                assert_eq!(wav_path, "songs/output.wav");
                assert_eq!(title, "Test Song");
                assert_eq!(artist, "Test Artist");
                assert_eq!(db_path, DEFAULT_DB_PATH);
                assert!(config_path.is_none());
                assert!(!no_config);
            }
            _ => panic!("expected index command"),
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
    fn parse_index_command_with_config_flags() {
        let args = vec![
            "shazam".to_string(),
            "index".to_string(),
            "songs/output.wav".to_string(),
            "Test Song".to_string(),
            "Test Artist".to_string(),
            "--config".to_string(),
            "custom.toml".to_string(),
            "--no-config".to_string(),
        ];

        let command = parse_cli(&args).unwrap();
        match command {
            Command::Index {
                config_path,
                no_config,
                ..
            } => {
                assert_eq!(config_path.unwrap(), "custom.toml");
                assert!(no_config);
            }
            _ => panic!("expected index command"),
        }
    }
}
