# VOX

A terminal music player with an ncmpcpp-style TUI, written in Rust.

![screenshot](https://img.shields.io/badge/license-MIT-blue)

## Features

- **Two-panel layout** — album browser (right) + track list with metadata panel (left)
- **Keyboard-driven** — vim-style navigation, no mouse needed
- **Metadata display** — title, artist, album, bitrate, sample rate, codec
- **Library management** — scan folders, browse by album, search across all tracks
- **Playback controls** — play/pause, next/previous, volume, mute, repeat, shuffle
- **Progress bar** — real-time elapsed/total with visual progress indicator
- **Persistent config** — volume, shuffle, repeat saved to `~/.config/vox/config.json`
- **Database-backed library** — fast metadata lookups via sled
- **Format support** — FLAC, MP3, WAV, OGG Vorbis, Opus, AAC, AIFF, M4A

## Installation

### From source

```bash
git clone https://github.com/codemonkx/VOX.git
cd VOX
cargo build --release
cp target/release/vox ~/.local/bin/
```

Or run the install script:

```bash
./install.sh
```

**Dependencies:** Rust toolchain, ALSA dev libraries (`alsa-lib` on Arch, `libasound2-dev` on Debian).

### Arch Linux

Install via the install script or `cargo install --path .`. An AUR package may be available in the future.

## Usage

```bash
vox                    # Launch the TUI
vox scan ~/Music       # Scan a folder into the library
vox --help             # Show all commands
```

### Keybindings

| Key | Action |
|------|--------|
| `↑` / `↓` | Navigate albums / tracks |
| `←` / `→` or `Tab` | Switch between album and track panel |
| `Enter` | Select album / play track |
| `Space` / `k` | Play / pause |
| `n` / `N` | Next track |
| `b` / `B` | Previous track |
| `+` / `=` | Volume up |
| `-` / `_` | Volume down |
| `m` / `M` | Mute / unmute |
| `r` / `R` | Toggle repeat |
| `s` / `S` | Toggle shuffle |
| `/` | Add folder to library |
| `f` / `F` | Search tracks (title, artist, album) |
| `d` / `D` | Remove selected album from library |
| `q` / `Q` | Quit |
| `Ctrl+C` | Force quit |

## Configuration

Config file: `~/.config/vox/config.json`

```json
{
  "music_dirs": ["/home/user/Music"],
  "volume": 0.8,
  "shuffle": false,
  "repeat": false
}
```

## Database

The library database is stored at `~/.config/vox/library.db` (sled database). If you get a "database locked" error, delete this directory and re-scan.

## Tech Stack

- [ratatui](https://github.com/ratatui/ratatui) — TUI framework
- [crossterm](https://github.com/crossterm-rs/crossterm) — terminal backend
- [rodio](https://github.com/RustAudio/rodio) — audio playback
- [lofty](https://github.com/Serial-ATA/lofty-rs) — metadata reading
- [sled](https://github.com/spacejam/sled) — embedded database
- [clap](https://github.com/clap-rs/clap) — CLI argument parsing

## License

MIT
