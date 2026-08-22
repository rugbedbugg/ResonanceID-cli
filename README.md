# ResonanceID-cli

![GitHub last commit](https://img.shields.io/github/last-commit/rugbedbugg/ResonanceID-cli?style=for-the-badge&labelColor=000000)
![GitHub repo size](https://img.shields.io/github/repo-size/rugbedbugg/ResonanceID-cli?style=for-the-badge&labelColor=000000)
![Stars](https://img.shields.io/github/stars/rugbedbugg/ResonanceID-cli?style=for-the-badge&labelColor=000000)
![AUR version](https://img.shields.io/aur/version/resonanceid-cli?style=for-the-badge&labelColor=000000)
![License](https://img.shields.io/github/license/rugbedbugg/ResonanceID-cli?style=for-the-badge&labelColor=000000)

A Rust-based audio fingerprinting CLI inspired by Shazam-style matching. It stores a reference track as a set of hashed spectral fingerprints, then identifies unknown clips by voting on the timing offset where the most fingerprints agree.

This project is being built for a **Design and Analysis of Algorithms** course, with focus on:

- fingerprint pipeline design
- matching quality vs false positives
- practical CLI workflows
- measurable runtime behavior

---

## Features

- Store reference songs into a local SQLite fingerprint database
- Recognize an unknown clip against everything stored
- Show ranked candidates for a clip (`list-top-matches`)
- Manage the database from the CLI (`list-songs`, `remove-song`, `db-stats`)
- Layered TOML config (`/etc`, user config, local config), with CLI flags overriding all of it
- Optional clipping for reference indexing (`--clip-start`, `--clip-duration`, `--auto-clip`)
- Cross-platform: Linux and Windows (PowerShell helper scripts included)

---

## Tech Stack

- **Rust** (edition 2024, needs rustc 1.85+)
- **SQLite** via `rusqlite` (bundled, no system SQLite library required)
- **FFT** via `rustfft`
- **WAV I/O** via `hound`
- **TOML config** via `serde` + `toml`

---

## Pipeline

### Store / Remember

1. Read WAV samples
2. Optionally clip to a sub-range of the track
3. STFT spectrogram
4. Peak extraction (constellation points)
5. Pair each peak with nearby peaks to generate `(hash, anchor_time_ms)` fingerprints
6. Insert song metadata + fingerprints into SQLite

### Recognize

1. Read WAV samples
2. STFT spectrogram
3. Peak extraction
4. Fingerprint generation (same hashing as above)
5. Look up each hash in the database and record the time offset between query and match
6. Rank candidate songs by whichever offset gets the most votes: a real match produces one dominant, consistent offset

---

## Install

### Arch Linux (AUR)

```bash
paru -S resonanceid-cli
# or
yay -S resonanceid-cli
```

### From source

Requires Rust 1.85+ (edition 2024). Works on Linux, macOS and Windows — SQLite is compiled in via rusqlite's `bundled` feature, so no system SQLite development library is needed.

```bash
git clone https://github.com/rugbedbugg/ResonanceID-cli.git
cd ResonanceID-cli
cargo build --release
```

The binary is `target/release/resonanceid-cli` (`resonanceid-cli.exe` on Windows). During development, run it via `cargo run --`:

```bash
cargo run -- --help
```

> Args after `--` go to the program, not to cargo.

---

## CLI Commands

### Store a reference track

```bash
resonanceid-cli store <wav_path> "<Title>" "<Artist>" [options]
```

Aliases: `remember`, `index`.

### Recognize a clip

```bash
resonanceid-cli recognize <wav_path> [options]
```

Prints the best match (if any), then every candidate ranked by score.

### Show ranked candidates

```bash
resonanceid-cli list-top-matches <wav_path> [options]
```

Same options as `recognize`, just prints the ranked list without singling out a "best" match.

### Database management

```bash
resonanceid-cli list-songs [--db <db_path>]
resonanceid-cli remove-song <song_id> [--db <db_path>]
resonanceid-cli db-stats [--db <db_path>]
```

Every command also accepts `--help` for its own usage summary.

---

## Options

### Common (all commands)

| Flag | Description |
|---|---|
| `--db <path>` | SQLite database file. Default: `resonanceid-cli.db` in the working directory. |
| `--config <path>` | Load config from this exact file instead of the default search paths. |
| `--no-config` | Skip config files entirely and use built-in defaults. |

### Fingerprint (store, recognize, list-top-matches)

| Flag | Default | Description |
|---|---|---|
| `--window-size <n>` | `1024` | STFT window size, in samples. |
| `--hop-size <n>` | `512` | STFT hop size, in samples. |
| `--anchor-window <n>` | `5` | How many following peaks each anchor peak pairs with. |
| `--threshold-db <f32>` | `-20.0` | Minimum peak magnitude to keep. |

### Recognition (recognize, list-top-matches)

| Flag | Default | Description |
|---|---|---|
| `--min-match-score <n>` | `2` | Minimum offset-vote count for a candidate to count as a match. |
| `--dynamic-gate-scale <f32>` | `0.3` | Scales the match-score gate relative to query size, so short clips aren't held to the same bar as long ones. |
| `--small-query-threshold <n>` | `1000` | Fingerprint count below which a query is treated as "small" for gating purposes. |
| `--max-results <n>` | `5` | How many ranked candidates to return. |

### Clipping (store, remember, index)

| Flag | Description |
|---|---|
| `--clip-start <seconds>` | Start offset into the source file. |
| `--clip-duration <seconds>` | Length of the clip to index. |
| `--auto-clip` | Center a clip in the middle of the track instead of indexing the whole thing (20s by default, or `--clip-duration` if given). Overrides `--clip-start`. |

Indexing less than 15 seconds of audio prints a warning, matching quality gets unreliable below that.

---

## Config

Without `--config`, these paths are checked in order and merged: later files override fields set by earlier ones.

Linux/macOS:

1. `/etc/resonanceid-cli/config.toml`
2. `$HOME/.config/resonanceid-cli/config.toml`
3. `./resonanceid-cli.toml`

Windows:

1. `%APPDATA%\resonanceid-cli\config.toml`
2. `.\resonanceid-cli.toml`

Precedence overall: **CLI flags > config file(s) > built-in defaults**.

Copy `resonanceid-cli.toml.example` to get started:

```toml
[fingerprint]
window_size = 1024
hop_size = 512
anchor_window = 5
threshold_db = -20.0

[recognition]
min_match_score = 2
dynamic_gate_scale = 0.3
small_query_threshold = 1000
max_results = 5
```

---

## Quick Demo

bash:

```bash
# 1) Convert to WAV: mono, 44.1kHz, 16-bit PCM
ffmpeg -y -i input.mp3 -ac 1 -ar 44100 -sample_fmt s16 input.wav

# 2) Store a reference track
resonanceid-cli store input.wav "My Song" "My Artist"

# 3) Recognize a clip against it
resonanceid-cli recognize clip.wav
```

PowerShell:

```powershell
# 1) Convert to WAV: mono, 44.1kHz, 16-bit PCM
ffmpeg -y -i input.mp3 -ac 1 -ar 44100 -sample_fmt s16 input.wav

# 2) Store a reference track
.\target\release\resonanceid-cli.exe store input.wav "My Song" "My Artist"

# 3) Recognize a clip against it
.\target\release\resonanceid-cli.exe recognize clip.wav
```

Or use the bundled helper scripts (`scripts/`):

```powershell
# Convert any audio file to the required WAV format
.\scripts\Convert-ToWav.ps1 -InputFile input.mp3

# Convert + store + recognize in one go
.\scripts\Invoke-ResonanceDemo.ps1 -Reference input.mp3 -Clip clip.wav
```

---

## Notes

- Input must be **16-bit integer PCM WAV**. Anything else (float WAV, 24-bit, compressed formats) is rejected outright, convert with `ffmpeg` first.
- WAV samples are read as a flat, single-channel stream. A stereo file will produce garbage fingerprints unless you downmix to mono first (`-ac 1` above).
- For stable matching, reference clips of roughly 20–45 seconds work best; the tool will warn if you index under 15 seconds.

---

## Testing

```bash
cargo test
```

Covers CLI argument parsing, config loading/layering, clip-range resolution, hashing, and DB integration.

---

## License

MIT, see [LICENSE](LICENSE).
