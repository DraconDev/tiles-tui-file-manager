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
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::state::CurrentView;
use crate::{state::AppMode, App};
use crate::icons::Icon;
use crate::ui::theme as theme;
use crate::ui::theme::{accent_primary, accent_secondary, selection_bg};

/// Smallest tab width worth rendering: a leading space + 2 chars + ellipsis.
/// Below this a tab is drawn as a degenerate 1-cell sliver with a click rect
/// too small to hit — skip such tabs entirely (mirrors the existing
/// `max_width == 0` bail-out).
const MIN_TAB_WIDTH: u16 = 4;

/// Truncate a line to `max_width` cells at character granularity, keeping
/// per-span styling and reserving the last cell for "…". The returned line
/// has width exactly `min(original, max_width)`, so the click rect built
/// from the same numbers always matches what is actually drawn. Whole-span
/// truncation drew a lone "…" inside a wide rect whose click bounds were
/// far larger than the visible tab.
fn truncate_line_to_width(line: Line, max_width: u16) -> Line {
    let mut truncated: Vec<Span> = Vec::new();
    let mut current_w: u16 = 0;
    let budget = max_width.saturating_sub(1);
    for span in line.spans {
        if current_w >= budget {
            break;
        }
        let mut kept = String::new();
        for ch in span.content.chars() {
            let cw = ch.width().unwrap_or(0) as u16;
            if current_w + cw > budget {
                break;
            }
            kept.push(ch);
            current_w += cw;
        }
        if !kept.is_empty() {
            truncated.push(Span::styled(kept, span.style));
        }
    }
    truncated.push(Span::styled("\u{2026}", Style::default().fg(theme::muted())));
    Line::from(truncated)
}

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
                        .fg(theme::selection_fg())
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
    // Toolbar icons occupy indices 0-6; tabs follow from 7.
    let mut global_tab_idx = if show_icons { 7 } else { 0 };
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
                    .view.preview
                    .as_ref()
                    .and_then(|p| p.editor.as_ref())
                    .map(|e| e.modified)
                    .unwrap_or(false);

                let mut base_style = if is_active_tab {
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
                    Style::default().fg(theme::muted())
                };

                // Header-mode focus highlight (matches the non-Editor branch):
                // keyboard focus on this tab must be visible in Editor view too.
                if let AppMode::Header(idx) = app.core.mode {
                    if idx == global_tab_idx {
                        base_style = base_style
                            .bg(accent_primary())
                            .fg(theme::selection_fg())
                            .add_modifier(Modifier::BOLD);
                    }
                }

                let base_name = if is_active_tab {
                    if let Some(fs) = pane.current_state() {
                        if let Some(preview) = &fs.view.preview {
                            preview
                                .path
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| "Editor".to_string())
                        } else {
                            tab.nav.current_path
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| "/".to_string())
                        }
                    } else {
                        tab.nav.current_path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| "/".to_string())
                    }
                } else {
                    tab.nav.current_path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "/".to_string())
                };

                let mut spans = vec![Span::styled(format!(" {}", base_name), base_style)];

                // Show git branch in Editor view tabs too
                if let Some(branch) = &tab.git.git_branch {
                    let pending = tab.git.git_pending.len();
                    let ahead = tab.git.git_ahead;
                    let behind = tab.git.git_behind;

                    let branch_color = if pending > 0 {
                        theme::danger()
                    } else if ahead > 0 || behind > 0 {
                        theme::warning()
                    } else {
                        theme::success()
                    };

                    let mut branch_style = Style::default().fg(branch_color);
                    if is_active_tab && is_focused_pane {
                        branch_style = branch_style.add_modifier(Modifier::BOLD);
                    }

                    spans.push(Span::styled(format!("({})", branch), branch_style));

                    if pending > 0 {
                        spans.push(Span::styled(
                            format!(" +{}", pending),
                            Style::default().fg(theme::danger()),
                        ));
                    }
                    if ahead > 0 {
                        spans.push(Span::styled(
                            format!(" ↑{}", ahead),
                            Style::default().fg(theme::warning()),
                        ));
                    }
                    if behind > 0 {
                        spans.push(Span::styled(
                            format!(" ↓{}", behind),
                            Style::default().fg(theme::warning()),
                        ));
                    }
                    spans.push(Span::raw(" "));
                }

                let line = Line::from(spans);
                let total_width = line.width() as u16;
                let end_x = chunk.x + chunk.width;
                let max_width = end_x.saturating_sub(current_x);
                // No point drawing a tab that cannot show even a few glyphs +
                // ellipsis: it would emit a degenerate 1-cell click rect.
                if max_width < MIN_TAB_WIDTH {
                    break;
                }

                // Character-level truncation so the rendered width always
                // equals the click rect width.
                let final_line = if total_width > max_width {
                    truncate_line_to_width(line, max_width)
                } else {
                    line
                };

                let width = total_width.min(max_width);
                if width > 0 && current_x + width <= chunk.x + chunk.width {
                    let rect = Rect::new(current_x, area.y, width, 1);
                    f.render_widget(Paragraph::new(final_line), rect);
                    app.layout.tab_bounds.push((rect, p_i, t_i));
                    // Advance: without this every tab overdraws at chunk.x so
                    // only the last tab is visible and click bounds coincide.
                    current_x = current_x.saturating_add(width + 1);
                    global_tab_idx += 1;
                }
            }
            continue;
        }

        for (t_i, tab) in pane.tabs.iter().enumerate() {
            let mut spans = Vec::new();
            let base_name = tab
                .nav.current_path
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
                Style::default().fg(theme::muted())
            };

            if let AppMode::Header(idx) = app.core.mode {
                if idx == global_tab_idx {
                    base_style = base_style
                        .bg(accent_primary())
                        .fg(theme::selection_fg())
                        .add_modifier(Modifier::BOLD);
                }
            }

            spans.push(Span::styled(format!(" {} ", base_name), base_style));

            if matches!(app.core.current_view, CurrentView::Files | CurrentView::Git) {
                if let Some(branch) = &tab.git.git_branch {
                    let pending = tab.git.git_pending.len();
                    let ahead = tab.git.git_ahead;
                    let behind = tab.git.git_behind;

                    let branch_color = if pending > 0 {
                        theme::danger()
                    } else if ahead > 0 || behind > 0 {
                        theme::warning()
                    } else {
                        theme::success()
                    };

                    let mut branch_style = Style::default().fg(branch_color);
                    if is_active_tab && is_focused_pane {
                        branch_style = branch_style.add_modifier(Modifier::BOLD);
                    }

                    spans.push(Span::styled(format!("({})", branch), branch_style));

                    if pending > 0 {
                        spans.push(Span::styled(
                            format!(" +{}", pending),
                            Style::default().fg(theme::danger()),
                        ));
                    }
                    if ahead > 0 {
                        spans.push(Span::styled(
                            format!(" ↑{}", ahead),
                            Style::default().fg(theme::warning()),
                        ));
                    }
                    if behind > 0 {
                        spans.push(Span::styled(
                            format!(" ↓{}", behind),
                            Style::default().fg(theme::warning()),
                        ));
                    }
                    spans.push(Span::raw(" "));
                }
            }

            let line = Line::from(spans.clone());
            let total_width = line.width() as u16;

            // Calculate max available width for this tab (saturating so an
            // exactly-filled previous tab cannot underflow the next one).
            let end_x = chunk.x + chunk.width;
            let max_available = end_x.saturating_sub(current_x);
            if max_available == 0 {
                break;
            }

            // Actually truncate the line content if too wide
            let final_line = if total_width > max_available && max_available > 3 {
                // Build truncated spans
                let mut truncated = vec![];
                let mut current_w = 0;
                for span in spans {
                    let span_w = span.content.width() as u16;
                    if current_w + span_w > max_available - 1 {
                        // Add ellipsis and stop
                        truncated.push(Span::styled("\u{2026}", Style::default().fg(theme::muted())));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::FileColumn;
    use ratatui::backend::TestBackend;

    /// Probe: dump what the header emits for Editor-view tabs on a narrow width.
    #[test]
    fn probe_editor_header_tab_bounds() {
        for (width, panes, sidebar) in [
            (40, 1, false),
            (60, 2, true),
            (120, 2, true),
            (200, 3, true),
        ] {
            let mut app = App::new();
            app.core.current_view = CurrentView::Editor;
            app.sidebar.show_sidebar = sidebar;
            // Force pane count by duplicating panes.
            while app.panes.len() < panes {
                let p = app.panes[0].clone();
                app.panes.push(p);
            }
            for pane in &mut app.panes {
                pane.tabs.clear();
                for name in [
                    "/home/user/Documents-Long-Name-Alpha",
                    "/home/user/src-Beta",
                    "/home/user/Gamma",
                ] {
                    pane.tabs.push(crate::state::FileState::new(
                        std::path::PathBuf::from(name),
                        None,
                        false,
                        app.layout.single_columns.clone(),
                        FileColumn::Name,
                        true,
                    ));
                }
            }
            let backend = TestBackend::new(width, 1);
            let mut terminal = ratatui::Terminal::new(backend).unwrap();
            terminal
                .draw(|f| draw_global_header(f, f.area(), 0, &mut app))
                .unwrap();
            println!(
                "PROBE w={} panes={} sidebar={} -> {}",
                width,
                panes,
                sidebar,
                app.layout
                    .tab_bounds
                    .iter()
                    .map(|(r, p, t)| format!("p{}t{}:x{}w{}", p, t, r.x, r.width))
                    .collect::<Vec<_>>()
                    .join(" ")
            );
        }
    }
}
