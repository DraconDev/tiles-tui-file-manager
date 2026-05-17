#![allow(unused_imports)]

//! Git page rendering — commit view, pending changes, helpers.
//! Extracted from ui/pane.rs (Phase 3 continuation).

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, Borders, Cell, Clear, List, ListItem, ListState, Paragraph, Row,
        Scrollbar, ScrollbarOrientation, ScrollbarState, Table, TableState, Widget,
    },
    Frame,
};
use std::vec::Vec;

use crate::app::{App, CurrentView, DropTarget};
use crate::icons::Icon;
use crate::state::{FileColumn, FileViewState};
use crate::ui::theme as theme;
use crate::ui::theme::THEME;
use crate::ui::footer::{draw_stat_bar, draw_footer};
use crate::ui::git_view::draw_commit_view;
use crate::ui::small_modals::draw_signal_select_modal;
use dracon_terminal_engine::layout::centered_rect;
use dracon_terminal_engine::utils::{
    format_permissions, format_size, format_time, get_visual_width, squarify, truncate_to_width,
};
use dracon_terminal_engine::widgets::HotkeyHint;
use unicode_width::UnicodeWidthStr;

pub fn parse_commit_refs(decorations: &str) -> Vec<String> {
    decorations
        .trim()
        .trim_matches(|c| c == '(' || c == ')')
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

pub fn style_for_ref_label(label: &str) -> Style {
    if label.starts_with("HEAD -> ") {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else if label.starts_with("tag: ") {
        Style::default().fg(Color::Magenta)
    } else if label.starts_with("origin/") {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::Yellow)
    }
}

pub fn refs_line(refs: &[String], max_refs: usize) -> Line<'static> {
    if refs.is_empty() {
        return Line::from("");
    }

    let mut spans = Vec::new();
    let shown = refs.len().min(max_refs);
    for (i, r) in refs.iter().take(shown).enumerate() {
        if i > 0 {
            spans.push(Span::styled(", ", Style::default().fg(Color::DarkGray)));
        }
        spans.push(Span::styled(
            truncate_to_width(r, 18, ".."),
            style_for_ref_label(r),
        ));
    }

    if refs.len() > shown {
        spans.push(Span::styled(
            format!(" +{}", refs.len() - shown),
            Style::default().fg(Color::DarkGray),
        ));
    }

    Line::from(spans)
}

pub fn draw_git_page(f: &mut Frame, area: Rect, app: &mut App) {
    f.render_widget(Clear, area);
    let pane_idx = app.focused_pane_index;
    let tab_idx = if let Some(pane) = app.panes.get(pane_idx) {
        pane.active_tab_index
    } else {
        0
    };

    let Some(tab) = app.panes.get(pane_idx).and_then(|p| p.tabs.get(tab_idx)) else {
        return;
    };

    let branch_name = tab.git.git_branch.as_deref().unwrap_or("HEAD");
    let summary_text = tab.git.git_summary.as_deref().unwrap_or("");
    let current_path_label = tab.nav.current_path.to_string_lossy().to_string();
    let history_len = tab.git.git_history.len();
    let pending_len = tab.git.git_pending.len();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::accent_primary()))
        .style(Style::default().bg(Color::Rgb(0, 0, 0)))
        .title_top(Line::from(vec![
            Span::styled(
                " GIT HUB ",
                Style::default()
                    .fg(Color::Black)
                    .bg(theme::accent_primary())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" [{}] ", branch_name),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {} ", current_path_label),
                Style::default().fg(theme::accent_secondary()),
            ),
        ]))
        .title_bottom(Line::from(format!(" {} ", summary_text)).alignment(Alignment::Right))
        .title_top(
            Line::from(vec![
                Span::styled(
                    " Esc ",
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Red)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Back ", Style::default().fg(Color::Red)),
            ])
            .alignment(Alignment::Right),
        );

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Only show ACTIVE (pending) changes at top, no INFO panel
    let top_h = if pending_len == 0 {
        0
    } else {
        (pending_len as u16 + 2).min(inner.height / 3)
    };

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(top_h), Constraint::Min(0)])
        .split(inner);

    let top_area = main_chunks[0];
    let history_area = main_chunks[1];

    if top_h > 0 {
        let active_area = top_area;
        f.render_widget(Clear, active_area);

        if pending_len > 0 {
            let active_title = format!(" ACTIVE ({} Affected) ", pending_len);
            let pending_rows: Vec<_> = app.panes
                .get(pane_idx)
                .and_then(|p| p.tabs.get(tab_idx))
                .map(|t| {
                    t.git.git_pending
                        .iter()
                        .map(|p| {
                            let status_color = match p.status.as_str() {
                                "M" => Color::Yellow,
                                "A" | "??" => Color::Green,
                                "D" => Color::Red,
                                "R" => Color::Cyan,
                                _ => Color::White,
                            };

                            let mut stats_spans = Vec::new();
                            if p.insertions > 0 {
                                stats_spans.push(Span::styled(
                                    format!(" +{}", p.insertions),
                                    Style::default().fg(Color::Green),
                                ));
                            }
                            if p.deletions > 0 {
                                stats_spans.push(Span::styled(
                                    format!(" -{}", p.deletions),
                                    Style::default().fg(Color::Red),
                                ));
                            }

                            Row::new(vec![
                                Cell::from(format!(" {} ", p.status)).style(
                                    Style::default()
                                        .bg(status_color)
                                        .fg(Color::Black)
                                        .add_modifier(Modifier::BOLD),
                                ),
                                Cell::from(p.path.clone()).style(Style::default().fg(THEME.fg)),
                                Cell::from(Line::from(stats_spans)),
                            ])
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            let pending_table = Table::new(
                pending_rows,
                [
                    Constraint::Length(6),
                    Constraint::Fill(1),
                    Constraint::Length(15),
                ],
            )
            .block(
                Block::default()
                    .title(active_title)
                    .border_style(Style::default().fg(Color::Rgb(40, 45, 55)))
                    .borders(Borders::RIGHT),
            )
            .row_highlight_style(
                Style::default()
                    .bg(Color::Rgb(40, 40, 50))
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );

            if let Some(pane) = app.panes.get_mut(pane_idx) {
                if let Some(tab) = pane.tabs.get_mut(tab_idx) {
                    f.render_stateful_widget(
                        pending_table,
                        active_area,
                        &mut tab.git.git_pending_state,
                    );
                }
            }
        }
    }

    if history_len == 0 {
        f.render_widget(
            Paragraph::new("\n\n No git history found for this path or not a git repository.")
                .alignment(Alignment::Center),
            history_area,
        );
    } else {
        let rows: Vec<_> = app.panes
            .get(pane_idx)
            .and_then(|p| p.tabs.get(tab_idx))
            .map(|t| {
                t.git.git_history
                    .iter()
                    .map(|act| {
                        let h_short = act.hash.chars().take(7).collect::<String>();
                        let refs = parse_commit_refs(&act.decorations);
                        let refs_compact = refs_line(&refs, 2);

                        let mut stats_cells = Vec::new();
                        if act.files_changed > 0 {
                            stats_cells.push(
                                Cell::from(act.files_changed.to_string())
                                    .style(Style::default().fg(Color::Cyan)),
                            );
                            stats_cells.push(
                                Cell::from(format!("+{}", act.insertions))
                                    .style(Style::default().fg(Color::Green)),
                            );
                            stats_cells.push(
                                Cell::from(format!("-{}", act.deletions))
                                    .style(Style::default().fg(Color::Red)),
                            );
                        } else {
                            stats_cells.push(Cell::from(""));
                            stats_cells.push(Cell::from(""));
                            stats_cells.push(Cell::from(""));
                        }

                        let mut row_cells = vec![
                            Cell::from(act.date.clone())
                                .style(Style::default().fg(Color::DarkGray)),
                            Cell::from(h_short).style(
                                Style::default()
                                    .fg(theme::accent_secondary())
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Cell::from(refs_compact),
                            Cell::from(act.author.clone()).style(Style::default().fg(Color::Cyan)),
                            Cell::from(act.message.clone()).style(Style::default().fg(THEME.fg)),
                        ];
                        row_cells.extend(stats_cells);

                        Row::new(row_cells)
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let table = Table::new(
            rows,
            [
                Constraint::Length(15),
                Constraint::Length(8),
                Constraint::Length(20),
                Constraint::Length(15),
                Constraint::Fill(1),
                Constraint::Length(6),
                Constraint::Length(6),
                Constraint::Length(6),
            ],
        )
        .header(
            Row::new(vec![
                "DATE", "HASH", "REFS", "AUTHOR", "MESSAGE", "FILES", "ADD", "DEL",
            ])
            .style(
                Style::default()
                    .fg(theme::accent_secondary())
                    .add_modifier(Modifier::BOLD),
            )
            .bottom_margin(1),
        )
        .block(
            Block::default()
                .title(" HISTORY ")
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Color::Rgb(40, 45, 55))),
        )
        .row_highlight_style(
            Style::default()
                .bg(Color::Rgb(40, 40, 50))
                .fg(theme::accent_secondary())
                .add_modifier(Modifier::BOLD),
        );

        if let Some(pane) = app.panes.get_mut(pane_idx) {
            if let Some(tab) = pane.tabs.get_mut(tab_idx) {
                f.render_stateful_widget(table, history_area, &mut tab.git.git_history_state);
            }
        }
    }
}
