#![allow(unused_imports)]

//! Small modals — signal select, drag-drop, hotkeys, open-with.
//! Extracted from ui/mod.rs (Phase 3).

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, Borders, Cell, Clear, List, ListItem, ListState, Paragraph, Row, Table, TableState, Widget,
    },
    Frame,
};
use std::path::Path;

use crate::app::App;
use crate::icons::Icon;
use crate::ui::theme as theme;
use dracon_terminal_engine::layout::centered_rect;
use dracon_terminal_engine::utils::truncate_to_width;

pub fn draw_signal_select_modal(f: &mut Frame, _app: &App, pid: u32, name: &str, selected_index: usize) {
    let signals: [(i32, &str); 6] = [
        (1, "SIGHUP"),
        (2, "SIGINT"),
        (9, "SIGKILL"),
        (15, "SIGTERM"),
        (18, "SIGCONT"),
        (19, "SIGSTOP"),
    ];
    let area = centered_rect(50, 60, f.area());
    f.render_widget(Clear, area);
    let block = Block::default()
        .title(" Send Signal ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::warning()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut text = Vec::new();
    text.push(Line::from(vec![
        Span::styled(" PID ", Style::default().fg(theme::selection_fg()).bg(theme::accent_secondary()).add_modifier(Modifier::BOLD)),
        Span::styled(format!(" {} ", pid), Style::default().fg(theme::fg())),
        Span::styled(name, Style::default().fg(theme::fg()).add_modifier(Modifier::BOLD)),
    ]));
    text.push(Line::from(""));

    for (i, (sig, sig_name)) in signals.iter().enumerate() {
        let is_selected = i == selected_index;
        let style = if is_selected {
            Style::default().bg(theme::accent_primary()).fg(theme::selection_fg()).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::fg())
        };
        let danger_style = if is_selected {
            Style::default().bg(theme::accent_primary()).fg(theme::selection_fg()).add_modifier(Modifier::BOLD)
        } else if *sig == 9 {
            Style::default().fg(theme::danger())
        } else {
            Style::default().fg(theme::fg())
        };
        text.push(Line::from(vec![
            Span::styled(format!("  {} ", if is_selected { "▸" } else { " " }), style),
            Span::styled(format!("{:>2}", sig), danger_style),
            Span::styled(format!("  {:<10}", sig_name), style),
        ]));
    }

    text.push(Line::from(""));
    text.push(Line::from(vec![
        Span::styled("  Enter ", Style::default().fg(theme::muted())),
        Span::styled("Send", Style::default().fg(theme::accent_secondary())),
        Span::raw("    "),
        Span::styled(" Esc ", Style::default().fg(theme::muted())),
        Span::styled("Cancel", Style::default().fg(theme::accent_primary())),
    ]));

    f.render_widget(Paragraph::new(text), inner);
}

pub fn draw_drag_drop_modal(
    f: &mut Frame,
    app: &App,
    sources: &[std::path::PathBuf],
    target: &std::path::Path,
) {
    let area = centered_rect(60, 20, f.area());
    f.render_widget(Clear, area);
    let block = Block::default()
        .title(" Choice Action ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::warning()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let dest_path = target.to_string_lossy();

    // Calculate correct button offset based on content
    let button_y_offset = if sources.len() == 1 {
        3
    } else {
        let display_count = std::cmp::min(sources.len(), 3);
        let mut offset = 1 + display_count;
        if sources.len() > 3 {
            offset += 1;
        }
        offset + 2 // + To: line + spacing line
    };

    let (mx, my) = app.core.mouse_pos;

    let is_hover = |bx: u16, len: u16| {
        mx >= inner.x + bx && mx < inner.x + bx + len && my == inner.y + button_y_offset as u16
    };

    let copy_style = if is_hover(0, 10) {
        Style::default().bg(theme::success()).fg(theme::selection_fg())
    } else {
        Style::default().fg(theme::success())
    };
    let move_style = if is_hover(12, 10) {
        Style::default().bg(theme::warning()).fg(theme::selection_fg())
    } else {
        Style::default().fg(theme::warning())
    };
    let link_style = if is_hover(24, 10) {
        Style::default().bg(theme::accent_secondary()).fg(theme::selection_fg())
    } else {
        Style::default().fg(theme::accent_secondary())
    };
    let cancel_style = if is_hover(36, 14) {
        Style::default().bg(theme::accent_primary()).fg(theme::selection_fg())
    } else {
        Style::default().fg(theme::accent_primary())
    };

    let mut text = Vec::new();

    if sources.len() == 1 {
        let src_name = sources[0].file_name().unwrap_or_default().to_string_lossy();
        text.push(Line::from(vec![
            Span::raw("Item: "),
            Span::styled(
                src_name,
                Style::default()
                    .fg(theme::info())
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
    } else {
        text.push(Line::from(vec![
            Span::raw("Items: "),
            Span::styled(
                format!("{} files/folders", sources.len()),
                Style::default()
                    .fg(theme::info())
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        // List first few items
        for source in sources.iter().take(std::cmp::min(sources.len(), 3)) {
            let name = source.file_name().unwrap_or_default().to_string_lossy();
            text.push(Line::from(vec![
                Span::raw("  - "),
                Span::styled(name, Style::default().fg(theme::muted())),
            ]));
        }
        if sources.len() > 3 {
            text.push(Line::from(vec![Span::raw("  ... ")]));
        }
    }

    text.push(Line::from(vec![
        Span::raw("To:    "),
        Span::styled(
            truncate_to_width(&dest_path, (inner.width as usize).saturating_sub(7), "..."),
            Style::default().fg(theme::info()),
        ),
    ]));

    // Spacing
    text.push(Line::from(""));

    text.push(Line::from(vec![
        Span::styled(" [C] Copy ", copy_style.add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(" [M] Move ", move_style.add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(" [L] Link ", link_style.add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(" [Esc] Cancel ", cancel_style.add_modifier(Modifier::BOLD)),
    ]));

    f.render_widget(Paragraph::new(text), inner);
}

pub fn draw_hotkeys_modal(f: &mut Frame, _area: Rect) {
    let area = centered_rect(70, 80, f.area());
    f.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" KEYBINDINGS ")
        .border_style(Style::default().fg(theme::accent_primary()));
    f.render_widget(block.clone(), area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Fill(1),
            Constraint::Length(2),
        ])
        .split(block.inner(area));

    f.render_widget(
        Paragraph::new("Press ESC or F1 to Close")
            .style(Style::default().fg(theme::muted()))
            .alignment(ratatui::layout::Alignment::Center),
        chunks[0],
    );

    let keys = vec![
        (
            "Global",
            vec![
                ("F1", "Show this Help"),
                ("Ctrl + Q", "Quit Application"),
                ("Ctrl + B", "Toggle Sidebar"),
                ("Ctrl + M", "Toggle Main Stage"),
                ("Ctrl + P", "Toggle Split View"),
                ("Ctrl + G", "Open Settings"),
                ("Ctrl + L", "Git History"),
                ("Ctrl + E", "Toggle Editor View (IDE)"),
                ("Ctrl + J", "Toggle Bottom Panel"),
                ("Ctrl + Space", "Command Palette"),
                ("Ctrl + N", "Open Terminal"),
                ("Backspace", "Go Up Directory"),
            ],
        ),
        (
            "IDE Mode",
            vec![
                ("Ctrl + B", "Toggle Sidebar"),
                ("Ctrl + P", "Toggle Split Panes"),
                ("Esc", "Focus Sidebar / Back"),
                ("Enter", "Open File/Folder"),
                ("Arrows", "Navigate Tree / Editor"),
            ],
        ),
        (
            "File Navigation",
            vec![
                ("Arrows", "Navigate"),
                ("Enter", "Open Folder / Launch"),
                ("Ctrl + R", "Run File"),
                ("Space", "Expand/Collapse Folder"),
                ("Ctrl + I", "Information"),
                ("Backspace", "Go Up Directory"),
                ("Home / ~", "Go Home"),
                ("Alt + Left/Right", "Resize Sidebar"),
                ("F2", "Rename File"),
                ("Delete", "Delete to Trash"),
            ],
        ),
        (
            "Editor",
            vec![
                ("Ctrl + F", "Find (Live Filter)"),
                ("F2", "Replace All"),
                ("Ctrl + G", "Go To Line"),
                ("Ctrl + C", "Copy Line"),
                ("Ctrl + X", "Cut Line / Delete Line"),
                ("Ctrl + Bksp", "Delete Word"),
                ("Esc", "Exit Editor"),
            ],
        ),
        (
            "System Monitor",
            vec![
                ("1 / 2 / 3", "Overview / Processes / Apps"),
                ("k", "Kill Process (Signal Picker)"),
                ("t", "Toggle Tree View"),
                ("Arrows", "Navigate / Scroll"),
            ],
        ),
    ];

    let mut rows = Vec::new();
    for (section, items) in keys {
        rows.push(Row::new(vec![
            Cell::from(Span::styled(
                section,
                Style::default()
                    .fg(theme::accent_primary())
                    .add_modifier(Modifier::BOLD),
            )),
            Cell::from(""),
        ]));
        for (key, desc) in items {
            rows.push(Row::new(vec![
                Cell::from(Span::styled(
                    format!("  {}", key),
                    Style::default().fg(theme::warning()),
                )),
                Cell::from(desc),
            ]));
        }
        rows.push(Row::new(vec![Cell::from(""), Cell::from("")]));
    }

    let table = Table::new(
        rows,
        [Constraint::Percentage(30), Constraint::Percentage(70)],
    )
    .block(Block::default());

    f.render_widget(table, chunks[1]);
}

pub fn draw_open_with_modal(f: &mut Frame, app: &App, path: &std::path::Path) {
    let area = centered_rect(60, 60, f.area()); // Increased height
    f.render_widget(Clear, area);
    let block = Block::default()
        .title(" Open With... ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::warning()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Info
            Constraint::Length(3), // Input
            Constraint::Min(0),    // Suggestions List
        ])
        .split(inner);

    let file_name = path.file_name().unwrap_or_default().to_string_lossy();
    f.render_widget(Paragraph::new(format!("Opening: {}", file_name)), chunks[0]);

    let input_block = Block::default()
        .borders(Borders::ALL)
        .title(" Custom Command ")
        .border_style(Style::default().fg(theme::accent_primary()));
    f.render_widget(
        Paragraph::new(app.core.input.value.as_str()).block(input_block),
        chunks[1],
    );

    // Simple common suggestions based on extension
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let mut suggestions = crate::events::mouse_helpers::get_open_with_suggestions(app, &ext);

    // Filter suggestions based on input
    if !app.core.input.value.is_empty() {
        let query = app.core.input.value.to_lowercase();
        suggestions.retain(|s| s.to_lowercase().contains(&query));
    }

    let (mx, my) = app.core.mouse_pos;
    let list_items: Vec<ListItem> = suggestions
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let item_y = chunks[2].y + i as u16;
            let is_mouse_hovered =
                mx >= chunks[2].x && mx < chunks[2].x + chunks[2].width && my == item_y;
            let is_selected = i == app.settings.open_with_index;

            let style = if is_mouse_hovered || is_selected {
                Style::default()
                    .bg(theme::accent_primary())
                    .fg(theme::selection_fg())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::fg())
            };

            ListItem::new(format!("  󰀻  {}", s)).style(style)
        })
        .collect();

    let title = if app.core.input.value.is_empty() {
        " Suggestions (Click to Launch) "
    } else {
        " Filtered Suggestions (Click to Launch) "
    };

    let list = List::new(list_items).block(
        Block::default()
            .title(title)
            .borders(Borders::TOP)
            .border_style(Style::default().fg(theme::muted())),
    );
    f.render_widget(list, chunks[2]);
}
