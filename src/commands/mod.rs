use std::sync::Arc;

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::config::Config;
use crate::db::Database;
use crate::library::Library;
use crate::metadata;
use crate::utils;

#[derive(Parser)]
#[command(name = "music", version, about = "A terminal music player — ncmpcpp-style TUI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Scan a folder for music files and add to library
    Scan {
        /// Folder to scan (defaults to configured music dir)
        folder: Option<String>,
    },
    /// List tracks, albums, or artists in the library
    List {
        /// What to list: tracks, albums, artists
        kind: String,
    },
    /// Search the library
    Search {
        /// Search keyword
        keyword: String,
    },
    /// Show track metadata
    Info {
        /// Track path or search keyword
        track: String,
    },
    /// Remove all tracks under a path from the library
    RemovePath {
        /// Path to remove from library
        path: String,
    },
    /// Configuration management
    Config {
        #[command(subcommand)]
        cmd: ConfigCommand,
    },
    /// Playlist management
    Playlist {
        #[command(subcommand)]
        cmd: PlaylistCommand,
    },
}

#[derive(Subcommand)]
pub enum ConfigCommand {
    /// Set your music directory
    SetMusicDir {
        /// Path to your music collection
        path: String,
    },
    /// Show current configuration
    Show,
}

#[derive(Subcommand)]
pub enum PlaylistCommand {
    Create { name: String },
    List,
    Show { name: String },
}

pub fn handle_command(cli: &Cli, db: &Arc<Database>) -> Result<()> {
    let library = Library::new(db.clone());

    match &cli.command {
        Some(Commands::Scan { folder }) => cmd_scan(folder.as_deref(), &library),
        Some(Commands::List { kind }) => cmd_list(kind, &library),
        Some(Commands::Search { keyword }) => cmd_search(keyword, &library),
        Some(Commands::Info { track }) => cmd_info(track, &library),
        Some(Commands::RemovePath { path }) => cmd_remove_path(path, &library),
        Some(Commands::Config { cmd }) => cmd_config(cmd),
        Some(Commands::Playlist { cmd }) => cmd_playlist(cmd, &library),
        None => {
            // No subcommand = launch TUI
            Ok(())
        }
    }
}

fn cmd_scan(folder: Option<&str>, library: &Library) -> Result<()> {
    let folder = match folder {
        Some(f) => f.to_string(),
        None => {
            let config = Config::load()?;
            match config.music_dirs.first() {
                Some(d) => d.to_string_lossy().to_string(),
                None => anyhow::bail!("No music directory configured. Use: music config set-music-dir <path>"),
            }
        }
    };
    let path = std::path::Path::new(&folder);
    if !path.exists() {
        anyhow::bail!("Folder '{folder}' does not exist");
    }
    println!("Scanning {folder}...");
    let count = library.scan(path)?;
    println!("Scanned {count} tracks into library");
    Ok(())
}

fn cmd_remove_path(path: &str, library: &Library) -> Result<()> {
    let p = std::path::Path::new(path);
    let prefix = if p.exists() {
        p.canonicalize()?.to_string_lossy().to_string()
    } else {
        path.to_string()
    };
    let count = library.remove_by_prefix(&prefix)?;
    println!("Removed {count} tracks from library");
    Ok(())
}

fn cmd_config(cmd: &ConfigCommand) -> Result<()> {
    match cmd {
        ConfigCommand::SetMusicDir { path } => {
            let p = std::path::Path::new(path);
            if !p.exists() {
                anyhow::bail!("Path '{path}' does not exist");
            }
            let mut config = Config::load()?;
            config.music_dirs = vec![p.to_path_buf()];
            config.save()?;
            println!("Music directory set to: {path}");
            println!("Run `music scan` to scan your library.");
        }
        ConfigCommand::Show => {
            let config = Config::load()?;
            println!("Music directories:");
            for d in &config.music_dirs {
                println!("  {}", d.display());
            }
            println!("Volume: {}%", (config.volume * 100.0) as u8);
            println!("Shuffle: {}", if config.shuffle { "ON" } else { "OFF" });
            println!("Repeat: {}", if config.repeat { "ON" } else { "OFF" });
        }
    }
    Ok(())
}

fn cmd_list(kind: &str, library: &Library) -> Result<()> {
    match kind {
        "tracks" | "songs" => {
            let tracks = library.list_tracks()?;
            for (i, t) in tracks.iter().enumerate() {
                println!(
                    "{:>4}. {} - {} [{}]",
                    i + 1,
                    t.artist,
                    t.title,
                    t.album
                );
            }
            println!("\nTotal: {} tracks", tracks.len());
        }
        "albums" => {
            let albums = library.albums()?;
            for (album, artist) in &albums {
                println!("{album} - {artist}");
            }
            println!("\nTotal: {} albums", albums.len());
        }
        "artists" => {
            let artists = library.artists()?;
            for artist in &artists {
                println!("{artist}");
            }
            println!("\nTotal: {} artists", artists.len());
        }
        _ => {
            anyhow::bail!("Unknown list type '{kind}'. Use: tracks, albums, or artists");
        }
    }
    Ok(())
}

fn cmd_search(keyword: &str, library: &Library) -> Result<()> {
    let tracks = library.search(keyword)?;
    if tracks.is_empty() {
        println!("No results found for '{keyword}'");
        return Ok(());
    }
    for t in &tracks {
        println!(
            "  {} - {} [{}] ({})",
            t.artist,
            t.title,
            t.album,
            utils::format_duration(t.duration)
        );
    }
    println!("\nFound {} result(s)", tracks.len());
    Ok(())
}

fn cmd_info(track: &str, library: &Library) -> Result<()> {
    let path = std::path::Path::new(track);
    let track_meta = if path.exists() {
        metadata::read_track(path)?
    } else {
        let tracks = library.search(track)?;
        match tracks.first() {
            Some(t) => metadata::read_track(std::path::Path::new(&t.path))?,
            None => anyhow::bail!("Track not found: '{track}'"),
        }
    };

    println!("  Title:       {}", track_meta.title);
    println!("  Artist:      {}", track_meta.artist);
    println!("  Album:       {}", track_meta.album);
    println!("  Genre:       {}", track_meta.genre);
    println!("  Year:        {}", track_meta.year);
    println!("  Duration:    {}", utils::format_duration(track_meta.duration));
    println!("  Bitrate:     {}", utils::format_bitrate(track_meta.bitrate));
    println!("  Sample Rate: {}", utils::format_sample_rate(track_meta.sample_rate));
    println!("  Codec:       {}", track_meta.codec);
    println!("  Path:        {}", track_meta.path);

    Ok(())
}

fn cmd_playlist(cmd: &PlaylistCommand, _library: &Library) -> Result<()> {
    let pl_dir = Config::playlists_dir()?;

    match cmd {
        PlaylistCommand::Create { name } => {
            let pl = crate::playlist::Playlist::new(name);
            let path = pl_dir.join(format!("{name}.json"));
            pl.save(&path)?;
            println!("Playlist '{name}' created");
        }
        PlaylistCommand::List => {
            for entry in std::fs::read_dir(&pl_dir)? {
                let entry = entry?;
                if entry.file_name().to_string_lossy().ends_with(".json") {
                    let name = entry.file_name().to_string_lossy().replace(".json", "");
                    let pl = crate::playlist::Playlist::load(&entry.path())?;
                    println!("  {name} ({} tracks)", pl.tracks.len());
                }
            }
        }
        PlaylistCommand::Show { name } => {
            let path = pl_dir.join(format!("{name}.json"));
            let pl = crate::playlist::Playlist::load(&path)?;
            println!("Playlist: {name}");
            for (i, t) in pl.tracks.iter().enumerate() {
                println!("  {}. {t}", i + 1);
            }
        }
    }
    Ok(())
}
