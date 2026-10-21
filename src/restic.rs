//! Invocaciones al binario `restic` del sistema.

use std::process::{Child, Command, Stdio};

pub(crate) fn restic_exists() -> bool {
    Command::new("restic")
        .arg("version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub(crate) fn restic_version() -> String {
    Command::new("restic")
        .arg("version")
        .output()
        .map(|o| {
            let mut s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if s.is_empty() {
                s = String::from_utf8_lossy(&o.stderr).trim().to_string();
            }
            if s.is_empty() {
                "restic (versión desconocida)".to_string()
            } else {
                s.lines().next().unwrap_or(&s).to_string()
            }
        })
        .unwrap_or_else(|_| "restic no encontrado".to_string())
}

/// Ejecuta restic de forma síncrona capturando salida.
/// Devuelve (éxito, salida_combinada).
pub(crate) fn run_restic(repo: &str, args: &[&str], password: &str) -> (bool, String) {
    let out = Command::new("restic")
        .arg("-r")
        .arg(repo)
        .args(args)
        .env("RESTIC_PASSWORD", password)
        .output();

    match out {
        Err(e) => (
            false,
            format!("No se pudo ejecutar restic: {e}\n¿Está instalado? (pacman -S restic)"),
        ),
        Ok(o) => {
            let mut s = String::new();
            let stdout = String::from_utf8_lossy(&o.stdout);
            let stderr = String::from_utf8_lossy(&o.stderr);
            if !stdout.trim().is_empty() {
                s.push_str(&stdout);
            }
            if !stderr.trim().is_empty() {
                if !s.is_empty() {
                    s.push_str("\n──── stderr ────\n");
                }
                s.push_str(&stderr);
            }
            if s.trim().is_empty() {
                s = if o.status.success() {
                    "(sin salida, código 0)".to_string()
                } else {
                    format!("(sin salida, código {})", o.status.code().unwrap_or(-1))
                };
            }
            (o.status.success(), s)
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Snapshot {
    pub(crate) id: String,
    pub(crate) time: String,
    pub(crate) host: String,
    pub(crate) tags: String,
}

pub(crate) fn fetch_snapshots(
    repo: &str,
    password: &str,
) -> std::result::Result<Vec<Snapshot>, String> {
    let out = Command::new("restic")
        .arg("-r")
        .arg(repo)
        .arg("snapshots")
        .arg("--json")
        .env("RESTIC_PASSWORD", password)
        .output()
        .map_err(|e| format!("No se pudo ejecutar restic: {e}"))?;

    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if err.is_empty() {
            "restic devolvió error sin mensaje".to_string()
        } else {
            err
        });
    }

    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value =
        serde_json::from_str(&stdout).map_err(|e| format!("JSON inválido de restic: {e}"))?;
    let arr = v.as_array().cloned().unwrap_or_default();
    let mut snaps = Vec::new();
    for item in arr {
        let id_full = item.get("id").and_then(|x| x.as_str()).unwrap_or("?");
        snaps.push(Snapshot {
            id: id_full.chars().take(8).collect(),
            time: item
                .get("time")
                .and_then(|x| x.as_str())
                .unwrap_or("?")
                .chars()
                .take(19)
                .collect::<String>()
                .replace('T', " "),
            host: item
                .get("hostname")
                .and_then(|x| x.as_str())
                .unwrap_or("-")
                .to_string(),
            tags: item
                .get("tags")
                .and_then(|x| x.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|t| t.as_str())
                        .collect::<Vec<_>>()
                        .join(",")
                })
                .unwrap_or_default(),
        });
    }
    snaps.reverse(); // más recientes primero
    Ok(snaps)
}

pub(crate) struct MountInfo {
    pub(crate) child_pid: u32,
    pub(crate) point: String,
    pub(crate) vault_name: String,
}

pub(crate) fn spawn_mount(
    repo: &str,
    point: &str,
    password: &str,
) -> std::result::Result<Child, String> {
    std::fs::create_dir_all(point).map_err(|e| format!("No se pudo crear {point}: {e}"))?;
    Command::new("restic")
        .arg("-r")
        .arg(repo)
        .arg("mount")
        .arg(point)
        .env("RESTIC_PASSWORD", password)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("No se pudo montar: {e}"))
}

pub(crate) fn unmount(point: &str) {
    // Intenta en cadena (fuse3 usa `fusermount3`, fuse2 `fusermount`);
    // para en el primero que tenga éxito. `umount` es el último recurso
    // (requiere privilegios). El kill del hijo lo remata si nada funciona.
    for (bin, args) in [
        ("fusermount3", vec!["-u", point]),
        ("fusermount", vec!["-u", point]),
        ("umount", vec![point]),
    ] {
        let ok = Command::new(bin)
            .args(&args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok {
            return;
        }
    }
}
