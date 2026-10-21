//! Manejo de teclado estilo vim.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::app::{App, FormMode, Overlay, PendingAction, Screen};
use crate::ui::MENU_ITEMS;

fn menu_action_at(idx: usize) -> Option<PendingAction> {
    match idx {
        0 => Some(PendingAction::Push),
        1 => Some(PendingAction::Pull),
        2 => Some(PendingAction::History),
        3 => Some(PendingAction::Init),
        4 => Some(PendingAction::Mount),
        _ => None,
    }
}

fn trigger_menu_action(app: &mut App, action: PendingAction) {
    match action {
        PendingAction::Mount => {
            if !app.restic_ok {
                app.status = "restic no está instalado: sudo pacman -S restic".to_string();
                return;
            }
            if app.vault().is_none() {
                return;
            }
            if app.mount.is_some() {
                app.status = "Ya hay un mount activo. Pulsa U para desmontar.".to_string();
                return;
            }
            let v = app.vault().cloned().unwrap();
            app.mount_input = format!("/tmp/aegis-{}", v.name.replace(' ', "-").to_lowercase());
            app.pending = Some(PendingAction::Mount);
            app.overlay = Overlay::MountInput;
        }
        PendingAction::Pull => {
            if !app.restic_ok {
                app.status = "restic no está instalado: sudo pacman -S restic".to_string();
                return;
            }
            if app.vault().is_none() {
                app.status = "No hay bóveda seleccionada.".to_string();
                return;
            }
            // El restore escribe en las rutas originales: confirmar primero.
            app.pending = Some(PendingAction::Pull);
            app.overlay = Overlay::ConfirmPull;
        }
        other => app.require_password(other),
    }
}

pub(crate) fn handle_key(app: &mut App, code: KeyCode, mods: KeyModifiers) {
    // Ctrl-C / Ctrl-Q siempre sale.
    if mods.contains(KeyModifiers::CONTROL)
        && matches!(code, KeyCode::Char('c') | KeyCode::Char('q'))
    {
        app.should_quit = true;
        return;
    }

    match app.overlay {
        Overlay::Password => match code {
            KeyCode::Esc => {
                app.overlay = Overlay::None;
                app.pending = None;
                app.password_buf.clear();
            }
            KeyCode::Enter => app.exec_pending(),
            KeyCode::Backspace => {
                app.password_buf.pop();
            }
            KeyCode::Char(c) => app.password_buf.push(c),
            _ => {}
        },
        Overlay::Log => match code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => {
                app.overlay = Overlay::None;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                app.log_scroll = app.log_scroll.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.log_scroll = app.log_scroll.saturating_add(1);
            }
            _ => {}
        },
        Overlay::Form => match code {
            KeyCode::Esc => app.overlay = Overlay::None,
            KeyCode::Tab | KeyCode::Down => {
                app.form_idx = (app.form_idx + 1) % 3;
            }
            KeyCode::Up => {
                app.form_idx = (app.form_idx + 2) % 3;
            }
            KeyCode::Enter => app.save_form(),
            KeyCode::Backspace => {
                app.form_fields[app.form_idx].pop();
            }
            KeyCode::Char(c)
                if !mods.contains(KeyModifiers::CONTROL) && !mods.contains(KeyModifiers::ALT) =>
            {
                app.form_fields[app.form_idx].push(c);
            }
            _ => {}
        },
        Overlay::MountInput => match code {
            KeyCode::Esc => {
                app.overlay = Overlay::None;
                app.pending = None;
            }
            KeyCode::Enter => {
                // paso 2: pedir password
                app.overlay = Overlay::Password;
                app.password_buf.clear();
            }
            KeyCode::Backspace => {
                app.mount_input.pop();
            }
            KeyCode::Char(c) => app.mount_input.push(c),
            _ => {}
        },
        Overlay::ConfirmDelete => match code {
            KeyCode::Char('y') | KeyCode::Char('Y') => app.delete_selected(),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                app.overlay = Overlay::None;
            }
            _ => {}
        },
        Overlay::ConfirmPull => match code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                app.require_password(PendingAction::Pull);
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                app.overlay = Overlay::None;
                app.pending = None;
            }
            _ => {}
        },
        Overlay::None => match app.screen {
            Screen::List => match code {
                KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
                KeyCode::Down | KeyCode::Char('j') => {
                    if !app.cfg.vaults.is_empty() {
                        app.selected = (app.selected + 1) % app.cfg.vaults.len();
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if !app.cfg.vaults.is_empty() {
                        app.selected = app
                            .selected
                            .checked_sub(1)
                            .unwrap_or(app.cfg.vaults.len().saturating_sub(1));
                    }
                }
                KeyCode::Enter => {
                    if app.vault().is_some() {
                        app.screen = Screen::Menu;
                        app.menu_selected = 0;
                    } else {
                        app.status = "No hay bóvedas. Pulsa n para crear una.".to_string();
                    }
                }
                KeyCode::Char('n') => app.open_form(FormMode::Create),
                KeyCode::Char('e') => {
                    if app.vault().is_some() {
                        app.open_form(FormMode::Edit);
                    }
                }
                KeyCode::Char('d') if app.vault().is_some() => {
                    app.overlay = Overlay::ConfirmDelete;
                }
                _ => {}
            },
            Screen::Menu => match code {
                KeyCode::Esc => app.screen = Screen::List,
                KeyCode::Char('q') => app.should_quit = true,
                KeyCode::Up | KeyCode::Char('k') => {
                    app.menu_selected = app
                        .menu_selected
                        .checked_sub(1)
                        .unwrap_or(MENU_ITEMS.len() - 1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    app.menu_selected = (app.menu_selected + 1) % MENU_ITEMS.len();
                }
                KeyCode::Enter => {
                    if let Some(a) = menu_action_at(app.menu_selected) {
                        trigger_menu_action(app, a);
                    }
                }
                KeyCode::Char('b') => trigger_menu_action(app, PendingAction::Push),
                KeyCode::Char('u') => trigger_menu_action(app, PendingAction::Pull),
                KeyCode::Char('h') => trigger_menu_action(app, PendingAction::History),
                KeyCode::Char('i') => trigger_menu_action(app, PendingAction::Init),
                KeyCode::Char('m') => trigger_menu_action(app, PendingAction::Mount),
                KeyCode::Char('U') => app.do_unmount(),
                KeyCode::Char('e') => app.open_form(FormMode::Edit),
                _ => {}
            },
            Screen::Snapshots => match code {
                KeyCode::Esc => app.screen = Screen::Menu,
                KeyCode::Char('q') => app.should_quit = true,
                KeyCode::Up | KeyCode::Char('k') => {
                    let i = app.snap_state.selected().unwrap_or(0);
                    if i > 0 {
                        app.snap_state.select(Some(i - 1));
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    let i = app.snap_state.selected().unwrap_or(0);
                    if i + 1 < app.snaps.len() {
                        app.snap_state.select(Some(i + 1));
                    }
                }
                _ => {}
            },
        },
    }
}
