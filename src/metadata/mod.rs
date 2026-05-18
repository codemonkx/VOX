use std::path::Path;

use anyhow::{Context, Result};
use lofty::file::AudioFile;
use lofty::file::TaggedFileExt;
use lofty::tag::Accessor;
use lofty::file::FileType;
use lofty::read_from_path;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Track {
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub genre: String,
    pub year: i32,
    pub duration: f64,
    pub bitrate: u32,
    pub sample_rate: u32,
    pub codec: String,
}

pub fn read_track(path: &Path) -> Result<Track> {
    let file = read_from_path(path).context("Failed to read audio file")?;
    let properties = file.properties();
    let tag = file.tags().first().cloned();

    let title = tag
        .as_ref()
        .and_then(|t| t.title())
        .unwrap_or_default()
        .to_string();
    let artist = tag
        .as_ref()
        .and_then(|t| t.artist())
        .unwrap_or_default()
        .to_string();
    let album = tag
        .as_ref()
        .and_then(|t| t.album())
        .unwrap_or_default()
        .to_string();
    let genre = tag
        .as_ref()
        .and_then(|t| t.genre().map(|g| g.to_string()))
        .unwrap_or_default();
    let year = tag
        .as_ref()
        .and_then(|t| t.year())
        .map(|y| y as i32)
        .unwrap_or(0);

    let duration = properties.duration().as_secs_f64();
    let bitrate = properties.audio_bitrate().unwrap_or(0);
    let sample_rate = properties.sample_rate().unwrap_or(0);
    let codec = format_codec(file.file_type());

    Ok(Track {
        path: path.to_string_lossy().to_string(),
        title,
        artist,
        album,
        genre,
        year,
        duration,
        bitrate,
        sample_rate,
        codec,
    })
}

fn format_codec(ft: FileType) -> String {
    match ft {
        FileType::Flac => "FLAC".into(),
        FileType::Mpeg => "MP3".into(),
        FileType::Wav => "WAV".into(),
        FileType::Vorbis => "OGG Vorbis".into(),
        FileType::Opus => "Opus".into(),
        FileType::Aiff => "AIFF".into(),
        FileType::Aac => "AAC".into(),
        FileType::Mp4 => "AAC (M4A)".into(),
        _ => format!("{ft:?}"),
    }
}

const SUPPORTED_EXTENSIONS: &[&str] = &["mp3", "flac", "wav", "ogg", "m4a", "aac", "opus"];

pub fn is_supported(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| SUPPORTED_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}
