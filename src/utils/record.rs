use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};

pub const TARGET_SAMPLE_RATE: u32 = 44100;

/// Post-capture loudness target (~-20 dBFS). Normalizing to a fixed RMS makes
/// matching independent of mic gain, distance from speakers and AGC behavior.
const TARGET_RMS: f32 = 0.1;
const MAX_NORMALIZATION_GAIN: f32 = 64.0;

/// Records `duration_seconds` from the default input device.
/// Returns mono 16-bit PCM samples resampled to 44.1 kHz so recordings
/// are directly comparable with reference tracks indexed at that rate.
pub fn record_mic_samples(duration_seconds: f32) -> Result<(Vec<i16>, u32), Box<dyn std::error::Error>> {
    if duration_seconds <= 0.0 || !duration_seconds.is_finite() {
        return Err("recording duration must be a positive number of seconds".into());
    }

    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or("no microphone/input device found")?;
    let config = device
        .default_input_config()
        .map_err(|e| format!("failed to query input device config: {e}"))?;

    let channels = config.channels() as usize;
    let device_rate = config.sample_rate();
    let sample_format = config.sample_format();
    let stream_config: cpal::StreamConfig = config.into();

    let collected: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let err_fn = |err: cpal::Error| {
        // WASAPI shared mode reports transient underruns/overruns at stream
        // startup; they don't affect captured audio, so don't alarm the user.
        let msg = err.to_string();
        let lower = msg.to_lowercase();
        if !lower.contains("underrun") && !lower.contains("overrun") {
            eprintln!("audio stream error: {msg}");
        }
    };

    macro_rules! build_stream {
        ($ty:ty, $to_f32:expr) => {{
            let collected = Arc::clone(&collected);
            device.build_input_stream(
                stream_config,
                move |data: &[$ty], _: &cpal::InputCallbackInfo| {
                    if let Ok(mut buf) = collected.lock() {
                        buf.extend(data.iter().map(|s| ($to_f32)(*s)));
                    }
                },
                err_fn,
                None,
            )?
        }};
    }

    let stream = match sample_format {
        cpal::SampleFormat::F32 => build_stream!(f32, |s: f32| s),
        cpal::SampleFormat::I16 => build_stream!(i16, |s: i16| s as f32 / i16::MAX as f32),
        cpal::SampleFormat::U16 => build_stream!(u16, |s: u16| (s as f32 - 32768.0) / 32768.0),
        other => return Err(format!("unsupported sample format: {other:?}").into()),
    };

    stream.play().map_err(|e| format!("failed to start recording: {e}"))?;
    println!("🎤 Recording {:.0}s...", duration_seconds);
    std::thread::sleep(std::time::Duration::from_secs_f32(duration_seconds));
    drop(stream);

    let samples_f32 = Arc::try_unwrap(collected)
        .map_err(|_| "failed to collect recorded audio")?
        .into_inner()?;

    // Downmix interleaved multi-channel capture to mono
    let mono = downmix_to_mono(&samples_f32, channels);

    // Normalize device rate to the pipeline's target rate
    let resampled = if device_rate == TARGET_SAMPLE_RATE {
        mono
    } else {
        linear_resample(&mono, device_rate, TARGET_SAMPLE_RATE)
    };

    let normalized = normalize_rms(&resampled, TARGET_RMS, MAX_NORMALIZATION_GAIN);

    let pcm: Vec<i16> = normalized
        .iter()
        .map(|&s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
        .collect();

    Ok((pcm, TARGET_SAMPLE_RATE))
}

/// Scales samples so their RMS matches `target_rms`, clamped to [-1, 1].
/// Gain is capped at `max_gain` so near-silence doesn't get amplified into
/// pure noise. Silent input is returned untouched.
fn normalize_rms(samples: &[f32], target_rms: f32, max_gain: f32) -> Vec<f32> {
    if samples.is_empty() || target_rms <= 0.0 {
        return samples.to_vec();
    }

    let mean_sq = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    if mean_sq <= 0.0 {
        return samples.to_vec();
    }

    let gain = (target_rms / mean_sq.sqrt()).min(max_gain);
    samples.iter().map(|&s| (s * gain).clamp(-1.0, 1.0)).collect()
}

/// Averages interleaved channel frames into a single mono track.
fn downmix_to_mono(samples: &[f32], channels: usize) -> Vec<f32> {
    if channels <= 1 || samples.is_empty() {
        return samples.to_vec();
    }
    samples.chunks_exact(channels).map(|frame| frame.iter().sum::<f32>() / channels as f32).collect()
}

/// Naive linear interpolation between source and target sample rates.
fn linear_resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate || samples.len() < 2 {
        return samples.to_vec();
    }

    let ratio = from_rate as f64 / to_rate as f64;
    let out_len = ((samples.len() as f64) / ratio).floor() as usize;
    let mut out = Vec::with_capacity(out_len);

    for i in 0..out_len {
        let src = i as f64 * ratio;
        let idx = src as usize;
        let frac = (src - idx as f64) as f32;
        let a = samples[idx];
        let b = samples[(idx + 1).min(samples.len() - 1)];
        out.push(a + (b - a) * frac);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downmix_single_channel_is_identity() {
        let samples = vec![0.5, -0.25, 0.75];
        assert_eq!(downmix_to_mono(&samples, 1), samples);
    }

    #[test]
    fn downmix_stereo_averages_frames() {
        let stereo = vec![1.0, 0.0, -1.0, 1.0];
        let mono = downmix_to_mono(&stereo, 2);
        assert_eq!(mono, vec![0.5, 0.0]);
    }

    #[test]
    fn resample_same_rate_is_identity() {
        let samples = vec![0.1, 0.2, 0.3];
        assert_eq!(linear_resample(&samples, 44100, 44100), samples);
    }

    #[test]
    fn resample_upsample_doubles_length() {
        let samples = vec![0.0, 0.5, 1.0];
        let up = linear_resample(&samples, 22050, 44100);
        assert_eq!(up.len(), 6);
        assert!((up[1] - 0.25).abs() < 1e-6);
    }

    #[test]
    fn resample_downsample_halves_length() {
        let samples = vec![0.0, 0.25, 0.5, 0.75, 1.0, 1.0];
        let down = linear_resample(&samples, 88200, 44100);
        assert_eq!(down.len(), 3);
        assert!((down[1] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn normalize_quiet_signal_up_to_target() {
        // constant ±0.01 signal has rms 0.01 -> gain 10x to reach 0.1
        let samples: Vec<f32> = (0..1000).map(|i| if i % 2 == 0 { 0.01 } else { -0.01 }).collect();
        let out = normalize_rms(&samples, 0.1, 64.0);
        let rms = (out.iter().map(|s| s * s).sum::<f32>() / out.len() as f32).sqrt();
        assert!((rms - 0.1).abs() < 1e-4);
    }

    #[test]
    fn normalize_loud_signal_down_to_target() {
        let samples = vec![1.0; 1000];
        let out = normalize_rms(&samples, 0.1, 64.0);
        assert!(out.iter().all(|&s| s <= 1.0));
        let rms = (out.iter().map(|s| s * s).sum::<f32>() / out.len() as f32).sqrt();
        assert!((rms - 0.1).abs() < 1e-2);
    }

    #[test]
    fn normalize_silence_untouched_and_gain_capped() {
        assert!(normalize_rms(&[], 0.1, 64.0).is_empty());
        assert!(normalize_rms(&[0.0; 100], 0.1, 64.0).iter().all(|&s| s == 0.0));

        // near-silence must not exceed the max gain
        let quiet = vec![1e-6; 100];
        let out = normalize_rms(&quiet, 0.1, 64.0);
        assert!(out.iter().all(|&s| s <= 1e-6 * 64.0 + 1e-9));
    }
}
