use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playlist {
    pub name: String,
    pub tracks: Vec<String>,
}

impl Playlist {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            tracks: Vec::new(),
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(path, data).context("Failed to save playlist")
    }

    pub fn load(path: &Path) -> Result<Self> {
        let data = std::fs::read_to_string(path).context("Failed to load playlist")?;
        let pl: Playlist = serde_json::from_str(&data)?;
        Ok(pl)
    }
}
