//! Debug page and remote settings rendering.
//! Extracted from ui/mod.rs (Phase 3).

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Cell, Clear, Paragraph, Row, Table,
    },
    Frame,
};

use crate::app::{App, SettingsSection};
use crate::icons::Icon;
use crate::state::AppMode;
use crate::ui::theme as theme;
use crate::ui::theme::{accent_primary, accent_secondary, border_inactive};
use dracon_terminal_engine::layout::centered_rect;
use dracon_terminal_engine::utils::truncate_to_width;

pub fn draw_debug_page(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title_top(Line::from(vec![Span::styled(
            " DEBUG ",
            Style::default()
                .fg(theme::selection_fg())
                .bg(accent_primary())
                .add_modifier(Modifier::BOLD),
        )]))
        .title_top(
            Line::from(vec![
                Span::styled(
                    " Esc ",
                    Style::default()
                        .fg(theme::selection_fg())
                        .bg(theme::accent_primary())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Back ", Style::default().fg(theme::accent_primary())),
            ])
            .alignment(Alignment::Right),
        )
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_inactive()))
        .style(Style::default().bg(theme::bg()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let pane_idx = app.focused_pane_index;
    let (path, filter, remote) = app.current_file_state()
        .map(|fs| {
            (
                fs.nav.current_path.display().to_string(),
                fs.nav.search_filter.clone(),
                fs.nav.remote_session.is_some(),
            )
        })
        .unwrap_or_else(|| ("-".to_string(), "".to_string(), false));

    let lines = vec![
        Line::from(format!("view={:?} mode={:?}", app.core.current_view, app.core.mode)),
        Line::from(format!(
            "pane={} sidebar_focus={}",
            pane_idx, app.sidebar.sidebar_focus
        )),
        Line::from(format!(
            "split={} sidebar={} stage={}",
            app.core.is_split_mode, app.sidebar.show_sidebar, app.show_main_stage
        )),
        Line::from(format!("remote={} filter='{}'", remote, filter)),
        Line::from(format!(
            "path={}",
            truncate_to_width(&path, inner.width.saturating_sub(8) as usize, "...")
        )),
        Line::from("Open/Close: Ctrl+D"),
    ];
    f.render_widget(
        Paragraph::new(lines).style(Style::default().fg(theme::fg())),
        inner,
    );
}

pub fn draw_remote_settings(f: &mut Frame, area: Rect, app: &App) {
    let rows: Vec<_> = app.remote.remote_bookmarks
        .iter()
        .enumerate()
        .map(|(i, b)| {
            let is_selected =
                i == app.settings.settings_index && app.settings.settings_section == SettingsSection::Remotes;
            let mut style = Style::default().fg(theme::fg());
            if is_selected {
                style = style
                    .bg(accent_primary())
                    .fg(theme::selection_fg())
                    .add_modifier(Modifier::BOLD);
            }

            let icon = Icon::Remote.get(app.core.icon_mode);
            Row::new(vec![
                Cell::from(format!(" {} {}", icon, b.name)).style(style),
                Cell::from(format!("{}@{}", b.user, b.host)).style(style),
                Cell::from(b.port.to_string()).style(style),
                Cell::from(b.last_path.to_string_lossy().to_string()).style(style),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Length(6),
            Constraint::Fill(1),
        ],
    )
    .header(
        Row::new(vec![" NAME ", " CONNECTION ", " PORT ", " LAST PATH "]).style(
            Style::default()
                .fg(accent_secondary())
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::default()
            .title(" REMOTE SERVER BOOKMARKS ")
            .borders(Borders::TOP)
            .border_style(Style::default().fg(theme::border_subtle())),
    )
    .column_spacing(2);

    let text = vec![
        Line::from("Manage your remote server bookmarks here."),
        Line::from(vec![
            Span::raw("Tip: Import servers by clicking "),
            Span::styled(
                " REMOTES [Import] ",
                Style::default()
                    .fg(accent_secondary())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" in the sidebar."),
        ]),
        Line::from("Format (TOML): [[servers]] name=\"...\" host=\"...\" user=\"...\" port=22"),
        Line::from(""),
    ];

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Min(0)])
        .split(area);

    f.render_widget(Paragraph::new(text), chunks[0]);

    if app.remote.remote_bookmarks.is_empty() {
        f.render_widget(
            Paragraph::new("\n (No remote servers configured)")
                .style(Style::default().fg(theme::muted())),
            chunks[1],
        );
    } else {
        f.render_widget(table, chunks[1]);
    }
}

pub fn draw_add_remote_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 50, f.area());
    f.render_widget(Clear, area);
    let block = Block::default()
        .title(" Add Remote Server ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::success()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Name
            Constraint::Length(3), // Host
            Constraint::Length(3), // User
            Constraint::Length(3), // Port
            Constraint::Length(3), // Key Path
            Constraint::Min(0),    // Help
        ])
        .split(inner);

    let active_idx = if let AppMode::AddRemote(idx) = app.core.mode {
        idx
    } else {
        0
    };

    let fields = [
        ("Name", &app.remote.pending_remote.name),
        ("Host", &app.remote.pending_remote.host),
        ("User", &app.remote.pending_remote.user),
        ("Port", &app.remote.pending_remote.port.to_string()),
        (
            "Key Path",
            &app.remote.pending_remote
                .key_path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default(),
        ),
    ];

    for (i, (label, value)) in fields.iter().enumerate() {
        let is_active = i == active_idx;
        let mut style = Style::default().fg(theme::muted());
        if is_active {
            style = Style::default().fg(theme::warning());
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", label))
            .border_style(style);
        let field_area = chunks[i];

        if is_active {
            f.render_widget(
                Paragraph::new(app.core.input.value.as_str()).block(block),
                field_area,
            );
        } else {
            f.render_widget(Paragraph::new(value.as_str()).block(block), field_area);
        }
    }

    let help_text = vec![
        Line::from(vec![
            Span::styled(
                " [Tab/Enter] ",
                Style::default()
                    .fg(theme::info())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Next Field  "),
            Span::styled(
                " [Esc] ",
                Style::default().fg(theme::accent_primary()).add_modifier(Modifier::BOLD),
            ),
            Span::raw("Cancel"),
        ]),
        Line::from("On the last field, [Enter] will save the bookmark."),
    ];
    f.render_widget(Paragraph::new(help_text), chunks[5]);
}
