//! Footer rendering — status bar with task info.
//! Extracted from ui/mod.rs (Phase 3).

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::{App, CurrentView, DropTarget};
use crate::ui::theme as theme;
use crate::ui::theme::THEME;
use dracon_terminal_engine::utils::format_size;
use dracon_terminal_engine::widgets::HotkeyHint;
use unicode_width::UnicodeWidthStr;

pub fn draw_stat_bar(
    label: &str,
    value: f32,
    max: f32,
    low_color: Color,
    mid_color: Color,
    label_color: Color,
) -> Line<'static> {
    let width = 10;
    let ratio = (value / max.max(1.0)).clamp(0.0, 1.0);
    let filled = (ratio * width as f32).round() as usize;

    let mut spans = vec![Span::styled(
        format!("{} ", label),
        Style::default().fg(label_color),
    )];

    for i in 0..width {
        let symbol = if i < filled { "█" } else { "░" };
        let color = if ratio < 0.4 {
            low_color
        } else if ratio < 0.7 {
            mid_color
        } else {
            Color::Red // Warning/Danger
        };

        if i < filled {
            spans.push(Span::styled(symbol, Style::default().fg(color)));
        } else {
            spans.push(Span::styled(
                symbol,
                Style::default().fg(Color::Rgb(30, 30, 35)),
            ));
        }
    }

    spans.push(Span::styled(
        format!(" {:>3.0}%", ratio * 100.0),
        Style::default().fg(THEME.fg).add_modifier(Modifier::BOLD),
    ));
    Line::from(spans)
}

pub fn draw_footer(f: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),    // Log, Clipboard & Shortcuts
            Constraint::Length(20), // Selection Info
            Constraint::Length(45), // Stats (CPU/MEM)
        ])
        .split(chunks[0]);

    // 1. Left Section: ^Q Quit, Activity Log, Clipboard & Essential Shortcuts
    let mut left_spans = vec![Span::raw(" ")];

    // Log - If present, hide other shortcuts on the left
    let mut showing_log = false;
    if let Some((msg, time)) = &app.output.last_action_msg {
        if time.elapsed().as_secs() < 5 {
            left_spans.push(Span::styled(
                format!(" [ SYSTEM ] {} ", msg),
                Style::default()
                    .fg(theme::accent_secondary())
                    .bg(Color::Rgb(20, 25, 30)),
            ));
            showing_log = true;
        }
    }

    if app.drag.is_dragging {
        if let Some(src) = &app.drag.drag_source {
            let name = src.file_name().and_then(|n| n.to_str()).unwrap_or("...");
            left_spans.push(Span::styled(
                " DRAGGING ",
                Style::default()
                    .fg(Color::Black)
                    .bg(theme::accent_primary())
                    .add_modifier(Modifier::BOLD),
            ));
            left_spans.push(Span::styled(
                format!(" {} ", name),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ));

            if let Some(target) = &app.drag.hovered_drop_target {
                left_spans.push(Span::raw(" to "));
                let target_desc = match target {
                    DropTarget::Folder(p) => {
                        p.file_name().and_then(|n| n.to_str()).unwrap_or("Folder")
                    }
                    DropTarget::Favorites => "Favorites",
                    DropTarget::ReorderFavorite(_) => "Favorites (Reorder)",
                };
                left_spans.push(Span::styled(
                    format!(" {} ", target_desc),
                    Style::default()
                        .fg(theme::accent_secondary())
                        .add_modifier(Modifier::BOLD),
                ));
            }
            showing_log = true; // Use this to skip shortcuts
        }
    }

    if !showing_log {
        left_spans.extend(HotkeyHint::render("^Q", "Quit", Color::Red));

        let hidden_on = if let Some(fs) = app.current_file_state() {
            fs.nav.show_hidden
        } else {
            app.settings.default_show_hidden
        };

        let mut shortcuts = Vec::new();
        if app.core.current_view == CurrentView::Editor {
            shortcuts.extend(HotkeyHint::render("Esc", "Back", Color::Red));
            shortcuts.extend(HotkeyHint::render(
                "^B",
                "Sidebar",
                theme::accent_secondary(),
            ));
            shortcuts.extend(HotkeyHint::render(
                "^P",
                "Split",
                theme::accent_secondary(),
            ));
            shortcuts.extend(HotkeyHint::render(
                "^F",
                "Find",
                theme::accent_secondary(),
            ));
            shortcuts.extend(HotkeyHint::render(
                "F2",
                "Replace",
                theme::accent_secondary(),
            ));
            shortcuts.extend(HotkeyHint::render(
                "^G",
                "GoTo",
                theme::accent_secondary(),
            ));
            shortcuts.extend(HotkeyHint::render(
                "^R",
                "Run",
                theme::accent_secondary(),
            ));
        } else {
            shortcuts.extend(HotkeyHint::render(
                "^P",
                "Split",
                theme::accent_secondary(),
            ));
            shortcuts.extend(HotkeyHint::render(
                "^T",
                "Tab",
                theme::accent_secondary(),
            ));
            shortcuts.extend(HotkeyHint::render(
                "^N",
                "TermTab",
                theme::accent_secondary(),
            ));
            shortcuts.extend(HotkeyHint::render(
                "^K",
                "TermWin",
                theme::accent_secondary(),
            ));
            shortcuts.extend(HotkeyHint::render(
                "^R",
                "Run",
                theme::accent_secondary(),
            ));
            shortcuts.extend(HotkeyHint::render(
                "^H",
                "Hidden",
                if hidden_on { Color::Green } else { Color::Red },
            ));
            shortcuts.extend(HotkeyHint::render(
                "Space",
                "Expand/Edit",
                Color::Rgb(88, 166, 255),
            )); // GitHub Blue
        }

        for s in shortcuts {
            left_spans.push(s);
        }

        // Add Remote Status Badge
        let is_remote = app.panes.iter().any(|p| {
            if let Some(fs) = p.current_state() {
                fs.nav.remote_session.is_some()
            } else {
                false
            }
        });

        if is_remote {
            left_spans.push(Span::raw(" │ "));
            left_spans.push(Span::styled(
                " REMOTE ",
                Style::default()
                    .bg(theme::accent_secondary())
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            ));
        }
    }

    f.render_widget(
        Paragraph::new(Line::from(left_spans)).wrap(ratatui::widgets::Wrap { trim: false }),
        top_chunks[0],
    );

    // 2. Center Section: Selection Summary (Only in Files view)
    if app.core.current_view == CurrentView::Files {
        if let Some(fs) = app.current_file_state() {
            let sel_count = if !fs.list.selection.is_empty() {
                fs.list.selection.multi.len()
            } else if fs.list.selection.selected.is_some() {
                1
            } else {
                0
            };
            let total_count = fs.list.files.len();
            let selected_bytes = if !fs.list.selection.is_empty() {
                let mut sum = 0u64;
                for &idx in fs.list.selection.multi_selected_indices() {
                    if let Some(path) = fs.list.files.get(idx) {
                        if let Some(meta) = fs.list.metadata.get(path) {
                            if !meta.is_dir {
                                sum = sum.saturating_add(meta.size);
                            }
                        }
                    }
                }
                sum
            } else if let Some(idx) = fs.list.selection.selected {
                if let Some(path) = fs.list.files.get(idx) {
                    fs.list.metadata
                        .get(path)
                        .map(|m| if m.is_dir { 0 } else { m.size })
                        .unwrap_or(0)
                } else {
                    0
                }
            } else {
                0
            };
            let summary_w = top_chunks[1].width as usize;
            let size_tag = if sel_count > 1 {
                format!(" {}", format_size(selected_bytes))
            } else {
                String::new()
            };
            let summary_plain = format!(" {}/{} ", sel_count, total_count);
            let summary_with_size = format!(" {}/{}{} ", sel_count, total_count, size_tag);
            let summary = if !size_tag.is_empty() && summary_with_size.width() <= summary_w {
                summary_with_size
            } else {
                summary_plain
            };
            let summary_style = if app.sidebar.sidebar_focus {
                Style::default()
                    .bg(Color::Rgb(85, 80, 20))
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .bg(theme::accent_primary())
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD)
            };
            f.render_widget(
                Paragraph::new(Span::styled(summary, summary_style))
                    .alignment(ratatui::layout::Alignment::Center),
                top_chunks[1],
            );
        }
    }

    // 3. Stats (CPU/MEM) - Far Right
    let cpu_bar = draw_stat_bar(
        "CPU",
        app.system_state.cpu_usage,
        100.0,
        Color::Rgb(80, 200, 255),
        Color::Yellow,
        Color::DarkGray,
    );
    let mem_usage = (app.system_state.mem_usage / app.system_state.total_mem.max(1.0)) * 100.0;
    let mem_bar = draw_stat_bar(
        "MEM",
        mem_usage,
        100.0,
        Color::Rgb(88, 166, 255),
        Color::Rgb(255, 170, 0),
        Color::Rgb(140, 165, 210),
    );

    let stats_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(22),
            Constraint::Length(22),
            Constraint::Fill(1),
        ])
        .split(top_chunks[2]);

    f.render_widget(
        Paragraph::new(cpu_bar).alignment(ratatui::layout::Alignment::Right),
        stats_layout[0],
    );
    f.render_widget(
        Paragraph::new(mem_bar).alignment(ratatui::layout::Alignment::Right),
        stats_layout[1],
    );

    // 4. CYBER_PULSE (Animated Indicator)
    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let pulse_frames = [
        " ", "▂", "▃", "▄", "▅", "▆", "▇", "█", "▇", "▆", "▅", "▄", "▃", "▂",
    ];
    let pulse_idx = (time / 80) % pulse_frames.len() as u128;
    let pulse_char = pulse_frames[pulse_idx as usize];

    let pulse_spans = vec![
        Span::styled(" PULSE ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            pulse_char.repeat(3),
            Style::default().fg(theme::accent_primary()),
        ),
    ];

    f.render_widget(
        Paragraph::new(Line::from(pulse_spans)).alignment(ratatui::layout::Alignment::Right),
        stats_layout[2],
    );

    // 5. Bottom Line: Background Tasks
    let mut task_spans = Vec::new();
    for task in &app.output.background_tasks {
        let pct = (task.progress * 100.0) as usize;
        let bar = "█".repeat(pct / 10) + &"░".repeat(10 - (pct / 10));
        task_spans.push(Span::styled(
            format!(" {} [{}%] ", task.name, pct),
            Style::default().fg(Color::Cyan),
        ));
        task_spans.push(Span::styled(
            format!("{} ", bar),
            Style::default().fg(Color::Cyan),
        ));
    }

    if !task_spans.is_empty() {
        f.render_widget(Paragraph::new(Line::from(task_spans)), chunks[1]);
    }
}