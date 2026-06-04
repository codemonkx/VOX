use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use rayon::prelude::*;
use walkdir::WalkDir;

use crate::db::Database;
use crate::metadata::{self, Track};

pub struct Library {
    pub db: Arc<Database>,
}

impl Library {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn scan(&self, folder: &Path) -> Result<(usize, usize)> {
        let paths: Vec<_> = WalkDir::new(folder)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| e.path().to_path_buf())
            .filter(|p| metadata::is_supported(p))
            .collect();

        let results: Vec<Result<Track, ()>> = paths.par_iter()
            .map(|path| match metadata::read_track(path) {
                Ok(track) => Ok(track),
                Err(_) => Err(()),
            })
            .collect();

        let mut ok = 0;
        let mut err = 0;
        for r in results {
            match r {
                Ok(track) => {
                    self.db.store_track(&track)?;
                    ok += 1;
                }
                Err(_) => {
                    err += 1;
                }
            }
        }
        self.db.flush()?;
        Ok((ok, err))
    }

    pub fn list_tracks(&self) -> Result<Vec<Track>> {
        self.db.all_tracks()
    }

    pub fn tracks_by_album(&self, album: &str) -> Result<Vec<Track>> {
        self.db.tracks_by_album(album)
    }

    pub fn album_info(&self) -> Result<Vec<(String, usize)>> {
        self.db.album_info()
    }

    pub fn search(&self, keyword: &str) -> Result<Vec<Track>> {
        self.db.search_tracks(keyword)
    }

    pub fn artists(&self) -> Result<Vec<String>> {
        self.db.distinct_artists()
    }

    pub fn albums(&self) -> Result<Vec<(String, String)>> {
        self.db.distinct_albums()
    }

        pub fn remove_by_prefix(&self, prefix: &str) -> Result<usize> {
        self.db.remove_tracks_by_prefix(prefix)
    }
}