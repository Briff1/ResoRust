<div align="center">

# ResoRust

**Your offline music player — light, beautiful, and yours alone.**

![Rust](https://img.shields.io/badge/Rust-2024-orange?logo=rust)
![egui](https://img.shields.io/badge/UI-egui%200.37-6b4f3a)
![Windows](https://img.shields.io/badge/Windows-11%2F10-0078D4?logo=windows)
![License](https://img.shields.io/badge/license-Personal-gray)

Built in Rust, with care and no ads.  
**By Briff** 🎧

</div>

---

## ✨ Features

- **Local library** — recursively scans your music folder (MP3, FLAC, WAV, OGG and M4A).
- **Covers and metadata** — read and edit title, artist, album and cover inside the file itself. Folder covers (`cover.jpg`, `folder.jpg`, `front.jpg`) work too.
- **Playlists with custom covers** — create, edit, delete and pick an image for each playlist.
- **Playback queue** — playing a playlist follows its order; favorites and the library become a queue as well.
- **YouTube / YouTube Music downloads** — paste a link and the track lands straight in your library, with cover and tags. No `ffmpeg`, no Spotify, no heavy services.
- **Repeat that keeps working** — repeat a single track with the app minimized and playback continues normally.
- **Black & brown interface** with hand-drawn icons, animated vinyl, favorites, search and keyboard shortcuts.

> 🎵 Spotify is not supported: the service uses DRM, so legal downloading is not possible without its official app running.

## 🎮 Shortcuts

| Key | Action |
| --- | --- |
| `Space` | Play / pause |
| `Ctrl + →` / `Ctrl + ←` | Next / previous |
| `↑` / `↓` | Volume |
| `M` | Mute |

## 📁 Where things live

| Item | Path |
| --- | --- |
| Music | `%USERPROFILE%\Music\ResoRust` (configurable) |
| Preferences, favorites and playlists | `%APPDATA%\ResoRust\state.json` |
| Playlist covers | `%APPDATA%\ResoRust\covers` |
| `yt-dlp` | `%APPDATA%\ResoRust\bin` |

## 🚀 Building

Requirement: [Rust](https://rustup.rs/) (toolchain `stable-x86_64-pc-windows-msvc`).

```powershell
cargo build --release
```

The executable is at `target\release\resorust.exe`.  
For development, run `cargo run`.

## 🧱 Stack

| Layer | Tool |
| --- | --- |
| Interface | [`egui` / `eframe`](https://github.com/emilk/egui) 0.37 |
| Audio | [`rodio`](https://github.com/RustAudio/rodio) 0.23 |
| Metadata | [`lofty`](https://github.com/Serial-ATA/lofty-rs) 0.25 |
| Images | [`image`](https://github.com/image-rs/image) |
| Native dialogs | [`rfd`](https://github.com/PolyMeilex/rfd) |
| Downloads | external, optional `yt-dlp` |

## 📄 License

Personal prototype. The Poppins fonts are under the SIL Open Font License.

<div align="center">

By Briff

</div>
