use crate::utils::{
    audio_to_spectrogram::audio_to_spectrogram, extract_peaks::extract_peaks,
    peaks_to_hashes::peaks_to_hashes, read_wav::read_wav,
};

#[test]
fn full_pipeline_on_test_wav() {
    let (samples, sample_rate) = read_wav("songs/output.wav").unwrap();
    let spectrogram = audio_to_spectrogram(&samples, sample_rate, 1024, 512);
    let peaks = extract_peaks(&spectrogram, -20.0);
    let hashes = peaks_to_hashes(&peaks, 50);

    assert!(!samples.is_empty());
    assert!(!spectrogram.is_empty());
    assert!(!peaks.is_empty());
    assert!(!hashes.is_empty());
}
