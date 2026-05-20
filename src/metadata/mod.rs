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
    match read_from_path(path) {
        Ok(file) => {
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
        Err(_) => read_with_ffprobe(path),
    }
}

fn read_with_ffprobe(path: &Path) -> Result<Track> {
    let output = std::process::Command::new("ffprobe")
        .args([
            "-v", "quiet",
            "-print_format", "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(path)
        .output()
        .context("Failed to run ffprobe")?;

    if !output.status.success() {
        anyhow::bail!("ffprobe returned error");
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)
        .context("Failed to parse ffprobe output")?;

    let format = &json["format"];
    let stream = json["streams"].as_array()
        .and_then(|s| s.first())
        .unwrap_or(&serde_json::Value::Null);

    let extract = |field: &str| -> String {
        format["tags"][field]
            .as_str()
            .or_else(|| stream["tags"][field].as_str())
            .unwrap_or_default()
            .to_string()
    };

    let title = extract("title");
    let artist = extract("artist");
    let album = extract("album");

    let duration: f64 = format["duration"].as_str()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);

    let bitrate: u32 = format["bit_rate"].as_str()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let sample_rate: u32 = stream["sample_rate"].as_str()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let codec_name = stream["codec_name"].as_str().unwrap_or("unknown");
    let codec = match codec_name {
        "dsd_lsbf_planar" | "dsd_lsbf" | "dsd_msbf_planar" | "dsd_msbf" => "DSD",
        _ => codec_name.to_uppercase(),
    };

    Ok(Track {
        path: path.to_string_lossy().to_string(),
        title,
        artist,
        album,
        genre: extract("genre"),
        year: extract("date").parse().unwrap_or(0),
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

const SUPPORTED_EXTENSIONS: &[&str] = &["mp3", "flac", "wav", "ogg", "m4a", "aac", "opus", "dsf", "aiff"];

pub fn is_supported(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| SUPPORTED_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}
