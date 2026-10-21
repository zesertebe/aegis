//! Estado de la aplicacion (pantallas, overlays, operaciones).

use std::process::{Command, Stdio};
use std::time::Duration;

use ratatui::widgets::TableState;

use crate::config::{expand_tilde, load_config, save_config, AppConfig, Vault};
use crate::restic::{
    fetch_snapshots, restic_exists, restic_version, run_restic, spawn_mount, unmount, MountInfo,
    Snapshot,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Screen {
    List,
    Menu,
    Snapshots,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FormMode {
    Create,
    Edit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PendingAction {
    Push,
    Pull,
    Init,
    History,
    Mount,
}

impl PendingAction {
    pub(crate) fn label(self) -> &'static str {
        match self {
            PendingAction::Push => "Push (backup)",
            PendingAction::Pull => "Pull (restore latest)",
            PendingAction::Init => "Init repo",
            PendingAction::History => "History (snapshots)",
            PendingAction::Mount => "Mount",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Overlay {
    None,
    Password,
    MountInput,
    Log,
    Form,
    ConfirmDelete,
    ConfirmPull,
}

pub(crate) const FORM_LABELS: [&str; 3] = ["Nombre", "Ruta local", "Repo restic"];

pub(crate) struct App {
    pub(crate) cfg: AppConfig,
    pub(crate) screen: Screen,
    pub(crate) overlay: Overlay,
    pub(crate) selected: usize,
    pub(crate) menu_selected: usize,
    pub(crate) status: String,
    pub(crate) restic_ok: bool,
    pub(crate) restic_ver: String,
    // overlays
    pub(crate) password_buf: String,
    pub(crate) pending: Option<PendingAction>,
    pub(crate) log_title: String,
    pub(crate) log_lines: Vec<String>,
    pub(crate) log_scroll: usize,
    pub(crate) mount_input: String,
    pub(crate) mount: Option<MountInfo>,
    // form
    pub(crate) form_mode: FormMode,
    pub(crate) form_fields: [String; 3],
    pub(crate) form_idx: usize,
    // snapshots
    pub(crate) snaps: Vec<Snapshot>,
    pub(crate) snap_state: TableState,
    pub(crate) should_quit: bool,
}

impl App {
    pub(crate) fn new() -> Self {
        let restic_ok = restic_exists();
        let restic_ver = if restic_ok {
            restic_version()
        } else {
            "NO ENCONTRADO".to_string()
        };
        let status = if restic_ok {
            "Listo. Pulsa ? o mira el pie para atajos.".to_string()
        } else {
            " restic no instalado — instala con: sudo pacman -S restic ".to_string()
        };
        Self {
            cfg: load_config(),
            screen: Screen::List,
            overlay: Overlay::None,
            selected: 0,
            menu_selected: 0,
            status,
            restic_ok,
            restic_ver,
            password_buf: String::new(),
            pending: None,
            log_title: String::new(),
            log_lines: Vec::new(),
            log_scroll: 0,
            mount_input: String::new(),
            mount: None,
            form_mode: FormMode::Create,
            form_fields: [String::new(), String::new(), String::new()],
            form_idx: 0,
            snaps: Vec::new(),
            snap_state: TableState::default(),
            should_quit: false,
        }
    }

    pub(crate) fn vault(&self) -> Option<&Vault> {
        self.cfg.vaults.get(self.selected)
    }

    pub(crate) fn persist(&mut self) {
        if let Err(e) = save_config(&self.cfg) {
            self.status = format!("Error guardando config: {e:#}");
        }
    }

    pub(crate) fn open_form(&mut self, mode: FormMode) {
        self.form_mode = mode;
        self.form_idx = 0;
        if mode == FormMode::Edit {
            if let Some(v) = self.vault() {
                self.form_fields = [v.name.clone(), v.local_path.clone(), v.repo_target.clone()];
            } else {
                return;
            }
        } else {
            self.form_fields = [String::new(), String::new(), String::new()];
        }
        self.overlay = Overlay::Form;
    }

    pub(crate) fn save_form(&mut self) {
        let name = self.form_fields[0].trim().to_string();
        let local = self.form_fields[1].trim().to_string();
        let repo = self.form_fields[2].trim().to_string();
        if name.is_empty() || local.is_empty() || repo.is_empty() {
            self.status = "Todos los campos son obligatorios.".to_string();
            return;
        }
        let v = Vault {
            name,
            local_path: local,
            repo_target: repo,
            password_env: None,
        };
        match self.form_mode {
            FormMode::Create => {
                self.cfg.vaults.push(v);
                self.selected = self.cfg.vaults.len().saturating_sub(1);
                self.status = "Bóveda creada.".to_string();
            }
            FormMode::Edit => {
                if let Some(slot) = self.cfg.vaults.get_mut(self.selected) {
                    *slot = v;
                    self.status = "Bóveda actualizada.".to_string();
                }
            }
        }
        self.persist();
        self.overlay = Overlay::None;
    }

    pub(crate) fn delete_selected(&mut self) {
        if self.selected < self.cfg.vaults.len() {
            let name = self.cfg.vaults[self.selected].name.clone();
            self.cfg.vaults.remove(self.selected);
            if self.selected >= self.cfg.vaults.len() {
                self.selected = self.cfg.vaults.len().saturating_sub(1);
            }
            self.persist();
            self.status = format!("Bóveda «{name}» eliminada.");
        }
        self.overlay = Overlay::None;
    }

    pub(crate) fn show_log(&mut self, title: String, body: String) {
        self.log_title = title;
        self.log_lines = body.lines().map(|l| l.to_string()).collect();
        if self.log_lines.is_empty() {
            self.log_lines.push("(vacío)".to_string());
        }
        self.log_scroll = 0;
        self.overlay = Overlay::Log;
    }

    pub(crate) fn require_password(&mut self, action: PendingAction) {
        if !self.restic_ok {
            self.status = "restic no está instalado: sudo pacman -S restic".to_string();
            return;
        }
        if self.vault().is_none() {
            self.status = "No hay bóveda seleccionada.".to_string();
            return;
        }
        self.pending = Some(action);
        self.password_buf.clear();
        self.overlay = Overlay::Password;
    }

    pub(crate) fn exec_pending(&mut self) {
        let Some(action) = self.pending else { return };
        let Some(v) = self.vault().cloned() else {
            return;
        };
        let password = std::mem::take(&mut self.password_buf);
        self.pending = None;
        if password.is_empty() {
            self.status = "Password vacío, operación cancelada.".to_string();
            self.overlay = Overlay::None;
            return;
        }
        let repo = expand_tilde(&v.repo_target);
        let local = expand_tilde(&v.local_path);

        match action {
            PendingAction::Push => {
                self.status = "Ejecutando backup…".to_string();
                let (ok, out) = run_restic(&repo, &["backup", &local], &password);
                let title = if ok {
                    format!("Push OK — {}", v.name)
                } else {
                    format!("Push FALLÓ — {}", v.name)
                };
                self.show_log(title, out);
                self.status = if ok {
                    "Backup completado.".to_string()
                } else {
                    "Backup falló, ver log.".to_string()
                };
            }
            PendingAction::Pull => {
                // El backup guarda rutas absolutas, así que el inverso exacto
                // es restaurar contra `/`: los archivos vuelven a su sitio
                // original en lugar de anidarse bajo la carpeta.
                self.status = "Ejecutando restore…".to_string();
                let (ok, out) =
                    run_restic(&repo, &["restore", "latest", "--target", "/"], &password);
                let title = if ok {
                    format!("Pull OK — {}", v.name)
                } else {
                    format!("Pull FALLÓ — {}", v.name)
                };
                self.show_log(title, out);
                self.status = if ok {
                    "Restore completado.".to_string()
                } else {
                    "Restore falló, ver log.".to_string()
                };
            }
            PendingAction::Init => {
                let (ok, out) = run_restic(&repo, &["init"], &password);
                let title = if ok {
                    format!("Init OK — {}", v.name)
                } else {
                    format!("Init FALLÓ — {}", v.name)
                };
                self.show_log(title, out);
            }
            PendingAction::History => match fetch_snapshots(&repo, &password) {
                Ok(snaps) => {
                    self.snaps = snaps;
                    self.snap_state
                        .select(if self.snaps.is_empty() { None } else { Some(0) });
                    self.screen = Screen::Snapshots;
                    self.overlay = Overlay::None;
                    self.status = format!("{} snapshots.", self.snaps.len());
                }
                Err(e) => {
                    self.show_log(format!("History FALLÓ — {}", v.name), e);
                }
            },
            PendingAction::Mount => {
                let point = if self.mount_input.trim().is_empty() {
                    format!("/tmp/aegis-{}", v.name.replace(' ', "-").to_lowercase())
                } else {
                    expand_tilde(self.mount_input.trim())
                };
                match spawn_mount(&repo, &point, &password) {
                    Ok(child) => {
                        let pid = child.id();
                        self.mount = Some(MountInfo {
                            child_pid: pid,
                            point: point.clone(),
                            vault_name: v.name.clone(),
                        });
                        // Soltamos el handle; el proceso sigue vivo. Guardamos pid.
                        std::mem::forget(child);
                        self.overlay = Overlay::None;
                        self.status =
                            format!("Montado en {point} (pid {pid}). Pulsa U para desmontar.");
                    }
                    Err(e) => {
                        self.show_log(format!("Mount FALLÓ — {}", v.name), e);
                    }
                }
            }
        }
    }

    pub(crate) fn do_unmount(&mut self) {
        if let Some(m) = self.mount.take() {
            unmount(&m.point);
            // Intenta matar el proceso hijo por pid (best-effort, Linux).
            let _ = Command::new("kill")
                .args([m.child_pid.to_string()])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            // Espera un poco y reintenta kill -9.
            std::thread::sleep(Duration::from_millis(300));
            let _ = Command::new("kill")
                .args(["-9", &m.child_pid.to_string()])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            unmount(&m.point);
            self.status = format!("Desmontado {} (pid {}).", m.point, m.child_pid);
        } else {
            self.status = "No hay ningún mount activo.".to_string();
        }
    }
}
