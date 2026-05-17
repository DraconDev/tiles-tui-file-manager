//! Global header rendering — top bar with mode indicator, tab switcher, icons.
//! Extracted from ui/mod.rs.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use std::time::SystemTime;
use unicode_width::UnicodeWidthStr;

use crate::state::CurrentView;
use crate::{state::AppMode, App};
use crate::icons::Icon;
use crate::ui::theme::{accent_primary, accent_secondary, selection_bg};

pub fn draw_global_header(f: &mut Frame, area: Rect, sidebar_width: u16, app: &mut App) {
    let _now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    let pane_count = app.panes.len();

    // Toolbar Icons Cluster (Far Left)
    let back_icon = Icon::Back.get(app.core.icon_mode);
    let forward_icon = Icon::Forward.get(app.core.icon_mode);
    let split_icon = Icon::Split.get(app.core.icon_mode);
    let burger_icon = Icon::Burger.get(app.core.icon_mode);

    let monitor_icon = Icon::Monitor.get(app.core.icon_mode);
    let git_icon = Icon::Git.get(app.core.icon_mode);
    let editor_icon = Icon::Document.get(app.core.icon_mode);

    app.layout.header_icon_bounds.clear();
    let mut cur_icon_x = area.x + 2;

    let show_icons = app.sidebar.show_sidebar;

    if show_icons {
        let icons = [
            (burger_icon, "burger"),
            (back_icon, "back"),
            (forward_icon, "forward"),
            (split_icon, "split"),
            (monitor_icon, "monitor"),
            (git_icon, "git"),
            (editor_icon, "project"),
        ];

        for (i, (icon, id)) in icons.into_iter().enumerate() {
            let width = icon.width() as u16;
            let rect = Rect::new(cur_icon_x, area.y, width, 1);

            let mut style = Style::default().fg(accent_secondary());
            if let AppMode::Header(idx) = app.core.mode {
                if idx == i {
                    style = style
                        .bg(accent_primary())
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD);
                }
            }

            f.render_widget(Paragraph::new(icon).style(style), rect);
            app.layout.header_icon_bounds.push((rect, id.to_string()));
            cur_icon_x += width + 2;
        }
    }

    if pane_count == 0 {
        return;
    }
    let start_x = if show_icons {
        std::cmp::max(area.x + sidebar_width, cur_icon_x + 1)
    } else {
        area.x + 2
    };
    let pane_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![Constraint::Fill(1); pane_count])
        .spacing(1) // Add spacing between panes to prevent tab bleeding
        .split(Rect::new(
            start_x,
            area.y,
            area.width.saturating_sub(start_x),
            1,
        ));

    app.layout.tab_bounds.clear();
    let mut global_tab_idx = if show_icons { 6 } else { 0 };
    for (p_i, pane) in app.panes.iter().enumerate() {
        let chunk = pane_chunks[p_i];
        let mut current_x = chunk.x;

        if app.core.current_view == CurrentView::Editor {
            if pane.tabs.is_empty() {
                continue;
            }
            for (t_i, tab) in pane.tabs.iter().enumerate() {
                let is_active_tab = t_i == pane.active_tab_index;
                let is_focused_pane = p_i == app.focused_pane_index && !app.sidebar.sidebar_focus;

                let is_modified = tab
                    .preview
                    .as_ref()
                    .and_then(|p| p.editor.as_ref())
                    .map(|e| e.modified)
                    .unwrap_or(false);

                let base_style = if is_active_tab {
                    if is_focused_pane {
                        Style::default()
                            .bg(if is_modified { selection_bg() } else { Color::Reset })
                            .fg(accent_primary())
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                            .bg(if is_modified { selection_bg() } else { Color::Reset })
                            .fg(accent_primary())
                    }
                } else {
                    Style::default().fg(Color::DarkGray)
                };

                let base_name = if is_active_tab {
                    if let Some(fs) = pane.current_state() {
                        if let Some(preview) = &fs.preview {
                            preview
                                .path
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| "Editor".to_string())
                        } else {
                            tab.current_path
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| "/".to_string())
                        }
                    } else {
                        tab.current_path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| "/".to_string())
                    }
                } else {
                    tab.current_path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "/".to_string())
                };

                let mut spans = vec![Span::styled(format!(" {}", base_name), base_style)];

                // Show git branch in Editor view tabs too
                if let Some(branch) = &tab.git_branch {
                    let pending = tab.git_pending.len();
                    let ahead = tab.git_ahead;
                    let behind = tab.git_behind;

                    let branch_color = if pending > 0 {
                        Color::Red
                    } else if ahead > 0 || behind > 0 {
                        Color::Yellow
                    } else {
                        Color::Green
                    };

                    let mut branch_style = Style::default().fg(branch_color);
                    if is_active_tab && is_focused_pane {
                        branch_style = branch_style.add_modifier(Modifier::BOLD);
                    }

                    spans.push(Span::styled(format!("({})", branch), branch_style));

                    if pending > 0 {
                        spans.push(Span::styled(
                            format!(" +{}", pending),
                            Style::default().fg(Color::Red),
                        ));
                    }
                    if ahead > 0 {
                        spans.push(Span::styled(
                            format!(" ↑{}", ahead),
                            Style::default().fg(Color::Yellow),
                        ));
                    }
                    if behind > 0 {
                        spans.push(Span::styled(
                            format!(" ↓{}", behind),
                            Style::default().fg(Color::Yellow),
                        ));
                    }
                    spans.push(Span::raw(" "));
                }

                let line = Line::from(spans.clone());
                let total_width = line.width() as u16;
                let max_width = chunk.x + chunk.width - current_x;

                let final_line = if total_width > max_width && max_width > 3 {
                    let mut truncated = vec![];
                    let mut current_w = 0;
                    for span in spans {
                        let span_w = span.content.width() as u16;
                        if current_w + span_w > max_width - 1 {
                            truncated.push(Span::styled("…", Style::default().fg(Color::DarkGray)));
                            break;
                        }
                        truncated.push(span);
                        current_w += span_w;
                    }
                    Line::from(truncated)
                } else {
                    line
                };

                let width = total_width.min(max_width);
                if width > 0 {
                    let rect = Rect::new(current_x, area.y, width, 1);
                    f.render_widget(Paragraph::new(final_line), rect);
                    app.layout.tab_bounds.push((rect, p_i, t_i));
                }
            }
            continue;
        }

        for (t_i, tab) in pane.tabs.iter().enumerate() {
            let mut spans = Vec::new();
            let base_name = tab
                .current_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "/".to_string());

            let is_active_tab = t_i == pane.active_tab_index;
            let is_focused_pane = p_i == app.focused_pane_index && !app.sidebar.sidebar_focus;

            let mut base_style = if is_active_tab {
                if is_focused_pane {
                    Style::default()
                        .fg(accent_primary())
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(accent_primary())
                }
            } else {
                Style::default().fg(Color::DarkGray)
            };

            if let AppMode::Header(idx) = app.core.mode {
                if idx == global_tab_idx {
                    base_style = base_style
                        .bg(accent_primary())
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD);
                }
            }

            spans.push(Span::styled(format!(" {} ", base_name), base_style));

            if matches!(app.core.current_view, CurrentView::Files | CurrentView::Git) {
                if let Some(branch) = &tab.git_branch {
                    let pending = tab.git_pending.len();
                    let ahead = tab.git_ahead;
                    let behind = tab.git_behind;

                    let branch_color = if pending > 0 {
                        Color::Red
                    } else if ahead > 0 || behind > 0 {
                        Color::Yellow
                    } else {
                        Color::Green
                    };

                    let mut branch_style = Style::default().fg(branch_color);
                    if is_active_tab && is_focused_pane {
                        branch_style = branch_style.add_modifier(Modifier::BOLD);
                    }

                    spans.push(Span::styled(format!("({})", branch), branch_style));

                    if pending > 0 {
                        spans.push(Span::styled(
                            format!(" +{}", pending),
                            Style::default().fg(Color::Red),
                        ));
                    }
                    if ahead > 0 {
                        spans.push(Span::styled(
                            format!(" ↑{}", ahead),
                            Style::default().fg(Color::Yellow),
                        ));
                    }
                    if behind > 0 {
                        spans.push(Span::styled(
                            format!(" ↓{}", behind),
                            Style::default().fg(Color::Yellow),
                        ));
                    }
                    spans.push(Span::raw(" "));
                }
            }

            let line = Line::from(spans.clone());
            let total_width = line.width() as u16;

            // Calculate max available width for this tab
            let max_available = chunk.x + chunk.width - current_x;

            // Actually truncate the line content if too wide
            let final_line = if total_width > max_available && max_available > 3 {
                // Build truncated spans
                let mut truncated = vec![];
                let mut current_w = 0;
                for span in spans {
                    let span_w = span.content.width() as u16;
                    if current_w + span_w > max_available - 1 {
                        // Add ellipsis and stop
                        truncated.push(Span::styled("…", Style::default().fg(Color::DarkGray)));
                        break;
                    }
                    truncated.push(span);
                    current_w += span_w;
                }
                Line::from(truncated)
            } else {
                line
            };

            let width = total_width.min(max_available);
            if width == 0 || current_x + width > chunk.x + chunk.width {
                break;
            }
            let rect = Rect::new(current_x, area.y, width, 1);
            f.render_widget(Paragraph::new(final_line), rect);
            app.layout.tab_bounds.push((rect, p_i, t_i));
            current_x += width + 1;
            global_tab_idx += 1;
        }
    }
}
