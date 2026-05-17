#![allow(unused_imports)]

//! Git commit view rendering.
//! Extracted from ui/mod.rs (Phase 3).

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Widget},
    Frame,
};
use std::vec::Vec;

use crate::app::App;
use crate::icons::Icon;
use crate::ui::theme as theme;
use crate::ui::theme::THEME;
use dracon_terminal_engine::utils::{format_permissions, truncate_to_width};

pub fn draw_commit_view(f: &mut Frame, area: Rect, app: &mut App) {
    f.render_widget(
        Block::default().style(Style::default().bg(Color::Black)),
        area,
    );

    let (
        mut commit_hash,
        mut author,
        mut date,
        mut subject,
        mut files_changed,
        mut additions,
        mut deletions,
        mut hunks,
    ) = (
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        0usize,
        0usize,
        0usize,
        0usize,
    );
    let mut touched_files: Vec<String> = Vec::new();

    let content_source = app.editor_global.editor_state.as_ref()
        .or_else(|| {
            let pane_idx = app.focused_pane_index;
            app.panes.get(pane_idx).and_then(|p| p.current_state().and_then(|fs| fs.view.preview.as_ref()))
        });

    if let Some(preview) = content_source {
        for line in preview.content.lines() {
            if commit_hash.is_empty() && line.starts_with("commit ") {
                commit_hash = line.trim_start_matches("commit ").trim().to_string();
                continue;
            }
            if author.is_empty() && line.starts_with("Author:") {
                author = line.trim_start_matches("Author:").trim().to_string();
                continue;
            }
            if date.is_empty() && line.starts_with("Date:") {
                date = line.trim_start_matches("Date:").trim().to_string();
                continue;
            }
            if subject.is_empty() && line.starts_with("    ") {
                let candidate = line.trim();
                if !candidate.is_empty() {
                    subject = candidate.to_string();
                    continue;
                }
            }
            if line.starts_with("diff --git ") {
                files_changed += 1;
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    let from = parts[2].trim_start_matches("a/");
                    let to = parts[3].trim_start_matches("b/");
                    let display = if from == to {
                        to.to_string()
                    } else {
                        format!("{} -> {}", from, to)
                    };
                    if !touched_files.contains(&display) {
                        touched_files.push(display);
                    }
                }
            } else if line.starts_with("@@") {
                hunks += 1;
            } else if line.starts_with('+') && !line.starts_with("+++") {
                additions += 1;
            } else if line.starts_with('-') && !line.starts_with("---") {
                deletions += 1;
            }
        }
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::border_inactive()))
        .title_top(Line::from(vec![Span::styled(
            " COMMIT ",
            Style::default()
                .fg(Color::Black)
                .bg(theme::accent_primary())
                .add_modifier(Modifier::BOLD),
        )]))
        .title_top(
            Line::from(vec![
                Span::styled(
                    " Esc ",
                    Style::default()
                        .fg(theme::selection_fg())
                        .bg(theme::danger())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Back to Git ", Style::default().fg(theme::danger())),
            ])
            .alignment(Alignment::Right),
        );
    let inner = block.inner(area);
    f.render_widget(block, area);

    let short_hash = if commit_hash.is_empty() {
        "unknown".to_string()
    } else {
        commit_hash.chars().take(12).collect::<String>()
    };
    if author.is_empty() {
        author = "unknown author".to_string();
    }
    if date.is_empty() {
        date = "unknown date".to_string();
    }
    if subject.is_empty() {
        subject = "(no subject)".to_string();
    }

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(inner);

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!("{} ", short_hash),
                Style::default()
                    .fg(Color::Black)
                    .bg(theme::accent_primary())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  ", Style::default()),
            Span::styled(author, Style::default().fg(Color::White)),
        ])),
        layout[0],
    );

    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            date,
            Style::default().fg(theme::muted()),
        )])),
        layout[1],
    );

    let subject_text = truncate_to_width(&subject, layout[2].width as usize, "...");
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            format!(" {}", subject_text),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )])),
        layout[2],
    );

    let files_preview = if touched_files.is_empty() {
        "files: none".to_string()
    } else {
        let visible = touched_files
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        let extra = touched_files.len().saturating_sub(3);
        if extra > 0 {
            format!("files: {} +{}", visible, extra)
        } else {
            format!("files: {}", visible)
        }
    };
    let files_preview = truncate_to_width(&files_preview, layout[3].width as usize, "...");

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!(" {} files ", files_changed),
                Style::default()
                    .fg(Color::Black)
                    .bg(theme::accent_secondary())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                format!(" +{} ", additions),
                Style::default()
                    .fg(Color::Black)
                    .bg(theme::success())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                format!(" -{} ", deletions),
                Style::default()
                    .fg(theme::selection_fg())
                    .bg(theme::danger())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                format!(" @@ {} ", hunks),
                Style::default()
                    .fg(Color::Black)
                    .bg(theme::header_fg())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {}", files_preview),
                Style::default().fg(theme::muted()),
            ),
        ])),
        layout[3],
    );

    let content_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::border_inactive()))
        .title_top(Line::from(vec![Span::styled(
            " PATCH ",
            Style::default()
                .fg(Color::Black)
                .bg(theme::accent_primary())
                .add_modifier(Modifier::BOLD),
        )]));
    let content_inner = content_block.inner(layout[4]);
    f.render_widget(content_block, layout[4]);

    if let Some(preview) = &app.editor_global.editor_state {
        if let Some(editor) = &preview.editor {
            let mut editor_clone = editor.clone();
            editor_clone.wrap = false;
            editor_clone.show_line_numbers = true;
            editor_clone.read_only = true;
            if editor_clone.language.is_empty() {
                editor_clone.language = "diff".to_string();
            }
            f.render_widget(&editor_clone, content_inner);
            return;
        }
    }

    if let Some(pane) = app.panes.get(app.focused_pane_index) {
        if let Some(fs) = pane.current_state() {
            if let Some(preview) = &fs.view.preview {
                if let Some(editor) = &preview.editor {
                let mut editor_clone = editor.clone();
                editor_clone.wrap = false;
                editor_clone.show_line_numbers = true;
                editor_clone.read_only = true;
                if editor_clone.language.is_empty() {
                    editor_clone.language = "diff".to_string();
                }
                f.render_widget(&editor_clone, content_inner);
                return;
                }
            }
        }
    }

    f.render_widget(
        Paragraph::new("Loading commit...")
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme::muted())),
        content_inner,
    );
}