# VOX

A terminal music player with an ncmpcpp-style TUI, written in Rust.

![license](https://img.shields.io/badge/license-MIT-blue)

## Features

- **Two-panel layout** — album browser (right) + track list with metadata panel (left)
- **Keyboard-driven** — vim-style navigation, fully mouseless
- **Metadata panel** — title, artist, album, bitrate, sample rate, codec for the current track
- **Now-playing indicators** — `♪` marks the current track and album in both panels
- **Interactive file browser** — press `/` to navigate your filesystem and select folders to scan
- **Library management** — scan folders, browse by album, search across all tracks, remove tracked paths
- **Playback controls** — play/pause, next/previous, volume, mute, repeat, shuffle
- **Progress bar** — real-time elapsed/total with visual progress indicator in the status bar
- **Search** — live-filter tracks by title, artist, or album
- **Persistent config** — volume, shuffle, repeat, and music directories saved to JSON
- **Database-backed library** — fast metadata lookups via sled (embedded)
- **Format support** — FLAC, MP3, WAV, OGG Vorbis, Opus, AAC, AIFF, M4A

## Screenshot

![VOX TUI](screenshots/Pasted%20image.png)

## Installation

### From source

```bash
git clone https://github.com/CodeMonkX/VOX.git
cd VOX
cargo build --release
cp target/release/vox ~/.local/bin/
```

Or run the install script:

```bash
./install.sh
```

**Dependencies:** Rust toolchain, ALSA development libraries (`alsa-lib` on Arch, `libasound2-dev` on Debian).

## Usage

```bash
vox                    # Launch the TUI
vox scan ~/Music       # Scan a folder into the library
vox list tracks        # List all tracks (CLI)
vox search "keyword"   # Search the library (CLI)
vox --help             # Show all commands
```

### TUI Keybindings

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
| `/` | Add folder to library (scan) |
| `x` | Remove tracked path from library |
| `f` / `F` | Search tracks (title, artist, album) |
| `d` / `D` | Remove selected album from library |
| `q` / `Q` | Quit |
| `Esc` | Cancel current input mode |
| `Ctrl+C` | Force quit |

### Input Modes

- **Browse folders** (`/`) — navigate your filesystem with `↑`/`↓`, `Enter` to open a directory, `Esc` to go up / cancel. Press `s` to scan the current folder into the library.
- **Remove path** (`x`) — select from your tracked directories, press `Enter` to remove all tracks under that path from the library and config.
- **Search** (`f` / `F`) — type a query to filter tracks by title, artist, or album. `↑`/`↓` to select, `Enter` to play, `Esc` to exit.

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

`music_dirs` is managed automatically when you add or remove folders through the TUI.

## Database

The library database is stored at `~/.config/vox/library.db` (sled database). If you get a "database locked" error, delete this directory and re-scan.

## Tech Stack

- [ratatui](https://github.com/ratatui/ratatui) — TUI framework
- [crossterm](https://github.com/crossterm-rs/crossterm) — terminal backend
- [rodio](https://github.com/RustAudio/rodio) — audio playback
- [lofty](https://github.com/Serial-ATA/lofty-rs) — metadata reading and tag parsing
- [sled](https://github.com/spacejam/sled) — embedded database engine
- [clap](https://github.com/clap-rs/clap) — CLI argument parsing

## License

MIT
