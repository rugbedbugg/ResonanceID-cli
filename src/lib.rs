pub mod utils {
    pub mod audio_to_spectrogram;
    pub mod extract_peaks;
    pub mod peaks_to_hashes;
    pub mod read_wav;
}
pub mod db {
    pub mod create_db;
    pub mod recognize_song;
    pub mod register_song;
}

pub mod config;
pub mod pipeline;

#[cfg(test)]
mod tests;
