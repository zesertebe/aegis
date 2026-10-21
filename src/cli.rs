//! CLI headless (clap): subcomandos sin TUI.

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "aegis",
    version,
    about = "Gestor de bovedas cifradas sobre restic"
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    /// Lista las bóvedas configuradas (headless)
    List,
    /// Comprueba que restic esté instalado y muestra su versión
    Check,
}
