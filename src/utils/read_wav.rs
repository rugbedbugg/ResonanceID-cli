use hound::{WavReader, WavSpec};

pub fn read_wav(path: &str) -> Result<(Vec<i16>, u32), Box<dyn std::error::Error>> {
    let mut reader = WavReader::open(path)?;
    let spec: WavSpec = reader.spec();

    // Only support 16-bit integer PCM for now
    if spec.sample_format != hound::SampleFormat::Int || spec.bits_per_sample != 16 {
        return Err("only 16-bit integer PCM WAV supported".into());
    }

    let samples: Vec<i16> = reader
                        .samples::<i16>()
                        .collect::<Result<Vec<_>, _>>()?;

    Ok((samples, spec.sample_rate))
}
