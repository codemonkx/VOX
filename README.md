# VOX

> A terminal music player — fast, keyboard-driven, and built with Rust

![MIT License](https://img.shields.io/badge/license-MIT-blue)
![Rust](https://img.shields.io/badge/built_with-Rust-orange)
![Platform](https://img.shields.io/badge/platform-Linux-lightgrey)

---

![VOX TUI](screenshots/Pasted%20image.png)

---

## Features

- **Two-panel TUI** — albums on the right, tracks on the left, metadata above the track list
- **Keyboard + mouse** — vim-style navigation, click to select, double-click to play
- **Live search** — filter by title, artist, or album in real time
- **File browser** — navigate your filesystem and scan folders directly from the TUI
- **Playback controls** — play, pause, seek, next, previous, repeat, shuffle
- **Volume control** — raise/lower in 5% steps, mute toggle
- **Progress bar** — visual elapsed/total with codec and bitrate info
- **Persistent state** — volume, shuffle, repeat, and music dirs saved to config automatically
- **Fast library** — embedded [sled](https://github.com/spacejam/sled) database, no external server needed
- **Format support** — FLAC, MP3, WAV, OGG, Opus, AAC, AIFF, M4A

---

## Installation

**Requirements:** Rust toolchain + ALSA dev libs

```bash
# Arch
sudo pacman -S alsa-lib

# Debian / Ubuntu
sudo apt install libasound2-dev
```

**Build from source:**

```bash
git clone https://github.com/CodeMonkX/VOX.git
cd VOX
cargo build --release
cp target/release/vox ~/.local/bin/
```

**Or use the install script:**

```bash
./install.sh
```

---

## Quick Start

```bash
vox                        # Launch the TUI
vox scan ~/Music           # Scan a folder into the library
vox list tracks            # List all tracks
vox search "pink floyd"    # Search the library
vox info "wish you were"   # Show full metadata for a track
vox --help                 # All CLI commands
```

---

## TUI Layout

```
┌─ status / mode bar ──────────────────────────────────────────┐
│                                                               │
│  ┌─ metadata (38%) ──────┐  ┌─ albums (62%) ──────────────┐  │
│  │  Name:   Track Title  │  │   Album One   (12 tracks)   │  │
│  │  Album:  Album Name   │  │ ▸ Album Two ♪ (8 tracks)    │  │
│  │  Artist: Artist Name  │  │   Album Three (5 tracks)    │  │
│  │  Length: 4:32         │  │                             │  │
│  └───────────────────────┘  └─────────────────────────────┘  │
│  ┌─ tracks ──────────────┐                                    │
│  │  01. Song One   3:12  │                                    │
│  │▸ 02. Song Two ♪ 4:32  │                                    │
│  │  03. Song Three 2:58  │                                    │
│  └───────────────────────┘                                    │
│                                                               │
├─ now playing · artist — title · ━━━━━━━━─────── · 2:10/4:32 ─┤
└─ help bar (context-sensitive) ───────────────────────────────┘
```

The `♪` symbol marks the currently playing track and its album.

---

## Keybindings

### Navigation

| Key | Action |
|---|---|
| `↑` / `↓` | Move up / down in focused panel |
| `←` / `→` or `Tab` | Switch focus between albums and tracks |
| `Enter` | Select album (loads tracks) / play track |

### Playback

| Key | Action |
|---|---|
| `Space` / `k` | Play / pause |
| `n` / `N` | Next track |
| `b` / `B` | Previous track |
| `p` / `P` | Restart current track (seek to 0) |
| `j` | Seek back 5 seconds |
| `l` | Seek forward 5 seconds |

### Volume

| Key | Action |
|---|---|
| `+` / `=` | Volume up 5% |
| `-` / `_` | Volume down 5% |
| `m` / `M` | Mute / unmute |

### Library

| Key | Action |
|---|---|
| `/` | Open file browser to scan a folder |
| `x` / `X` | Remove a tracked path from the library |
| `D` | Remove selected album from library |
| `f` / `F` | Search tracks |
| `Ctrl+R` | Rescan all tracked paths |

### Modes & Misc

| Key | Action |
|---|---|
| `r` / `R` | Toggle repeat |
| `s` / `S` | Toggle shuffle |
| `Esc` | Cancel current mode / go up in browser |
| `q` / `Q` | Quit |
| `Ctrl+C` | Force quit |

### Mouse

| Action | Result |
|---|---|
| Click — right panel | Select album |
| Click — left panel (below metadata) | Select track |
| Double-click — left panel | Play selected track |

---

## Input Modes

**Browse** (`/`) — Navigate the filesystem. `↑`/`↓` to move, `Enter` to open a directory, `s` to scan the current folder, `Esc` to go up or cancel.

**Search** (`f`) — Type to filter all tracks by title, artist, or album. `↑`/`↓` to pick a result, `Enter` to play, `Esc` to exit.

**Remove path** (`x`) — Shows all scanned folders. `↑`/`↓` to select, `Enter` to remove every track under it from the library, `Esc` to cancel.

---

## CLI Reference

```bash
vox scan [folder]              # Scan folder (defaults to configured music dir)
vox list tracks|albums|artists # List library contents
vox search <keyword>           # Search by title, artist, or album
vox info <path|keyword>        # Show full metadata for a track
vox remove-path <path>         # Remove all tracks under a path
vox config set-music-dir <path># Set your music directory
vox config show                # Show current config
vox playlist create <name>     # Create a playlist
vox playlist list              # List all playlists
vox playlist show <name>       # Show tracks in a playlist
```

---

## Configuration

Stored at `~/.config/vox/config.json` — edited automatically by the TUI and CLI.

```json
{
  "music_dirs": ["/home/user/Music"],
  "volume": 0.8,
  "shuffle": false,
  "repeat": false
}
```

| Path | Contents |
|---|---|
| `~/.config/vox/config.json` | Main config |
| `~/.config/vox/library.db` | sled database (track index) |
| `~/.config/vox/playlists/` | JSON playlist files |

> **Database locked?** Delete `~/.config/vox/library.db` and re-scan. VOX also attempts to recover from stale locks automatically on startup.

---

## Tech Stack

| Crate | Role |
|---|---|
| [ratatui](https://github.com/ratatui/ratatui) | TUI rendering |
| [crossterm](https://github.com/crossterm-rs/crossterm) | Terminal backend & input events |
| [rodio](https://github.com/RustAudio/rodio) | Audio decoding & playback |
| [lofty](https://github.com/Serial-ATA/lofty-rs) | Audio tag & metadata parsing |
| [sled](https://github.com/spacejam/sled) | Embedded key-value database |
| [clap](https://github.com/clap-rs/clap) | CLI argument parsing |
| [walkdir](https://github.com/BurntSushi/walkdir) | Recursive directory traversal |
| [serde_json](https://github.com/serde-rs/json) | Config & playlist serialization |

---

## License

MIT — do what you want with it.
