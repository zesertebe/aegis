//! Configuracion persistente: `~/.config/aegis/config.json`.

use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct Vault {
    pub(crate) name: String,
    pub(crate) local_path: String,
    pub(crate) repo_target: String,
    /// Compatibilidad con configs antiguas. No se usa: la password
    /// siempre se pide por prompt (decisión de diseño).
    #[serde(default)]
    pub(crate) password_env: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct AppConfig {
    #[serde(default)]
    pub(crate) vaults: Vec<Vault>,
}

pub(crate) fn config_path() -> PathBuf {
    if let Some(dir) = dirs::config_dir() {
        return dir.join("aegis").join("config.json");
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".config")
        .join("aegis")
        .join("config.json")
}

pub(crate) fn load_config() -> AppConfig {
    let path = config_path();
    let Ok(data) = std::fs::read_to_string(&path) else {
        return AppConfig::default();
    };
    serde_json::from_str(&data).unwrap_or_default()
}

pub(crate) fn save_config(cfg: &AppConfig) -> Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("no se pudo crear {}", parent.display()))?;
    }
    let data = serde_json::to_string_pretty(cfg)?;
    std::fs::write(&path, data)?;
    Ok(())
}

pub(crate) fn expand_tilde(p: &str) -> String {
    if let Some(rest) = p.strip_prefix("~/") {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        return format!("{home}/{rest}");
    }
    if p == "~" {
        return std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    }
    p.to_string()
}
