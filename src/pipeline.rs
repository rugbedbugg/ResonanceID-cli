use crate::utils::{
    audio_to_spectrogram::audio_to_spectrogram,
    extract_peaks::extract_peaks,
    peaks_to_hashes::peaks_to_fingerprints,
    read_wav::read_wav,
};

pub type Fingerprint = (u32, u32); // (hash, anchor_time_ms)
pub type MatchResult = (String, String, f32); // (title, artist, score)

pub fn fingerprint_wav(
    wav_path: &str,
    threshold_db: f32,
    window_size: usize,
    hop_size: usize,
    anchor_window: usize,
) -> Result<Vec<Fingerprint>, Box<dyn std::error::Error>> {
    let (samples, sample_rate) = read_wav(wav_path)?;
    let spectrogram = audio_to_spectrogram(&samples, sample_rate, window_size, hop_size);
    let peaks = extract_peaks(&spectrogram, threshold_db);
    let fingerprints = peaks_to_fingerprints(&peaks, anchor_window, sample_rate, hop_size);

    Ok(fingerprints)
}
