mod audio;
mod commands;
mod config;
mod db;
mod library;
mod metadata;
mod playlist;
mod tui;
mod utils;

use std::sync::Arc;

use anyhow::Result;
use clap::Parser;

use crate::commands::{handle_command, Cli};
use crate::config::Config;
use crate::db::Database;

fn open_db(db_path: &std::path::Path) -> Result<Database> {
    match Database::open(db_path) {
        Ok(db) => Ok(db),
        Err(e) => {
            let msg = format!("{e}");
            if msg.contains("could not acquire lock") {
                eprintln!("Database is locked. Removing stale lock...");
                // Remove the entire database directory to break the stale lock
                let _ = std::fs::remove_dir_all(db_path);
                std::thread::sleep(std::time::Duration::from_millis(500));
                Database::open(db_path)
            } else {
                Err(e)
            }
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let config = if Config::config_path()?.exists() {
        Config::load()?
    } else {
        let cfg = Config::default();
        cfg.save()?;
        cfg
    };

    let db_path = Config::db_path()?;
    let db = Arc::new(open_db(&db_path)?);

    // If a CLI subcommand was given, run it and exit
    if cli.command.is_some() {
        return handle_command(&cli, &db);
    }

    // Otherwise, launch the TUI (ncmpcpp-style interface)
    let player = audio::Player::new()?;
    if let Some(sys_vol) = audio::Player::read_system_volume() {
        player.set_volume(sys_vol);
    }
    let mut app = tui::App::new(config, db, player)?;
    app.run()
}
