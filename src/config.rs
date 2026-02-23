use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub fingerprint: FingerprintConfig,
    pub recognition: RecognitionConfig,
}

#[derive(Debug, Clone)]
pub struct FingerprintConfig {
    pub window_size: usize,
    pub hop_size: usize,
    pub anchor_window: usize,
    pub threshold_db: f32,
}

#[derive(Debug, Clone)]
pub struct RecognitionConfig {
    pub min_match_score: u32,
    pub dynamic_gate_scale: f32,
    pub small_query_threshold: usize,
    pub max_results: usize,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            fingerprint: FingerprintConfig::default(),
            recognition: RecognitionConfig::default(),
        }
    }
}

impl Default for FingerprintConfig {
    fn default() -> Self {
        Self {
            window_size: 1024,
            hop_size: 512,
            anchor_window: 50,
            threshold_db: -20.0,
        }
    }
}

impl Default for RecognitionConfig {
    fn default() -> Self {
        Self {
            min_match_score: 2,
            dynamic_gate_scale: 30.0,
            small_query_threshold: 1000,
            max_results: 5,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
struct AppConfigPartial {
    #[serde(default)]
    fingerprint: FingerprintConfigPartial,
    #[serde(default)]
    recognition: RecognitionConfigPartial,
}

#[derive(Debug, Default, Deserialize)]
struct FingerprintConfigPartial {
    window_size: Option<usize>,
    hop_size: Option<usize>,
    anchor_window: Option<usize>,
    threshold_db: Option<f32>,
}

#[derive(Debug, Default, Deserialize)]
struct RecognitionConfigPartial {
    min_match_score: Option<u32>,
    dynamic_gate_scale: Option<f32>,
    small_query_threshold: Option<usize>,
    max_results: Option<usize>,
}

impl AppConfig {
    pub fn load(config_path: Option<&str>, no_config: bool) -> Result<Self, Box<dyn std::error::Error>> {
        let mut config = AppConfig::default();

        if no_config {
            return Ok(config);
        }

        let mut paths = Vec::new();
        if let Some(path) = config_path {
            paths.push(path.to_string());
        } else {
            paths.extend(default_config_paths());
        }

        for path in paths {
            if !std::path::Path::new(&path).exists() {
                continue;
            }

            let raw = std::fs::read_to_string(&path)?;
            let partial: AppConfigPartial = toml::from_str(&raw)?;
            apply_partial(&mut config, partial);
        }

        Ok(config)
    }
}

fn apply_partial(config: &mut AppConfig, partial: AppConfigPartial) {
    if let Some(v) = partial.fingerprint.window_size {
        config.fingerprint.window_size = v;
    }
    if let Some(v) = partial.fingerprint.hop_size {
        config.fingerprint.hop_size = v;
    }
    if let Some(v) = partial.fingerprint.anchor_window {
        config.fingerprint.anchor_window = v;
    }
    if let Some(v) = partial.fingerprint.threshold_db {
        config.fingerprint.threshold_db = v;
    }

    if let Some(v) = partial.recognition.min_match_score {
        config.recognition.min_match_score = v;
    }
    if let Some(v) = partial.recognition.dynamic_gate_scale {
        config.recognition.dynamic_gate_scale = v;
    }
    if let Some(v) = partial.recognition.small_query_threshold {
        config.recognition.small_query_threshold = v;
    }
    if let Some(v) = partial.recognition.max_results {
        config.recognition.max_results = v;
    }
}

fn default_config_paths() -> Vec<String> {
    let mut paths = Vec::new();

    paths.push("/etc/shazam/config.toml".to_string());

    if let Ok(home) = std::env::var("HOME") {
        paths.push(format!("{}/.config/shazam/config.toml", home));
    }

    paths.push("./shazam.toml".to_string());

    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_loaded_without_config() {
        let cfg = AppConfig::load(None, true).unwrap();
        assert_eq!(cfg.fingerprint.window_size, 1024);
        assert_eq!(cfg.recognition.min_match_score, 2);
    }
}
