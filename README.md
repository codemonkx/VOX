<p align="center">
  <img src="screenshots/Pasted%20image.png" alt="VOX TUI" width="720">
</p>

<h1 align="center">VOX</h1>

<p align="center">
  <b>A terminal music player — fast, keyboard-driven, built with Rust</b>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/license-MIT-blue">
  <img src="https://img.shields.io/badge/built_with-Rust-orange">
  <img src="https://img.shields.io/badge/platform-Linux-lightgrey">
</p>

---

## Install

```bash
# Dependencies: Rust + ALSA
#   Arch:   sudo pacman -S alsa-lib
#   Debian: sudo apt install libasound2-dev

git clone https://github.com/CodeMonkX/VOX.git
cd VOX
cargo build --release
cp target/release/vox ~/.local/bin/
```

## Quick Start

```
vox                        Launch the TUI
vox scan ~/Music           Scan a folder into the library
vox search "pink floyd"    Search all tracks
vox --help                 All CLI commands
```

## Keybindings

| Keys | Action |
|---|---|
| `↑` `↓` | Navigate focused panel |
| `←` `→` / `Tab` | Switch panel focus |
| `Enter` | Select album / play track |
| `Space` / `k` | Play / pause |
| `n` `b` | Next / previous track |
| `j` `l` | Seek -5s / +5s |
| `+` `-` | Volume up / down 5% |
| `m` | Mute / unmute |
| `f` | Search tracks |
| `/` | Browse & scan folders |
| `r` `s` | Toggle repeat / shuffle |
| `q` | Quit |

## Layout

```
┌─ status bar ─────────────────────────────────────────────────┐
│                                                               │
│  ┌─ metadata ───────────┐  ┌─ albums ──────────────────────┐  │
│  │  Name:   Track Title  │  │   Album One   (12 tracks)    │  │
│  │  Album:  Album Name   │  │ ▸ Album Two ♪ (8 tracks)     │  │
│  │  Artist: Artist Name  │  │   Album Three (5 tracks)     │  │
│  │  Length: 4:32         │  │                              │  │
│  └───────────────────────┘  └──────────────────────────────┘  │
│  ┌─ tracks ──────────────┐                                    │
│  │  01. Song One   3:12  │                                    │
│  │▸ 02. Song Two ♪ 4:32  │                                    │
│  │  03. Song Three 2:58  │                                    │
│  └───────────────────────┘                                    │
│                                                               │
├─ now playing · Artist — Title · ████████────── · 2:10/4:32 ──┤
└─ help bar (context-sensitive) ───────────────────────────────┘
```

## Formats

FLAC, MP3, WAV, OGG, Opus, AAC, AIFF, M4A, **DSD/DSF**

## Config

Stored at `~/.config/vox/config.json` — edited automatically.

```json
{
  "music_dirs": ["/home/user/Music"],
  "volume": 0.8,
  "shuffle": false,
  "repeat": false
}
```

## License

MIT
