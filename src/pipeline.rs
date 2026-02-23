use crate::utils::{
    audio_to_spectrogram::audio_to_spectrogram,
    extract_peaks::extract_peaks,
    peaks_to_hashes::peaks_to_fingerprints,
    read_wav::read_wav,
};

pub type Fingerprint = (u32, u32); // (hash, anchor_time_ms)
pub type MatchResult = (String, String, f32); // (title, artist, score)

pub struct FingerprintReport {
    pub sample_rate: u32,
    pub sample_count: usize,
    pub frame_count: usize,
    pub peak_count: usize,
    pub fingerprint_count: usize,
    pub duration_seconds: f32,
}

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

pub fn fingerprint_wav_with_report(
    wav_path: &str,
    threshold_db: f32,
    window_size: usize,
    hop_size: usize,
    anchor_window: usize,
) -> Result<(Vec<Fingerprint>, FingerprintReport), Box<dyn std::error::Error>> {
    let (samples, sample_rate) = read_wav(wav_path)?;
    let spectrogram = audio_to_spectrogram(&samples, sample_rate, window_size, hop_size);
    let peaks = extract_peaks(&spectrogram, threshold_db);
    let fingerprints = peaks_to_fingerprints(&peaks, anchor_window, sample_rate, hop_size);

    let report = FingerprintReport {
        sample_rate,
        sample_count: samples.len(),
        frame_count: spectrogram.len(),
        peak_count: peaks.len(),
        fingerprint_count: fingerprints.len(),
        duration_seconds: samples.len() as f32 / sample_rate as f32,
    };

    Ok((fingerprints, report))
}
