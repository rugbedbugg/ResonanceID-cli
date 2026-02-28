# ResonanceID-cli
![GitHub last commit](https://img.shields.io/github/last-commit/rugbedbugg/ResonanceID-cli?style=for-the-badge&labelColor=000000&color=9ccbfb)
![GitHub repo size](https://img.shields.io/github/repo-size/rugbedbugg/ResonanceID-cli?style=for-the-badge&labelColor=000000&color=d3bfe6)
![Stars](https://img.shields.io/github/stars/rugbedbugg/ResonanceID-cli?style=for-the-badge&labelColor=000000&color=9ccbfe6)

A Rust-based audio fingerprinting CLI inspired by Shazam-style matching.

This project is being built for a **Design and Analysis of Algorithms** course, with focus on:

- fingerprint pipeline design
- matching quality vs false positives
- practical CLI workflows
- measurable runtime behavior

---

## Features

- Store songs into a local SQLite fingerprint DB
- Recognize unknown clips against stored references
- Show ranked candidates (top matches)
- Manage DB from CLI (`list-songs`, `remove-song`, `db-stats`)
- Config layering (`/etc`, user config, local config)
- CLI overrides for all key tuning params
- Optional clipping for reference indexing (`--clip-start`, `--clip-duration`, `--auto-clip`)

---

## Tech Stack

- **Rust**
- **SQLite** (`rusqlite`)
- **FFT** (`rustfft`)
- **WAV I/O** (`hound`)
- **TOML config** (`serde`, `toml`)

---

## Pipeline (High-Level)

### Store / Remember

1. Read WAV samples
2. (Optional) clip audio range
3. STFT spectrogram
4. Peak extraction (constellation points)
5. Fingerprint generation `(hash, anchor_time_ms)`
6. Insert song metadata + fingerprints into SQLite

### Recognize

1. Read WAV samples
2. STFT spectrogram
3. Peak extraction
4. Fingerprint generation
5. Hash lookup in DB + offset voting
6. Rank songs by strongest offset consistency

---

## Installation / Run

```bash
cargo build
cargo run -- --help
```

> Note: pass app args after `--` when using `cargo run`.  

Diagnose issues using

```bash
cargo test
```

---

## CLI Commands

### Store a reference track

```bash
cargo run -- store <wav_path> "<Title>" "<Artist>" [options]
```

Alias:

```bash
cargo run -- remember <wav_path> "<Title>" "<Artist>" [options]
```

### Recognize a clip

```bash
cargo run -- recognize <wav_path> [options]
```

### Show ranked candidates

```bash
cargo run -- list-top-matches <wav_path> [options]
```

### Database management

```bash
cargo run -- list-songs [--db <db_path>]
cargo run -- remove-song <song_id> [--db <db_path>]
cargo run -- db-stats [--db <db_path>]
```

---

## Common Options

- `--db <db_path>`
- `--config <path>`
- `--no-config`

Fingerprint options:

- `--window-size <n>`
- `--hop-size <n>`
- `--anchor-window <n>`
- `--threshold-db <f32>`

Recognition options:

- `--min-match-score <n>`
- `--dynamic-gate-scale <f32>`
- `--small-query-threshold <n>`
- `--max-results <n>`

Clip options (store/remember):

- `--clip-start <seconds>`
- `--clip-duration <seconds>`
- `--auto-clip` (center clip; default 20s if duration not specified)

---

## Config

Search order (when `--config` is not given):

1. `/etc/resonanceid-cli/config.toml`
2. `~/.config/resonanceid-cli/config.toml`
3. `./resonanceid-cli.toml`

Precedence:

**CLI flags > config file > defaults**

Example config:

```toml
[fingerprint]
window_size = 1024
hop_size = 512
anchor_window = 50
threshold_db = -20.0

[recognition]
min_match_score = 2
dynamic_gate_scale = 30.0
small_query_threshold = 1000
max_results = 5
```

You can copy from `resonanceid-cli.toml.example`.

## Quick Demo

```bash
# 1) Convert audio to WAV (mono, 44.1k)
ffmpeg -y -i input.mp3 -ac 1 -ar 44100 input.wav

# 2) Store reference
cargo run -- store input.wav "My Song" "My Artist"

# 3) Recognize clip
cargo run -- recognize clip.wav
```

---

## Notes

- Current pipeline expects **WAV** input files.
- Use ffmpeg for mp3/flac conversion before running commands.
- For stable matching quality, reference clips around **20–45 seconds** are recommended.
