#![allow(unused_imports)]

//! Small modals — signal select, drag-drop, hotkeys, open-with.
//! Extracted from ui/mod.rs (Phase 3).

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, Borders, Cell, Clear, List, ListItem, Paragraph, Row, Table,
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
    // Scrollable list dialogs size themselves relative to the frame; the
    // arguments here are percentages, not cells.
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

/// Rows the drag-drop choice dialog draws for `source_count` items.
pub fn drag_drop_content_rows(source_count: usize) -> u16 {
    let mut rows = if source_count <= 1 {
        1 // "Item: <name>"
    } else {
        1 + std::cmp::min(source_count, 3) as u16 // "Items: N" + listed names
    };
    if source_count > 3 {
        rows += 1; // "  ..." overflow line
    }
    rows + 3 // To: line, spacer, button row
}

/// Row offset (inside the border) of the drag-drop dialog's button row.
pub fn drag_drop_button_offset(source_count: usize) -> u16 {
    drag_drop_content_rows(source_count) - 1
}

pub fn draw_drag_drop_modal(
    f: &mut Frame,
    app: &App,
    sources: &[std::path::PathBuf],
    target: &std::path::Path,
) {
    // The height follows the number of rows actually drawn (item line, up to
    // three listed items, an "..." overflow line, the To: line, a spacer and
    // the button row). It is a cell count, not a percentage.
    let content_rows = drag_drop_content_rows(sources.len());
    let area = crate::ui::centered_modal(60, content_rows + 2, f.area());
    f.render_widget(Clear, area);
    let block = Block::default()
        .title(" Choice Action ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::warning()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let dest_path = target.to_string_lossy();

    // Buttons sit on the last content row; the offset is derived from the same
    // row count the box is sized with.
    let button_y_offset = drag_drop_button_offset(sources.len());

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
    // Percentages, not cells: the hotkey list scrolls, so it follows the frame.
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

pub fn draw_open_with_modal(f: &mut Frame, app: &mut App, path: &std::path::Path) {
    // Percentages: the suggestions list scrolls with the frame.
    let area = centered_rect(60, 60, f.area());
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

    let list_block = Block::default()
        .title(if app.core.input.value.is_empty() {
            " Suggestions (Click to Launch) "
        } else {
            " Filtered Suggestions (Click to Launch) "
        })
        .borders(Borders::TOP)
        .border_style(Style::default().fg(theme::muted()));
    // Keep the selected suggestion in view on small terminals, sharing the
    // persisted offset so hover styling lines up with the rendered rows.
    let visible = list_block.inner(chunks[2]).height as usize;
    let selected = (!suggestions.is_empty())
        .then_some(app.settings.open_with_index.min(suggestions.len() - 1));
    let start = crate::ui::settings::clamp_scroll(
        app.settings.open_with_scroll as usize,
        selected,
        suggestions.len(),
        visible,
    );
    app.settings.open_with_scroll = start as u16;
    let inner = list_block.inner(chunks[2]);
    let (mx, my) = app.core.mouse_pos;
    let list_items: Vec<ListItem> = (start..(start + visible).min(suggestions.len()))
        .map(|i| {
            let s = &suggestions[i];
            // True rendered row: the list body starts inside the top border.
            let item_y = inner.y + (i - start) as u16;
            let is_mouse_hovered =
                mx >= inner.x && mx < inner.x + inner.width && my == item_y;
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

    f.render_widget(List::new(list_items).block(list_block), chunks[2]);
}
    #[cfg(test)]
    mod drag_drop_layout_tests {
        use super::*;

        /// The box must grow with the item list so the Copy/Move/Link buttons
        /// stay inside it — a fixed 9-row box lost them at four or more
        /// sources.
        #[test]
        fn content_rows_cover_every_source_count_and_keep_the_buttons_inside() {
            for count in 1..=10usize {
                let rows = drag_drop_content_rows(count);
                let outer = rows + 2; // borders
                let inner = outer - 2;
                let button_offset = drag_drop_button_offset(count);
                assert_eq!(
                    button_offset, rows - 1,
                    "buttons must sit on the last content row ({count} sources)"
                );
                assert!(
                    button_offset < inner,
                    "button row {button_offset} must be inside {inner} inner rows ({count} sources)"
                );
            }
        }

        /// A single source stays compact; more sources add exactly the lines
        /// the renderer draws (listed items, and the "..." overflow line).
        #[test]
        fn content_rows_match_the_lines_the_renderer_builds() {
            assert_eq!(drag_drop_content_rows(1), 1 + 3, "one item line + To/spacer/buttons");
            assert_eq!(drag_drop_content_rows(2), 1 + 2 + 3);
            assert_eq!(drag_drop_content_rows(3), 1 + 3 + 3);
            assert_eq!(drag_drop_content_rows(4), 1 + 3 + 1 + 3, "overflow line appears above 3");
            assert_eq!(drag_drop_content_rows(10), 1 + 3 + 1 + 3, "listing is capped at three");
        }
    }

