use std::path::Path;

use anyhow::{Context, Result};
use sled::Db;

use crate::metadata::Track;

pub struct Database {
    db: Db,
    albums: sled::Tree,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        let db = sled::open(path).context("Failed to open database")?;
        let albums = db.open_tree("albums")?;
        Ok(Self { db, albums })
    }

    pub fn store_track(&self, track: &Track) -> Result<()> {
        let key = track.path.as_bytes();
        let value = serde_json::to_vec(track)?;
        self.db.insert(key, value)?;
        let album_key = format!("{}::{}", track.album, track.path);
        self.albums.insert(album_key.as_bytes(), b"")?;
        Ok(())
    }

    pub fn get_track(&self, path: &str) -> Result<Option<Track>> {
        let key = path.as_bytes();
        match self.db.get(key)? {
            Some(data) => {
                let track: Track = serde_json::from_slice(&data)?;
                Ok(Some(track))
            }
            None => Ok(None),
        }
    }

    pub fn remove_track(&self, path: &str) -> Result<()> {
        if let Some(track) = self.get_track(path)? {
            let album_key = format!("{}::{}", track.album, path);
            self.albums.remove(album_key.as_bytes())?;
        }
        self.db.remove(path.as_bytes())?;
        Ok(())
    }

    pub fn remove_tracks_by_prefix(&self, prefix: &str) -> Result<usize> {
        let to_remove: Vec<String> = self
            .all_tracks()?
            .into_iter()
            .filter(|t| t.path.starts_with(prefix))
            .map(|t| t.path.clone())
            .collect();
        let count = to_remove.len();
        for path in &to_remove {
            self.remove_track(path)?;
        }
        self.db.flush()?;
        Ok(count)
    }

    pub fn all_tracks(&self) -> Result<Vec<Track>> {
        let mut tracks = Vec::new();
        for result in self.db.iter() {
            let (_, value) = result?;
            let track: Track = serde_json::from_slice(&value)?;
            tracks.push(track);
        }
        Ok(tracks)
    }

    pub fn tracks_by_album(&self, album: &str) -> Result<Vec<Track>> {
        let prefix = format!("{}::", album);
        let mut tracks = Vec::new();
        for result in self.albums.scan_prefix(prefix.as_bytes()) {
            let (key, _) = result?;
            let path = std::str::from_utf8(&key)
                .ok()
                .and_then(|k| k.split_once("::"))
                .map(|(_, p)| p)
                .unwrap_or("");
            if !path.is_empty()
                && let Some(track) = self.get_track(path)? {
                    tracks.push(track);
                }
        }
        Ok(tracks)
    }

    pub fn album_info(&self) -> Result<Vec<(String, usize)>> {
        use std::collections::HashMap;
        let mut counts: HashMap<String, usize> = HashMap::new();
        for result in self.albums.iter() {
            let (key, _) = result?;
            if let Ok(ks) = std::str::from_utf8(&key)
                && let Some((album, _)) = ks.split_once("::")
                    && !album.is_empty() {
                        *counts.entry(album.to_string()).or_insert(0) += 1;
                    }
        }
        let mut info: Vec<(String, usize)> = counts.into_iter().collect();
        info.sort_by_key(|a| a.0.to_lowercase());
        Ok(info)
    }

    pub fn search_tracks(&self, keyword: &str) -> Result<Vec<Track>> {
        let lower = keyword.to_lowercase();
        let tracks = self.all_tracks()?;
        Ok(tracks
            .into_iter()
            .filter(|t| {
                t.title.to_lowercase().contains(&lower)
                    || t.artist.to_lowercase().contains(&lower)
                    || t.album.to_lowercase().contains(&lower)
            })
            .collect())
    }

    pub fn distinct_artists(&self) -> Result<Vec<String>> {
        let mut artists: Vec<String> = self
            .all_tracks()?
            .into_iter()
            .map(|t| t.artist)
            .filter(|a| !a.is_empty())
            .collect();
        artists.sort();
        artists.dedup();
        Ok(artists)
    }

    pub fn distinct_albums(&self) -> Result<Vec<(String, String)>> {
        let mut albums: Vec<(String, String)> = self
            .all_tracks()?
            .into_iter()
            .map(|t| (t.album.clone(), t.artist.clone()))
            .filter(|(a, _)| !a.is_empty())
            .collect();
        albums.sort();
        albums.dedup();
        Ok(albums)
    }

    pub fn flush(&self) -> Result<()> {
        self.db.flush()?;
        Ok(())
    }
}
