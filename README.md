# 🎵 VOX

> A terminal music player 🎧 — fast, keyboard-driven, and built with Rust 🦀

![MIT License](https://img.shields.io/badge/license-MIT-blue)

---

## ✨ Features

| | |
|---|---|
| 🎛️ **Two-panel layout** | Browse albums on the right, pick tracks on the left |
| ⌨️ **Keyboard-only** | No mouse needed — vim-style navigation |
| 📋 **Metadata panel** | See title, artist, album, bitrate, sample rate & codec |
| ▶️ **Now playing** | `♪` highlights the current track and album |
| 📂 **File browser** | Press `/` to browse folders and pick what to scan |
| 🔍 **Search** | Live filter tracks by title, artist, or album |
| 🔁 **Repeat & Shuffle** | Toggle on/off, indicators in the top bar |
| 🔊 **Volume control** | `+`/`-` to adjust, `m` to mute |
| 📊 **Progress bar** | Visual playback progress with elapsed/total time |
| ⚙️ **Persistent config** | Volume, shuffle, repeat & music dirs saved automatically |
| 🗄️ **Database** | Fast library lookups powered by sled |
| 🎶 **Formats** | FLAC, MP3, WAV, OGG, Opus, AAC, AIFF, M4A |

---

## 📸 Screenshot

![VOX TUI](screenshots/Pasted%20image.png)

---

## 🚀 Installation

```bash
git clone https://github.com/CodeMonkX/VOX.git
cd VOX
cargo build --release
cp target/release/vox ~/.local/bin/
```

Or use the install script:

```bash
./install.sh
```

**Dependencies:** Rust toolchain + ALSA dev libs  
(`alsa-lib` on Arch, `libasound2-dev` on Debian/Ubuntu)

---

## 🕹️ Usage

```bash
vox                     # Launch the TUI
vox scan ~/Music        # Scan a folder into the library
vox list tracks         # List all tracks (CLI)
vox search "artist"     # Search the library (CLI)
vox --help              # View all commands
```

### ⌨️ Keybindings

| Key | Action |
|---|---|
| `↑` / `↓` | Navigate albums / tracks |
| `←` / `→` or `Tab` | Switch between album & track panel |
| `Enter` | Select album / play track |
| `Space` / `k` | Play / pause |
| `n` / `N` | Next track |
| `b` / `B` | Previous track |
| `+` / `=` | Volume up |
| `-` / `_` | Volume down |
| `m` / `M` | Mute / unmute |
| `r` / `R` | Toggle repeat |
| `s` / `S` | Toggle shuffle |
| `/` | Open file browser to scan a folder |
| `x` | Remove a tracked path from the library |
| `f` / `F` | Search tracks |
| `d` / `D` | Remove selected album |
| `q` / `Q` | Quit |
| `Esc` | Cancel current mode |
| `Ctrl+R` | Rescan all tracked paths for new files |
| `Ctrl+C` | Force quit |

### 🧭 Input Modes

<details>
<summary><b>Browse folders</b> — press <code>/</code></summary>

Navigate the filesystem using `↑`/`↓`.  
Press `Enter` to open a directory, `Esc` to go up or cancel.  
Press `s` to scan the current folder into your library.

</details>

<details>
<summary><b>Remove path</b> — press <code>x</code></summary>

Shows all folders you've scanned.  
Use `↑`/`↓` to pick one and press `Enter` to remove every track under it.
</details>

<details>
<summary><b>Search</b> — press <code>f</code> / <code>F</code></summary>

Type to filter by title, artist, or album.  
`↑`/`↓` to select a result, `Enter` to play it, `Esc` to exit.
</details>

---

## ⚙️ Configuration

File: `~/.config/vox/config.json`

```json
{
  "music_dirs": ["/home/user/Music"],
  "volume": 0.8,
  "shuffle": false,
  "repeat": false
}
```

Folders you add or remove through the TUI are saved here automatically.

---

## 🗃️ Database

Stored at `~/.config/vox/library.db` (sled).  

> **Locked?** If you see `database locked`, delete this folder and re-scan.

---

## 🧰 Tech Stack

| Tool | What it does |
|---|---|
| [ratatui](https://github.com/ratatui/ratatui) | TUI framework |
| [crossterm](https://github.com/crossterm-rs/crossterm) | Terminal backend |
| [rodio](https://github.com/RustAudio/rodio) | Audio playback |
| [lofty](https://github.com/Serial-ATA/lofty-rs) | Metadata & tag parsing |
| [sled](https://github.com/spacejam/sled) | Embedded database |
| [clap](https://github.com/clap-rs/clap) | CLI argument parser |

---

## 📄 License

MIT — do what you want with it.
