//! aegis — gestor TUI de bóvedas cifradas sobre restic.
//! Un único binario autocontenido: `cargo build --release`.
//! Config en `~/.config/aegis/config.json`. Invoca al `restic` del sistema.

mod app;
mod cli;
mod config;
mod input;
mod restic;
mod ui;

use std::io::{self, Stdout};
use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::app::App;
use crate::cli::{Cli, Commands};
use crate::config::{config_path, load_config};
use crate::input::handle_key;
use crate::restic::{restic_exists, restic_version};
use crate::ui::draw;

fn run_tui() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout: Stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

    let res: Result<()> = (|| {
        loop {
            terminal.draw(|f| draw(f, &mut app))?;
            if app.should_quit {
                break;
            }
            if event::poll(Duration::from_millis(200))? {
                if let Event::Key(key) = event::read()? {
                    handle_key(&mut app, key.code, key.modifiers);
                }
            }
            if app.should_quit {
                break;
            }
        }
        Ok(())
    })();

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    // Limpieza best-effort del mount al salir.
    // (Se deja montado a propósito si el usuario quiere; solo informamos.)
    res
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::List) => {
            let cfg = load_config();
            if cfg.vaults.is_empty() {
                println!("(sin bóvedas en {})", config_path().display());
            } else {
                for v in &cfg.vaults {
                    println!("{} | {} -> {}", v.name, v.local_path, v.repo_target);
                }
            }
            Ok(())
        }
        Some(Commands::Check) => {
            if restic_exists() {
                println!("OK: {}", restic_version());
                Ok(())
            } else {
                println!("restic no encontrado. Instala con: sudo pacman -S restic");
                std::process::exit(1);
            }
        }
        None => run_tui(),
    }
}
