use shazam::{db::create_db::Database, pipeline::fingerprint_wav};

const DEFAULT_DB_PATH: &str = "shazam.db";
const DEFAULT_WINDOW_SIZE: usize = 1024;
const DEFAULT_HOP_SIZE: usize = 512;
const DEFAULT_ANCHOR_WINDOW: usize = 50;
const DEFAULT_THRESHOLD_DB: f32 = -20.0;

enum Command {
    Index {
        wav_path: String,
        title: String,
        artist: String,
        db_path: String,
    },
    Recognize {
        wav_path: String,
        db_path: String,
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
        } => {
            let mut db = Database::open(&db_path)?;
            let fingerprints = fingerprint_wav(
                &wav_path,
                DEFAULT_THRESHOLD_DB,
                DEFAULT_WINDOW_SIZE,
                DEFAULT_HOP_SIZE,
                DEFAULT_ANCHOR_WINDOW,
            )?;

            db.register_song(&wav_path, &title, &artist, &fingerprints)?;
            println!(
                "✅ Indexed '{}' by '{}' ({} fingerprints)",
                title,
                artist,
                fingerprints.len()
            );
        }
        Command::Recognize { wav_path, db_path } => {
            let db = Database::open(&db_path)?;
            let fingerprints = fingerprint_wav(
                &wav_path,
                DEFAULT_THRESHOLD_DB,
                DEFAULT_WINDOW_SIZE,
                DEFAULT_HOP_SIZE,
                DEFAULT_ANCHOR_WINDOW,
            )?;

            let matches = db.recognize_song(&fingerprints)?;
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
            let db_path = parse_db_path(args, 5)?;

            Ok(Command::Index {
                wav_path,
                title,
                artist,
                db_path,
            })
        }
        "recognize" => {
            // shazam recognize <wav_path> [--db <path>]
            if args.len() < 3 {
                print_usage();
                return Err("recognize requires <wav_path>".into());
            }

            let wav_path = args[2].clone();
            let db_path = parse_db_path(args, 3)?;

            Ok(Command::Recognize { wav_path, db_path })
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

fn parse_db_path(args: &[String], offset: usize) -> Result<String, Box<dyn std::error::Error>> {
    if args.len() == offset {
        return Ok(DEFAULT_DB_PATH.to_string());
    }

    if args.len() == offset + 2 && args[offset] == "--db" {
        return Ok(args[offset + 1].clone());
    }

    Err("invalid arguments after required positional values".into())
}

fn print_usage() {
    println!("Usage:");
    println!("  shazam index <wav_path> <title> <artist> [--db <db_path>]");
    println!("  shazam recognize <wav_path> [--db <db_path>]");
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
            } => {
                assert_eq!(wav_path, "songs/output.wav");
                assert_eq!(title, "Test Song");
                assert_eq!(artist, "Test Artist");
                assert_eq!(db_path, DEFAULT_DB_PATH);
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
            Command::Recognize { wav_path, db_path } => {
                assert_eq!(wav_path, "snippet/clip.wav");
                assert_eq!(db_path, "custom.db");
            }
            _ => panic!("expected recognize command"),
        }
    }

    #[test]
    fn fail_on_missing_command() {
        let args = vec!["shazam".to_string()];
        assert!(parse_cli(&args).is_err());
    }
}
