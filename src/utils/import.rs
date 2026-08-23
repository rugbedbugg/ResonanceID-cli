use std::path::{Path, PathBuf};
use std::process::Command;

pub const AUDIO_EXTENSIONS: [&str; 8] = ["wav", "mp3", "flac", "m4a", "ogg", "opus", "wma", "aac"];

/// Lists supported audio files directly inside `folder` (non-recursive),
/// sorted by path for deterministic processing order.
pub fn list_audio_files(folder: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut files = Vec::new();

    for entry in std::fs::read_dir(folder)? {
        let path = entry?.path();
        if !path.is_file() {
            continue;
        }

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        if AUDIO_EXTENSIONS.contains(&ext.as_str()) {
            files.push(path);
        }
    }

    files.sort();
    Ok(files)
}

/// Derives (title, artist) from a filename. Files named with the
/// "Artist - Title" convention are split on the first separator;
/// otherwise the whole stem becomes the title with an unknown artist.
pub fn derive_title_artist(file: &Path) -> (String, String) {
    let stem = file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled")
        .trim()
        .to_string();

    match stem.split_once(" - ") {
        Some((artist, title)) if !artist.trim().is_empty() && !title.trim().is_empty() => {
            (title.trim().to_string(), artist.trim().to_string())
        }
        _ => (stem, "Unknown Artist".to_string()),
    }
}

pub struct PreparedWav {
    /// WAV ready to be fingerprinted.
    pub wav_path: PathBuf,
    /// True when this file lives in the throwaway folder and was converted
    /// from another format. Originals are never touched.
    pub is_temporary: bool,
}

/// Returns a fingerprintable WAV for `file`. Native 16-bit PCM mono WAVs are
/// used as-is; anything else is converted via ffmpeg into `temp_dir`.
pub fn prepare_wav(
    file: &Path,
    temp_dir: &Path,
) -> Result<PreparedWav, Box<dyn std::error::Error>> {
    let ext = file
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    if ext == "wav" {
        // Try native use first; malformed WAVs fall through to ffmpeg,
        // which can often still decode them.
        let direct = PreparedWav { wav_path: file.to_path_buf(), is_temporary: false };
        if crate::utils::read_wav::read_wav(&file.to_string_lossy()).is_ok() {
            return Ok(direct);
        }
    }

    let stem = file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("track")
        .to_string();

    let dest = temp_dir.join(format!("{stem}.wav"));

    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-hide_banner",
            "-loglevel",
            "error",
            "-i",
        ])
        .arg(file)
        .args(["-ac", "1", "-ar", "44100", "-sample_fmt", "s16"])
        .arg(&dest)
        .status()
        .map_err(|e| format!("failed to launch ffmpeg (is it installed?): {e}"))?;

    if !status.success() || !dest.exists() {
        return Err(format!(
            "ffmpeg could not convert '{}'",
            file.to_string_lossy()
        )
        .into());
    }

    Ok(PreparedWav { wav_path: dest, is_temporary: true })
}

/// Removes the throwaway conversion folder, ignoring missing dirs.
pub fn cleanup_temp_dir(temp_dir: &Path) {
    let _ = std::fs::remove_dir_all(temp_dir);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_title_artist_from_convention() {
        let f = Path::new("C:\\music\\KITSCHKRIEG - Du Bist Gut Genug.mp3");
        let (title, artist) = derive_title_artist(f);
        assert_eq!(title, "Du Bist Gut Genug");
        assert_eq!(artist, "KITSCHKRIEG");
    }

    #[test]
    fn falls_back_to_stem_and_unknown_artist() {
        let f = Path::new("song_without_artist.flac");
        let (title, artist) = derive_title_artist(f);
        assert_eq!(title, "song_without_artist");
        assert_eq!(artist, "Unknown Artist");
    }

    #[test]
    fn keeps_hyphenated_titles_intact_when_one_side_missing() {
        let f = Path::new("- Just a Title.wav");
        let (title, artist) = derive_title_artist(f);
        assert_eq!(artist, "Unknown Artist");
        assert!(!title.is_empty());
    }

    #[test]
    fn lists_only_supported_extensions() {
        let dir = std::env::temp_dir().join(format!("rid_import_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.mp3"), b"x").unwrap();
        std::fs::write(dir.join("b.WAV"), b"x").unwrap();
        std::fs::write(dir.join("c.txt"), b"x").unwrap();
        std::fs::write(dir.join("d.ogg"), b"x").unwrap();

        let files = list_audio_files(&dir).unwrap();
        let names: Vec<String> = files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();

        assert_eq!(names, vec!["a.mp3", "b.WAV", "d.ogg"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn errors_on_missing_folder() {
        assert!(list_audio_files(Path::new("Z:/definitely/not/here")).is_err());
    }
}
