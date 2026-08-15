<p align="center">
  <img src="screenshots/Pasted%20image.png" alt="VOX TUI" width="720">
</p>

<h1 align="center">VOX</h1>

<p align="center">
  <b>A modern, high-performance terminal music player built with pure Rust & Ratatui</b>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/license-MIT-blue.svg">
  <img src="https://img.shields.io/badge/built_with-Rust-orange.svg">
  <img src="https://img.shields.io/badge/audio_engine-Symphonia-purple.svg">
  <img src="https://img.shields.io/badge/platform-Linux-lightgrey.svg">
</p>

---

## ✨ Features

- 🎵 **Bit-Perfect Lossless Audio Engine**: Pure Rust decoding via Symphonia — supports ALAC (Apple Lossless), Hi-Res FLAC (up to 24-bit / 192kHz), MP3, AAC, OGG, Opus, WAV, AIFF, and DSD/DSF.
- 🎤 **Apple Music / Spotify Split-View Lyrics**: Press `y` for a full-screen side-by-side lyrics viewport. Automatically extracts embedded tags (`©lyr`, `USLT`, `SYLT`, Vorbis comments) or local `.lrc` files with synchronized scrolling.
- ⚡ **Pure Rust Native Seeking**: Instant `-5s` (`j`) and `+5s` (`l`) seeking with live position updates and zero audio latency.
- 🎨 **6 Curated Color Themes**: Catppuccin Mocha, Nord, Dracula, Tokyo Night, Gruvbox Dark, and Cyberpunk Neon (toggle with `t`).
- 🗄️ **Embedded Fast Database**: Instant library search, tagging, and album sorting powered by Sled.
- ⌨️ **Intuitive Keyboard Controls**: Vim-friendly, frictionless navigation with context-sensitive help bar.
- 🌐 **Unicode-Aware Layout Engine**: Full grapheme clustering and proper display width handling for international scripts and multi-byte metadata.

---

## 📦 Installation

### Prerequisites (Linux)
* **Rust**: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
* **ALSA Development Libraries**:
  * **Arch / Manjaro**: `sudo pacman -S alsa-lib`
  * **Ubuntu / Debian / Linux Mint / Zorin**: `sudo apt install libasound2-dev pkg-config`
  * **Fedora**: `sudo dnf install alsa-lib-devel`

### Quick Install
```bash
git clone https://github.com/CodeMonkX/VOX.git
cd VOX
./install.sh
```

*The binary will be compiled with release optimizations and installed to `~/.local/bin/vox`.*

---

## 🚀 Quick Start

```bash
vox                        # Launch the interactive TUI
vox scan ~/Music           # Scan a music directory into the library
vox search "Pink Floyd"    # Instant search across all tracks and albums
vox list albums            # List all scanned albums
vox list tracks            # List all tracks with track numbers
vox info "/path/to/song"   # View full audio metadata and embedded tags
vox --help                 # View all CLI commands
```

---

## ⌨️ Keybindings

| Key | Action |
|---|---|
| <kbd>↑</kbd> / <kbd>↓</kbd> | Navigate focused panel (Albums / Tracks / Search) |
| <kbd>←</kbd> / <kbd>→</kbd> or <kbd>Tab</kbd> | Switch active panel focus |
| <kbd>Enter</kbd> | Select album / Play selected track |
| <kbd>Space</kbd> or <kbd>k</kbd> | Play / Pause |
| <kbd>j</kbd> / <kbd>l</kbd> | **Seek backward (-5s) / Seek forward (+5s)** |
| <kbd>n</kbd> / <kbd>b</kbd> | Next track / Previous track |
| <kbd>y</kbd> | **Toggle Split-View Lyrics Panel** (Synced `.lrc` & Embedded tags) |
| <kbd>t</kbd> | **Cycle Color Themes** *(Catppuccin, Nord, Dracula, Tokyo Night, Gruvbox, Cyberpunk)* |
| <kbd>+</kbd> / <kbd>-</kbd> | Volume Up / Volume Down (5% steps) |
| <kbd>m</kbd> | Mute / Unmute |
| <kbd>f</kbd> | Search music library |
| <kbd>/</kbd> | File browser & folder scanner |
| <kbd>r</kbd> / <kbd>s</kbd> | Toggle Repeat / Shuffle |
| <kbd>?</kbd> | Show Keybindings Help Overlay |
| <kbd>q</kbd> | Quit VOX |

---

## 🖥️ UI Layout

### Standard Library Mode
```
┌─ status bar ─────────────────────────────────────────────────────────┐
│                                                                       │
│  ┌─ Now Playing Metadata ─┐  ┌─ Albums ────────────────────────────┐  │
│  │ Title:   Song Title    │  │   Album One            (12 tracks)  │  │
│  │ Artist:  Artist Name   │  │ ▸ Engeyum Eppodhum   ♪ (5 tracks)   │  │
│  │ Album:   Album Name    │  │   Album Three          (8 tracks)   │  │
│  │ Format:  ALAC 16B/44k  │  └─────────────────────────────────────┘  │
│  └────────────────────────┘  ┌─ Tracks ────────────────────────────┐  │
│                              │  01. Govinda          4:05          │  │
│                              │▸ 02. Chotta Chotta  ♪ 4:58          │  │
│                              │  03. Maasama          4:44          │  │
│                              └─────────────────────────────────────┘  │
├─ ▶ Now Playing · Artist — Title · ▓▓▓▓▓▓▓▓▓▓──────── · 2:15 / 4:05 ──┤
└─ [Space] Pause  [j/l] Seek  [y] Lyrics  [t] Theme  [?] Help  [q] Quit ┘
```

### Split-View Lyrics Mode (`y`)
```
┌─ VOX ────────────────────────────────────────────────────────────────┐
│  ┌─ Metadata / Tracks ────┐  ┌─ 🎤 Lyrics · Song Title ────────────┐  │
│  │ Title:  Song Title     │  │   Artist · Album  [ 🎵 Synced ]      │  │
│  │ Artist: Artist Name    │  ├─────────────────────────────────────┤  │
│  │ Album:  Album Name     │  │   Previous line of lyrics           │  │
│  │ Format: FLAC 24B/96k   │  │                                     │  │
│  │                        │  │ ▸ Active Line Currently Playing     │  │
│  │                        │  │                                     │  │
│  │                        │  │   Upcoming line of lyrics           │  │
│  └────────────────────────┘  └─────────────────────────────────────┘  │
├─ ▶ Now Playing · Artist — Title · ▓▓▓▓▓▓▓▓▓▓──────── · 2:15 / 4:05 ──┤
└─ [Space] Pause  [j/l] Seek  [y] Close Lyrics  [t] Theme  [q] Quit ───┘
```

---

## 🎧 Supported Audio Formats

| Format | Extension | Bit Depth / Sample Rate | Engine |
|---|---|---|---|
| **ALAC** | `.m4a`, `.alac` | 16-bit, 24-bit, 32-bit (Native) | Symphonia 0.5 |
| **FLAC** | `.flac` | Up to 24-bit / 192kHz Hi-Res | Symphonia 0.5 |
| **MP3** | `.mp3` | Constant & Variable Bitrate (VBR) | Symphonia 0.5 |
| **AAC / M4A** | `.aac`, `.m4a` | LC, HE-AAC | Symphonia 0.5 |
| **OGG / Opus**| `.ogg`, `.opus` | Vorbis, Opus | Symphonia 0.5 |
| **WAV / AIFF**| `.wav`, `.aiff` | PCM 16/24/32-bit | Symphonia 0.5 |

---

## ⚙️ Configuration

Configuration is stored at `~/.config/vox/config.json` and automatically saved when you modify settings (such as volume, theme, repeat, or shuffle):

```json
{
  "music_dirs": [
    "/home/user/Music"
  ],
  "volume": 0.85,
  "shuffle": false,
  "repeat": false,
  "theme": "Catppuccin"
}
```

---

## 📄 License

This project is licensed under the **MIT License**.
