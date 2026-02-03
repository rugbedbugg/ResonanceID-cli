use shazam::utils::{
    read_wav::read_wav,
    audio_to_spectrogram::audio_to_spectrogram,
    extract_peaks::extract_peaks,
    peaks_to_hashes::peaks_to_hashes,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
/* 
   three main functions:
        1. raw audio     -- spectrogram
        2. spectrogram   -- peaks
        3. peaks         -- fingerprint hashes
*/
    let (samples, sample_rate) = read_wav("../songs/output.wav")?;
    let samples: Vec<i16> = samples;

    // Downsample if required
    // let sample_rate = 11025;
    // let samples = downsample(&samples, 44100, sample_rate); // not implemented yet

    let spectrogram = audio_to_spectrogram(&samples, sample_rate, 1024, 512);
    let peaks = extract_peaks(&spectrogram, -20.0); // -20db threshold
    let hashes = peaks_to_hashes(&peaks, 50);        // look 50 peaks

    println!("Generated {} hashes from {} samples", hashes.len(), samples.len());
    
    Ok(())
}
