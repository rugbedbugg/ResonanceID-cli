use crate::utils::{
    audio_to_spectrogram::audio_to_spectrogram,
    extract_peaks::extract_peaks,
    peaks_to_hashes::peaks_to_fingerprints,
    read_wav::read_wav,
};

pub type Fingerprint = (u32, u32); // (hash, anchor_time_ms)
pub type MatchResult = (String, String, f32); // (title, artist, score)

#[derive(Clone, Copy, Debug, Default)]
pub struct ClipOptions {
    pub clip_start_seconds: Option<f32>,
    pub clip_duration_seconds: Option<f32>,
    pub auto_clip: bool,
}

pub struct FingerprintReport {
    pub sample_rate: u32,
    pub sample_count: usize,
    pub frame_count: usize,
    pub peak_count: usize,
    pub fingerprint_count: usize,
    pub duration_seconds: f32,
    pub clip_start_seconds: f32,
    pub clip_duration_seconds: f32,
}

pub fn fingerprint_wav(
    wav_path: &str,
    threshold_db: f32,
    window_size: usize,
    hop_size: usize,
    anchor_window: usize,
) -> Result<Vec<Fingerprint>, Box<dyn std::error::Error>> {
    let (fingerprints, _report) = fingerprint_wav_with_report_and_clip(
        wav_path,
        threshold_db,
        window_size,
        hop_size,
        anchor_window,
        ClipOptions::default(),
    )?;

    Ok(fingerprints)
}

pub fn fingerprint_wav_with_report(
    wav_path: &str,
    threshold_db: f32,
    window_size: usize,
    hop_size: usize,
    anchor_window: usize,
) -> Result<(Vec<Fingerprint>, FingerprintReport), Box<dyn std::error::Error>> {
    fingerprint_wav_with_report_and_clip(
        wav_path,
        threshold_db,
        window_size,
        hop_size,
        anchor_window,
        ClipOptions::default(),
    )
}

pub fn fingerprint_wav_with_report_and_clip(
    wav_path: &str,
    threshold_db: f32,
    window_size: usize,
    hop_size: usize,
    anchor_window: usize,
    clip: ClipOptions,
) -> Result<(Vec<Fingerprint>, FingerprintReport), Box<dyn std::error::Error>> {
    let (samples, sample_rate) = read_wav(wav_path)?;
    let (start_idx, end_idx) = resolve_clip_range(samples.len(), sample_rate, clip);
    let clipped_samples = &samples[start_idx..end_idx];

    let spectrogram = audio_to_spectrogram(clipped_samples, sample_rate, window_size, hop_size);
    let peaks = extract_peaks(&spectrogram, threshold_db);
    let fingerprints = peaks_to_fingerprints(&peaks, anchor_window, sample_rate, hop_size);

    let report = FingerprintReport {
        sample_rate,
        sample_count: clipped_samples.len(),
        frame_count: spectrogram.len(),
        peak_count: peaks.len(),
        fingerprint_count: fingerprints.len(),
        duration_seconds: clipped_samples.len() as f32 / sample_rate as f32,
        clip_start_seconds: start_idx as f32 / sample_rate as f32,
        clip_duration_seconds: (end_idx - start_idx) as f32 / sample_rate as f32,
    };

    Ok((fingerprints, report))
}

fn resolve_clip_range(
    sample_count: usize,
    sample_rate: u32,
    clip: ClipOptions,
) -> (usize, usize) {
    if sample_count == 0 || sample_rate == 0 {
        return (0, 0);
    }

    let full_start = 0usize;
    let full_end = sample_count;

    if !clip.auto_clip && clip.clip_start_seconds.is_none() && clip.clip_duration_seconds.is_none() {
        return (full_start, full_end);
    }

    let total_duration = sample_count as f32 / sample_rate as f32;

    let mut duration = clip.clip_duration_seconds.unwrap_or(total_duration);
    if clip.auto_clip && clip.clip_duration_seconds.is_none() {
        duration = 20.0;
    }

    if duration <= 0.0 || !duration.is_finite() {
        return (full_start, full_end);
    }

    let duration = duration.min(total_duration);

    let mut start = clip.clip_start_seconds.unwrap_or(0.0);
    if clip.auto_clip {
        start = (total_duration - duration) / 2.0;
    }

    if !start.is_finite() || start < 0.0 {
        start = 0.0;
    }

    if start > total_duration {
        start = total_duration;
    }

    let end = (start + duration).min(total_duration);

    let start_idx = (start * sample_rate as f32) as usize;
    let end_idx = ((end * sample_rate as f32) as usize).max(start_idx).min(sample_count);

    (start_idx, end_idx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_clip_uses_full_range() {
        let (start, end) = resolve_clip_range(44_100, 44_100, ClipOptions::default());
        assert_eq!(start, 0);
        assert_eq!(end, 44_100);
    }

    #[test]
    fn auto_clip_uses_middle_20_seconds() {
        let opts = ClipOptions {
            auto_clip: true,
            ..Default::default()
        };
        let (start, end) = resolve_clip_range(44_100 * 60, 44_100, opts);

        assert_eq!(end - start, 44_100 * 20);
        assert_eq!(start, 44_100 * 20);
    }

    #[test]
    fn explicit_clip_start_and_duration() {
        let opts = ClipOptions {
            clip_start_seconds: Some(10.0),
            clip_duration_seconds: Some(5.0),
            auto_clip: false,
        };

        let (start, end) = resolve_clip_range(44_100 * 60, 44_100, opts);
        assert_eq!(start, 44_100 * 10);
        assert_eq!(end - start, 44_100 * 5);
    }
}
