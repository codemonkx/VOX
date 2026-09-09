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
                anyhow::bail!(
                    "VOX database is locked. Another instance of VOX is currently running. Please close it and try again."
                );
            } else {
                Err(e)
            }
        }
    }
}

fn setup_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = crossterm::terminal::disable_raw_mode();
        let mut stdout = std::io::stdout();
        let _ = crossterm::ExecutableCommand::execute(&mut stdout, crossterm::event::DisableMouseCapture);
        let _ = crossterm::ExecutableCommand::execute(&mut stdout, crossterm::terminal::LeaveAlternateScreen);
        default_hook(panic_info);
    }));
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

    // Install panic hook to restore terminal in case of unexpected panic
    setup_panic_hook();

    // Otherwise, launch the TUI (ncmpcpp-style interface)
    let player = audio::Player::new()?;
    if let Some(sys_vol) = audio::Player::read_system_volume() {
        player.set_volume(sys_vol);
    }
    let mut app = tui::App::new(config, db, player)?;
    app.run()
}
