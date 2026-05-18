use std::path::Path;

use anyhow::{Context, Result};
use sled::Db;

use crate::metadata::Track;

pub struct Database {
    db: Db,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        let db = sled::open(path).context("Failed to open database")?;
        Ok(Self { db })
    }

    pub fn store_track(&self, track: &Track) -> Result<()> {
        let key = track.path.as_bytes();
        let value = serde_json::to_vec(track)?;
        self.db.insert(key, value)?;
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
        self.db.remove(path.as_bytes())?;
        Ok(())
    }

    pub fn remove_tracks_by_prefix(&self, prefix: &str) -> Result<usize> {
        let to_remove: Vec<String> = self
            .all_tracks()?
            .into_iter()
            .filter(|t| t.path.starts_with(prefix))
            .map(|t| t.path)
            .collect();
        let count = to_remove.len();
        for path in &to_remove {
            self.db.remove(path.as_bytes())?;
        }
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

    pub fn distinct_album_names(&self) -> Result<Vec<String>> {
        let mut names: Vec<String> = self
            .all_tracks()?
            .into_iter()
            .map(|t| t.album)
            .filter(|a| !a.is_empty())
            .collect();
        names.sort();
        names.dedup();
        Ok(names)
    }

    pub fn track_count(&self) -> Result<u64> {
        Ok(self.db.len() as u64)
    }

}
