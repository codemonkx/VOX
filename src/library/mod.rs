use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use walkdir::WalkDir;

use crate::db::Database;
use crate::metadata::{self, Track};

pub struct Library {
    db: Arc<Database>,
}

impl Library {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn scan(&self, folder: &Path) -> Result<usize> {
        let mut count = 0;
        for entry in WalkDir::new(folder).follow_links(true) {
            let entry = entry?;
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if !metadata::is_supported(path) {
                continue;
            }
            match metadata::read_track(path) {
                Ok(track) => {
                    self.db.store_track(&track)?;
                    count += 1;
                }
                Err(e) => {
                    eprintln!("  Skipping {}: {e}", path.display());
                }
            }
        }
        Ok(count)
    }

    pub fn list_tracks(&self) -> Result<Vec<Track>> {
        self.db.all_tracks()
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

    pub fn album_names(&self) -> Result<Vec<String>> {
        self.db.distinct_album_names()
    }

    pub fn track_count(&self) -> Result<u64> {
        self.db.track_count()
    }

    pub fn remove_by_prefix(&self, prefix: &str) -> Result<usize> {
        self.db.remove_tracks_by_prefix(prefix)
    }
}
