//! Renderizado ratatui: pantallas y popups.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, List, ListItem, Paragraph, Row, Table, Wrap},
};

use crate::app::{App, FormMode, Overlay, Screen, FORM_LABELS};

fn centered_popup(area: Rect, w_pct: u16, h: u16) -> Rect {
    let w = area.width.min(100) * w_pct / 100;
    let h = h.min(area.height);
    let x = area.x + area.width.saturating_sub(w) / 2;
    let y = area.y + area.height.saturating_sub(h) / 2;
    Rect::new(x, y, w, h)
}

fn footer_hints(app: &App) -> Line<'static> {
    let hints: Vec<(&str, &str)> = match (app.screen, app.overlay) {
        (_, Overlay::Log) => vec![("↑/↓", "scroll"), ("Esc", "cerrar")],
        (_, Overlay::Password) => vec![("Enter", "ejecutar"), ("Esc", "cancelar")],
        (_, Overlay::Form) => vec![("Tab", "campo"), ("Enter", "guardar"), ("Esc", "cancelar")],
        (_, Overlay::ConfirmDelete) => vec![("y", "eliminar"), ("n/Esc", "cancelar")],
        (_, Overlay::ConfirmPull) => vec![("y", "restaurar"), ("n/Esc", "cancelar")],
        (_, Overlay::MountInput) => vec![("Enter", "montar"), ("Esc", "cancelar")],
        (Screen::List, Overlay::None) => vec![
            ("Enter", "abrir"),
            ("n", "nueva"),
            ("e", "editar"),
            ("d", "eliminar"),
            ("q", "salir"),
        ],
        (Screen::Menu, Overlay::None) => vec![
            ("b", "push"),
            ("u", "pull"),
            ("h", "history"),
            ("i", "init"),
            ("m", "mount"),
            ("U", "unmount"),
            ("Esc", "atrás"),
        ],
        (Screen::Snapshots, Overlay::None) => vec![("↑/↓", "navegar"), ("Esc", "atrás")],
    };
    let mut spans = Vec::new();
    for (i, (k, desc)) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(
            *k,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(format!(" {desc}")));
    }
    Line::from(spans)
}

pub(crate) fn draw(f: &mut ratatui::Frame, app: &mut App) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    // ── Header ──
    let restic_style = if app.restic_ok {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    };
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            " aegis ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  bóvedas restic!   "),
        Span::styled(format!("restic: {}", app.restic_ver), restic_style),
    ]))
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, chunks[0]);

    // ── Body ──
    match app.screen {
        Screen::List => draw_list(f, app, chunks[1]),
        Screen::Menu => draw_menu(f, app, chunks[1]),
        Screen::Snapshots => draw_snapshots(f, app, chunks[1]),
    }

    // ── Status + footer ──
    let status_style = if app.status.contains("no instalado")
        || app.status.contains("falló")
        || app.status.contains("FALLÓ")
    {
        Style::default().fg(Color::Red)
    } else {
        Style::default().fg(Color::Green)
    };
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            format!(" {}", app.status),
            status_style,
        )])),
        chunks[2],
    );
    f.render_widget(Paragraph::new(footer_hints(app)), chunks[3]);

    // ── Overlays ──
    match app.overlay {
        Overlay::Password => draw_password(f, app, area),
        Overlay::Log => draw_log(f, app, area),
        Overlay::Form => draw_form(f, app, area),
        Overlay::MountInput => draw_mount_input(f, app, area),
        Overlay::ConfirmDelete => draw_confirm(f, app, area),
        Overlay::ConfirmPull => draw_confirm_pull(f, app, area),
        Overlay::None => {}
    }
}

fn draw_list(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(area);

    let items: Vec<ListItem> = if app.cfg.vaults.is_empty() {
        vec![ListItem::new(Line::from(vec![Span::styled(
            "  (sin bóvedas — pulsa n para crear una)",
            Style::default().fg(Color::DarkGray),
        )]))]
    } else {
        app.cfg
            .vaults
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let marker = if Some(&v.name) == app.mount.as_ref().map(|m| &m.vault_name) {
                    " ●"
                } else {
                    ""
                };
                let style = if i == app.selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(Line::from(vec![Span::styled(
                    format!(" {}{}", v.name, marker),
                    style,
                )]))
            })
            .collect()
    };
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Bóvedas "))
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol("> ");
    let mut state = ratatui::widgets::ListState::default();
    if !app.cfg.vaults.is_empty() {
        state.select(Some(app.selected));
    }
    f.render_stateful_widget(list, cols[0], &mut state);

    // Detalle
    let detail_text = match app.vault() {
        None => vec![Line::from("(nada seleccionado)")],
        Some(v) => vec![
            Line::from(vec![
                Span::styled("Nombre: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(v.name.clone()),
            ]),
            Line::from(vec![
                Span::styled("Local:  ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(v.local_path.clone()),
            ]),
            Line::from(vec![
                Span::styled("Repo:   ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(v.repo_target.clone()),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                app.mount
                    .as_ref()
                    .map(|m| format!("Montado en {}", m.point))
                    .unwrap_or_else(|| "No montado".to_string()),
                Style::default().fg(Color::Cyan),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Enter para operar con esta bóveda.",
                Style::default().fg(Color::DarkGray),
            )]),
        ],
    };
    f.render_widget(
        Paragraph::new(detail_text)
            .block(Block::default().borders(Borders::ALL).title(" Detalle ")),
        cols[1],
    );
}

pub(crate) const MENU_ITEMS: [(&str, &str); 5] = [
    ("b", "Push   — backup local → repo"),
    ("u", "Pull   — restore latest → ubicación original"),
    ("h", "History — ver snapshots"),
    ("i", "Init   — inicializar repo nuevo"),
    ("m", "Mount  — montar repo como disco"),
];

fn draw_menu(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let Some(v) = app.vault().cloned() else {
        f.render_widget(
            Paragraph::new("Sin bóveda.").block(Block::default().borders(Borders::ALL)),
            area,
        );
        return;
    };
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(area);

    let items: Vec<ListItem> = MENU_ITEMS
        .iter()
        .enumerate()
        .map(|(i, (k, desc))| {
            let style = if i == app.menu_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!(" [{k}] "), style),
                Span::styled(*desc, style),
            ]))
        })
        .collect();
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} — operaciones ", v.name)),
        )
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol("> ");
    let mut state = ratatui::widgets::ListState::default();
    state.select(Some(app.menu_selected));
    f.render_stateful_widget(list, cols[0], &mut state);

    let info = vec![
        Line::from(vec![
            Span::styled("Local: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(v.local_path.clone()),
        ]),
        Line::from(vec![
            Span::styled("Repo:  ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(v.repo_target.clone()),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            app.mount
                .as_ref()
                .map(|m| format!("● montado en {}", m.point))
                .unwrap_or_else(|| "○ no montado".to_string()),
            Style::default().fg(Color::Cyan),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "↑/↓ + Enter o atajo directo.",
            Style::default().fg(Color::DarkGray),
        )]),
        Line::from(vec![Span::styled(
            "U desmonta · Esc vuelve.",
            Style::default().fg(Color::DarkGray),
        )]),
    ];
    f.render_widget(
        Paragraph::new(info).block(Block::default().borders(Borders::ALL).title(" Info ")),
        cols[1],
    );
}

fn draw_snapshots(f: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let name = app.vault().map(|v| v.name.clone()).unwrap_or_default();
    let rows: Vec<Row> = app
        .snaps
        .iter()
        .map(|s| {
            Row::new(vec![
                Cell::from(s.id.clone()),
                Cell::from(s.time.clone()),
                Cell::from(s.host.clone()),
                Cell::from(s.tags.clone()),
            ])
        })
        .collect();
    let widths = [
        Constraint::Length(10),
        Constraint::Length(20),
        Constraint::Length(18),
        Constraint::Min(10),
    ];
    let table = Table::new(rows, widths)
        .header(
            Row::new(vec!["ID", "FECHA", "HOST", "TAGS"]).style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .block(Block::default().borders(Borders::ALL).title(format!(
            " Snapshots — {} ({}), Esc atrás ",
            name,
            app.snaps.len()
        )))
        .row_highlight_style(Style::default().bg(Color::DarkGray));
    if app.snaps.is_empty() {
        f.render_widget(
            Paragraph::new("Sin snapshots.")
                .block(Block::default().borders(Borders::ALL).title(" Snapshots ")),
            area,
        );
    } else {
        f.render_stateful_widget(table, area, &mut app.snap_state);
    }
}

fn draw_password(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let popup = centered_popup(area, 60, 7);
    f.render_widget(Clear, popup);
    let action = app.pending.map(|p| p.label()).unwrap_or("restic");
    let masked = "*".repeat(app.password_buf.len());
    let p = Paragraph::new(vec![
        Line::from(vec![Span::styled(
            format!(" Password para: {action} "),
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::raw(" › "),
            Span::raw(masked),
            Span::styled("▌", Style::default().fg(Color::Yellow)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Enter ejecuta · Esc cancela · no se guarda en disco",
            Style::default().fg(Color::DarkGray),
        )]),
    ])
    .block(Block::default().borders(Borders::ALL).title(" 🔑 "));
    f.render_widget(p, popup);
}

fn draw_log(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let popup = centered_popup(area, 85, area.height.saturating_sub(6).max(10));
    f.render_widget(Clear, popup);
    let visible_h = popup.height.saturating_sub(4) as usize;
    let max_scroll = app.log_lines.len().saturating_sub(visible_h.max(1));
    let scroll = app.log_scroll.min(max_scroll);
    let end = (scroll + visible_h).min(app.log_lines.len());
    let text: Vec<Line> = app.log_lines[scroll..end.max(scroll)]
        .iter()
        .map(|l| Line::from(l.clone()))
        .collect();
    let p = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title(format!(
            " {}  ({}/{}) ",
            app.log_title,
            end,
            app.log_lines.len()
        )))
        .wrap(Wrap { trim: false });
    f.render_widget(p, popup);
}

fn draw_form(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let popup = centered_popup(area, 70, 12);
    f.render_widget(Clear, popup);
    let title = match app.form_mode {
        FormMode::Create => " Nueva bóveda ",
        FormMode::Edit => " Editar bóveda ",
    };
    let inner = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(1),
        ])
        .split(popup);
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            title,
            Style::default().add_modifier(Modifier::BOLD),
        )])),
        inner[0],
    );
    for (i, label) in FORM_LABELS.iter().enumerate() {
        let active = i == app.form_idx;
        let style = if active {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let cursor = if active { "▌" } else { "" };
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!("{label}: "), style),
                Span::raw(format!("{}{}", app.form_fields[i], cursor)),
            ]))
            .block(Block::default().borders(Borders::BOTTOM)),
            inner[i + 1],
        );
    }
    f.render_widget(
        Paragraph::new("Tab cambia campo · Enter guarda · Esc cancela"),
        inner[4],
    );
}

fn draw_mount_input(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let popup = centered_popup(area, 65, 7);
    f.render_widget(Clear, popup);
    let p = Paragraph::new(vec![
        Line::from(vec![Span::styled(
            " Punto de montaje ",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::raw(" › "),
            Span::raw(app.mount_input.clone()),
            Span::styled("▌", Style::default().fg(Color::Yellow)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Enter continúa → pedirá password · Esc cancela",
            Style::default().fg(Color::DarkGray),
        )]),
    ])
    .block(Block::default().borders(Borders::ALL).title(" 💾 "));
    f.render_widget(p, popup);
}

fn draw_confirm(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let popup = centered_popup(area, 50, 6);
    f.render_widget(Clear, popup);
    let name = app.vault().map(|v| v.name.clone()).unwrap_or_default();
    f.render_widget(
        Paragraph::new(vec![
            Line::from(vec![Span::styled(
                format!("¿Eliminar «{name}»?"),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from("Pulsa y para confirmar, n/Esc para cancelar."),
        ])
        .block(Block::default().borders(Borders::ALL).title(" ⚠ ")),
        popup,
    );
}

fn draw_confirm_pull(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let popup = centered_popup(area, 60, 8);
    f.render_widget(Clear, popup);
    let local = app
        .vault()
        .map(|v| v.local_path.clone())
        .unwrap_or_default();
    f.render_widget(
        Paragraph::new(vec![
            Line::from(vec![Span::styled(
                "¿Restaurar latest en su ubicación original?",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::styled(
                format!("Sobrescribirá: {local}"),
                Style::default().fg(Color::Red),
            )]),
            Line::from(""),
            Line::from("Pulsa y para continuar, n/Esc para cancelar."),
        ])
        .block(Block::default().borders(Borders::ALL).title(" ⚠ pull ")),
        popup,
    );
}
