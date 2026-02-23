use resonanceid_cli::utils::{
    read_wav::read_wav,
    audio_to_spectrogram::audio_to_spectrogram,
    extract_peaks::extract_peaks,
    peaks_to_hashes::peaks_to_hashes,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (samples, sample_rate) = read_wav("../songs/output.wav")?;
    let samples: Vec<i16> = samples;
    println!("Loaded {} samples at {} Hz", samples.len(), sample_rate);

    let spectrogram = audio_to_spectrogram(&samples, sample_rate, 1024, 512);
    println!("Computed {} time frames", spectrogram.len());

    let peaks = extract_peaks(&spectrogram, -20.0);
    println!("Found {} peaks", peaks.len());

    let hashes = peaks_to_hashes(&peaks, 50);
    println!("Generated {} hashes", hashes.len());

    Ok(())
}

