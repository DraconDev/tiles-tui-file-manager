#![allow(unused_imports)]

//! Monitor view rendering — system overview, applications, processes.
//! Extracted from ui/mod.rs (Phase 3).

use ratatui::{
    layout::{Alignment, Constraint, Direction, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Cell, Paragraph, Row, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Table,
    },
    Frame,
};

use crate::app::{App, MonitorSubview, ProcessColumn};
use crate::ui::theme as theme;
use dracon_terminal_engine::utils::format_size;

pub fn draw_monitor_page(f: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::default()
        .title_top(Line::from(vec![Span::styled(
            " SYSTEM MONITOR ",
            Style::default()
                .fg(theme::selection_fg())
                .bg(theme::accent_primary())
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
        .border_style(Style::default().fg(theme::accent_primary()));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(inner);

    let nav_area = chunks[0].inner(ratatui::layout::Margin {
        horizontal: 2,
        vertical: 1,
    });
    let nav_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(40), Constraint::Length(50)])
        .split(nav_area);

    let subviews = [
        (MonitorSubview::Overview, "󰊚 OVERVIEW"),
        (MonitorSubview::Applications, "󰀻 APPLICATIONS"),
        (MonitorSubview::Processes, "󰑮 PROCESSES"),
    ];

    app.monitor.monitor_subview_bounds.clear();
    let mut cur_x = nav_layout[0].x;
    for (view, name) in subviews {
        let is_active = app.monitor.monitor_subview == view;
        let width = name.chars().count() as u16 + 4;
        let rect = Rect::new(cur_x, nav_layout[0].y, width, 1);

        let mut style = if is_active {
            Style::default()
                .bg(theme::accent_primary())
                .fg(theme::selection_fg())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::monitor_label())
        };
        if app.core.mouse_pos.1 == nav_layout[0].y
            && app.core.mouse_pos.0 >= rect.x
            && app.core.mouse_pos.0 < rect.x + rect.width
        {
            style = style.fg(theme::fg());
        }

        f.render_widget(Paragraph::new(name).style(style), rect);
        if is_active {
            f.render_widget(
                Paragraph::new("━━━━")
                    .style(Style::default().fg(theme::accent_primary())),
                Rect::new(rect.x, rect.y + 1, 4, 1),
            );
        }

        app.monitor.monitor_subview_bounds.push((rect, view));
        cur_x += width + 2;
    }

    if app.monitor.monitor_subview != MonitorSubview::Overview {
        let tree_indicator = if app.monitor.process_tree_view { " 󰁔 TREE" } else { "" };
        let search_style = if app.monitor.process_search_filter.is_empty() {
            Style::default().fg(theme::border_subtle())
        } else {
            Style::default().fg(theme::accent_primary())
        };
        let tree_style = if app.monitor.process_tree_view {
            Style::default().fg(theme::accent_primary())
        } else {
            Style::default().fg(theme::border_subtle())
        };
        let nav_text = Line::from(vec![
            Span::styled(format!(" 󰍉 {}", app.monitor.process_search_filter), search_style),
            Span::styled(tree_indicator.to_string(), tree_style),
        ]);
        f.render_widget(Paragraph::new(nav_text), nav_layout[1]);
    }

    let content_area = chunks[1].inner(ratatui::layout::Margin {
        horizontal: 1,
        vertical: 1,
    });
    match app.monitor.monitor_subview {
        MonitorSubview::Overview => draw_monitor_overview(f, content_area, app),
        MonitorSubview::Processes => draw_processes_view(f, content_area, app),
        MonitorSubview::Applications => draw_monitor_applications(f, content_area, app),
    }
}

pub fn draw_monitor_overview(f: &mut Frame, area: Rect, app: &mut App) {
    use crate::ui::sparkline::Sparkline;
    use crate::ui::theme;

    let inner = area.inner(ratatui::layout::Margin {
        horizontal: 1,
        vertical: 0,
    });
    let w = inner.width as usize;

    // Helpers
    let gauge_bar = |ratio: f32, width: usize| -> String {
        let ratio = ratio.clamp(0.0, 1.0);
        let filled = (ratio * width as f32) as usize;
        format!("{}{}", "▓".repeat(filled), "░".repeat(width.saturating_sub(filled)))
    };

    let gauge_color = |ratio: f32| -> Color {
        if ratio > 0.85 {
            theme::gauge_danger()
        } else if ratio > 0.5 {
            theme::gauge_warning()
        } else {
            theme::accent_secondary()
        }
    };

    let sep = theme::monitor_separator();
    let label = theme::monitor_label();
    let dim = theme::monitor_dim();

    let mut lines: Vec<Line<'static>> = Vec::new();

    // ╭── CPU ───────────────────────────────────────────────────╮
    lines.push(Line::from(vec![
        Span::styled("╭─ ", sep),
        Span::styled("CPU USAGE", Style::default().fg(label).add_modifier(Modifier::BOLD)),
    ]));

    let cpu_ratio = (app.system_state.cpu_usage / 100.0).clamp(0.0, 1.0);
    let cpu_color = gauge_color(cpu_ratio);
    let bar_w = w.saturating_sub(12);
    lines.push(Line::from(vec![
        Span::styled("│  ", sep),
        Span::styled(gauge_bar(cpu_ratio, bar_w), Style::default().fg(cpu_color)),
        Span::styled(format!("  {:>3.0}%", app.system_state.cpu_usage), Style::default().fg(theme::fg()).add_modifier(Modifier::BOLD)),
    ]));

    // CPU sparkline
    if !app.system_state.cpu_history.is_empty() {
        let spark = Sparkline::new(app.system_state.cpu_history.iter().copied(), bar_w).color(cpu_color).render();
        lines.push(Line::from(vec![Span::raw("│  "), Span::styled(spark.to_string(), Style::default().fg(cpu_color))]));
    }

    // Temperature and frequency
    let mut sys_info_parts: Vec<Span<'static>> = vec![Span::styled("│  ", sep)];
    if let Some(temp) = app.system_state.cpu_temperature {
        sys_info_parts.push(Span::styled(format!("{:>4.0}°C  ", temp), Style::default().fg(dim)));
    }
    if let Some(freq) = app.system_state.cpu_frequency {
        sys_info_parts.push(Span::styled(format!("{:>4.1} GHz", freq), Style::default().fg(theme::fg())));
    }
    if sys_info_parts.len() > 1 {
        lines.push(Line::from(sys_info_parts));
    }

    lines.push(Line::from(Span::styled("│", sep)));

    // Per-core bars — 2 per row with 2-space gap
    let core_count = app.system_state.cpu_cores.len();
    if core_count > 0 {
        let cols = 2usize;
        let core_bar_w = ((bar_w.saturating_sub(2)) / cols).saturating_sub(8);
        for row in 0..core_count.div_ceil(cols) {
            let mut spans: Vec<Span<'static>> = vec![Span::styled("│  ", sep)];
            for c in 0..cols {
                let idx = row * cols + c;
                if idx < core_count {
                    let usage = app.system_state.cpu_cores[idx];
                    let ratio = (usage / 100.0).clamp(0.0, 1.0);
                    let color = gauge_color(ratio);
                    let bar = gauge_bar(ratio, core_bar_w);
                    spans.push(Span::styled(
                        format!("{:>2} {} {:>3.0}%", idx, bar, usage),
                        Style::default().fg(color),
                    ));
                    if c + 1 < cols && idx + 1 < core_count {
                        spans.push(Span::raw("  "));
                    }
                }
            }
            lines.push(Line::from(spans));
        }
    }

    lines.push(Line::from(vec![
        Span::styled("╰", sep),
        Span::styled("─".repeat(w.saturating_sub(2)), sep),
        Span::styled("╯", sep),
    ]));
    lines.push(Line::from(""));

    // ╭── MEMORY ────────────────────────────────────────────────╮
    lines.push(Line::from(vec![
        Span::styled("╭─ ", sep),
        Span::styled("MEMORY", Style::default().fg(label).add_modifier(Modifier::BOLD)),
    ]));

    let mem_ratio = if app.system_state.total_mem > 0.0 {
        (app.system_state.mem_usage / app.system_state.total_mem).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let mem_color = gauge_color(mem_ratio);
    let mem_bar_w = w.saturating_sub(28);
    lines.push(Line::from(vec![
        Span::styled("│  ", sep),
        Span::styled("RAM ", Style::default().fg(label)),
        Span::styled(gauge_bar(mem_ratio, mem_bar_w), Style::default().fg(mem_color)),
        Span::styled(
            format!("  {:.1}G / {:.1}G [{:.0}%]", app.system_state.mem_usage, app.system_state.total_mem, mem_ratio * 100.0),
            Style::default().fg(theme::fg()),
        ),
    ]));

    let swp_ratio = if app.system_state.total_swap > 0.0 {
        (app.system_state.swap_usage / app.system_state.total_swap).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let swp_color = gauge_color(swp_ratio);
    lines.push(Line::from(vec![
        Span::styled("│  ", sep),
        Span::styled("SWP ", Style::default().fg(label)),
        Span::styled(gauge_bar(swp_ratio, mem_bar_w), Style::default().fg(swp_color)),
        Span::styled(
            format!("  {:.1}G / {:.1}G [{:.0}%]", app.system_state.swap_usage, app.system_state.total_swap, swp_ratio * 100.0),
            Style::default().fg(theme::fg()),
        ),
    ]));

    if !app.system_state.mem_history.is_empty() {
        let spark = Sparkline::new(app.system_state.mem_history.iter().copied(), mem_bar_w).color(mem_color).render();
        lines.push(Line::from(vec![Span::raw("│  "), Span::styled(spark.to_string(), Style::default().fg(mem_color))]));
    }

    lines.push(Line::from(vec![
        Span::styled("╰", sep),
        Span::styled("─".repeat(w.saturating_sub(2)), sep),
        Span::styled("╯", sep),
    ]));
    lines.push(Line::from(""));

    // ╭── DISK STORAGE ──────────────────────────────────────────╮
    lines.push(Line::from(vec![
        Span::styled("╭─ ", sep),
        Span::styled("DISK STORAGE", Style::default().fg(label).add_modifier(Modifier::BOLD)),
    ]));

    let disk_bar_w = w.saturating_sub(32);
    for disk in &app.system_state.disks {
        let ratio = (disk.used_space / disk.total_space).clamp(0.0, 1.0);
        let color = if ratio > 0.9 {
            theme::gauge_danger()
        } else if ratio > 0.7 {
            theme::gauge_warning()
        } else {
            theme::accent_secondary()
        };
        lines.push(Line::from(vec![
            Span::styled("│  ", sep),
            Span::styled(format!("{:14}", disk.name), Style::default().fg(theme::fg())),
            Span::styled(gauge_bar(ratio as f32, disk_bar_w), Style::default().fg(color)),
            Span::styled(
                format!("  {:.0}G / {:.0}G [{:.0}%]", disk.used_space, disk.total_space, ratio * 100.0),
                Style::default().fg(dim),
            ),
        ]));
    }

    lines.push(Line::from(vec![
        Span::styled("╰", sep),
        Span::styled("─".repeat(w.saturating_sub(2)), sep),
        Span::styled("╯", sep),
    ]));
    lines.push(Line::from(""));

    // ╭── DISK I/O ──────────────────────────────────────────────╮
    lines.push(Line::from(vec![
        Span::styled("╭─ ", sep),
        Span::styled("DISK I/O", Style::default().fg(label).add_modifier(Modifier::BOLD)),
    ]));

    let disk_read_rate = app.system_state.disk_read_history.back().copied().unwrap_or(0);
    let disk_write_rate = app.system_state.disk_write_history.back().copied().unwrap_or(0);
    let read_mbps = disk_read_rate as f64 / 100.0;
    let write_mbps = disk_write_rate as f64 / 100.0;

    lines.push(Line::from(vec![
        Span::styled("│  ", sep),
        Span::styled("R ▶ ", Style::default().fg(theme::accent_secondary())),
        Span::styled(format!("{:>6.1} MB/s", read_mbps), Style::default().fg(theme::fg()).add_modifier(Modifier::BOLD)),
        Span::raw("     "),
        Span::styled("W ◀ ", Style::default().fg(theme::accent_primary())),
        Span::styled(format!("{:>6.1} MB/s", write_mbps), Style::default().fg(theme::fg()).add_modifier(Modifier::BOLD)),
    ]));

    let half_w = (w.saturating_sub(6)) / 2;
    if !app.system_state.disk_read_history.is_empty() && !app.system_state.disk_write_history.is_empty() {
        let r_spark = Sparkline::new(app.system_state.disk_read_history.iter().copied(), half_w).color(theme::accent_secondary()).render();
        let w_spark = Sparkline::new(app.system_state.disk_write_history.iter().copied(), half_w).color(theme::accent_primary()).render();
        lines.push(Line::from(vec![
            Span::raw("│  "),
            Span::styled(r_spark.to_string(), Style::default().fg(theme::accent_secondary())),
            Span::raw("  "),
            Span::styled(w_spark.to_string(), Style::default().fg(theme::accent_primary())),
        ]));
    }

    lines.push(Line::from(vec![
        Span::styled("╰", sep),
        Span::styled("─".repeat(w.saturating_sub(2)), sep),
        Span::styled("╯", sep),
    ]));
    lines.push(Line::from(""));

    // ╭── NETWORK ───────────────────────────────────────────────╮
    lines.push(Line::from(vec![
        Span::styled("╭─ ", sep),
        Span::styled("NETWORK", Style::default().fg(label).add_modifier(Modifier::BOLD)),
    ]));

    let rx = app.system_state.net_in_history.back().copied().unwrap_or(0);
    let tx = app.system_state.net_out_history.back().copied().unwrap_or(0);

    lines.push(Line::from(vec![
        Span::styled("│  ", sep),
        Span::styled("RX ▼ ", Style::default().fg(theme::accent_secondary())),
        Span::styled(format_size(rx), Style::default().fg(theme::fg()).add_modifier(Modifier::BOLD)),
        Span::raw("     "),
        Span::styled("TX ▲ ", Style::default().fg(theme::accent_primary())),
        Span::styled(format_size(tx), Style::default().fg(theme::fg()).add_modifier(Modifier::BOLD)),
    ]));

    let half_w = (w.saturating_sub(6)) / 2;
    if !app.system_state.net_in_history.is_empty() && !app.system_state.net_out_history.is_empty() {
        let rx_spark = Sparkline::new(app.system_state.net_in_history.iter().copied(), half_w).color(theme::accent_secondary()).render();
        let tx_spark = Sparkline::new(app.system_state.net_out_history.iter().copied(), half_w).color(theme::accent_primary()).render();
        lines.push(Line::from(vec![
            Span::raw("│  "),
            Span::styled(rx_spark.to_string(), Style::default().fg(theme::accent_secondary())),
            Span::raw("  "),
            Span::styled(tx_spark.to_string(), Style::default().fg(theme::accent_primary())),
        ]));
    }

    lines.push(Line::from(vec![
        Span::styled("│  ", sep),
        Span::styled("Total ↓ ", Style::default().fg(dim)),
        Span::styled(format_size(app.system_state.net_in), Style::default().fg(theme::fg())),
        Span::raw("   ↑ "),
        Span::styled(format_size(app.system_state.net_out), Style::default().fg(theme::fg())),
    ]));

    for iface in &app.system_state.net_interfaces {
        let rx_rate_kbps = iface.rx_rate as f64 / 1024.0;
        let tx_rate_kbps = iface.tx_rate as f64 / 1024.0;
        let rx_display = if rx_rate_kbps > 1024.0 {
            format!("{:.1} MB/s", rx_rate_kbps / 1024.0)
        } else {
            format!("{:.0} KB/s", rx_rate_kbps)
        };
        let tx_display = if tx_rate_kbps > 1024.0 {
            format!("{:.1} MB/s", tx_rate_kbps / 1024.0)
        } else {
            format!("{:.0} KB/s", tx_rate_kbps)
        };
        lines.push(Line::from(vec![
            Span::styled("│  ", sep),
            Span::styled(format!("{:10}", iface.name), Style::default().fg(theme::fg())),
            Span::styled(" ▼ ", Style::default().fg(theme::accent_secondary())),
            Span::styled(format!("{:>10}", rx_display), Style::default().fg(theme::accent_secondary())),
            Span::styled(" ▲ ", Style::default().fg(theme::accent_primary())),
            Span::styled(format!("{:>10}", tx_display), Style::default().fg(theme::accent_primary())),
        ]));

        let iface_half_w = (w.saturating_sub(6)) / 2;
        if !iface.rx_history.is_empty() && !iface.tx_history.is_empty() {
            let r_spark = Sparkline::new(iface.rx_history.iter().copied(), iface_half_w).color(theme::accent_secondary()).render();
            let t_spark = Sparkline::new(iface.tx_history.iter().copied(), iface_half_w).color(theme::accent_primary()).render();
            lines.push(Line::from(vec![
                Span::styled("│  ", sep),
                Span::styled(format!("{:10}", ""), Style::default().fg(theme::fg())),
                Span::styled(r_spark.to_string(), Style::default().fg(theme::accent_secondary())),
                Span::raw("  "),
                Span::styled(t_spark.to_string(), Style::default().fg(theme::accent_primary())),
            ]));
        }
    }

    lines.push(Line::from(vec![
        Span::styled("╰", sep),
        Span::styled("─".repeat(w.saturating_sub(2)), sep),
        Span::styled("╯", sep),
    ]));
    lines.push(Line::from(""));

    // ╭── SYSTEM ────────────────────────────────────────────────╮
    lines.push(Line::from(vec![
        Span::styled("╭─ ", sep),
        Span::styled("SYSTEM", Style::default().fg(label).add_modifier(Modifier::BOLD)),
    ]));

    let info = format!(
        "{} │ up {}d {}h │ {} │ {}",
        app.system_state.hostname,
        app.system_state.uptime / 86400,
        (app.system_state.uptime % 86400) / 3600,
        app.system_state.kernel_version,
        app.system_state.os_name,
    );
    lines.push(Line::from(vec![
        Span::styled("│  ", sep),
        Span::styled(info, Style::default().fg(theme::fg())),
    ]));

    lines.push(Line::from(vec![
        Span::styled("╰", sep),
        Span::styled("─".repeat(w.saturating_sub(2)), sep),
        Span::styled("╯", sep),
    ]));

    // ── Scroll handling ──
    let total_lines = lines.len() as u16;
    let visible_lines = inner.height;
    let max_scroll = total_lines.saturating_sub(visible_lines);
    app.monitor.overview_scroll_offset = app.monitor.overview_scroll_offset.min(max_scroll);

    let start = app.monitor.overview_scroll_offset as usize;
    let end = (start + visible_lines as usize).min(lines.len());
    let visible: Vec<Line> = lines[start..end].to_vec();

    for (i, line) in visible.into_iter().enumerate() {
        let y = inner.y + i as u16;
        if y < inner.y + inner.height {
            f.render_widget(Paragraph::new(line), Rect::new(inner.x, y, inner.width, 1));
        }
    }

    // Scrollbar
    if total_lines > visible_lines {
        let mut state = ScrollbarState::new(total_lines as usize)
            .position(app.monitor.overview_scroll_offset as usize);
        f.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            area,
            &mut state,
        );
    }
}

pub fn draw_monitor_applications(f: &mut Frame, area: Rect, app: &mut App) {
    // Never hard-code a person: an unset USER must not silently filter the
    // Applications list to someone else's processes.
    let current_user = std::env::var("USER").unwrap_or_default();
    let mut app_procs: Vec<_> = app.system_state
        .processes
        .iter()
        .filter(|p| {
            let matches = if app.monitor.process_search_filter.is_empty() {
                true
            } else {
                p.name
                    .to_lowercase()
                    .contains(&app.monitor.process_search_filter.to_lowercase())
            };
            p.user == current_user
                && !p.name.starts_with('[')
                && !p.name.contains("kworker")
                && matches
        })
        .cloned()
        .collect();

    if app.monitor.process_tree_view {
        crate::modules::system::tree_sort_processes(&mut app_procs, &app.system_state.process_ppid);
    }

    let rows = app_procs.iter().enumerate().map(|(i, p)| {
        let mut is_selected = false;
        let mut style = if i % 2 == 0 {
            Style::default().fg(theme::monitor_row_even())
        } else {
            Style::default().fg(theme::monitor_row_odd())
        };
        if app.monitor.process_selected_idx == Some(i)
            && app.monitor.monitor_subview == MonitorSubview::Applications
        {
            style = style
                .bg(theme::accent_primary())
                .fg(theme::selection_fg())
                .add_modifier(Modifier::BOLD);
            is_selected = true;
        }
        let cpu_color = if is_selected {
            theme::selection_fg()
        } else if p.cpu > 50.0 {
            theme::danger()
        } else {
            theme::accent_secondary()
        };
        let depth = if app.monitor.process_tree_view {
            crate::modules::system::process_tree_depth(p.pid, &app.system_state.process_ppid)
        } else {
            0
        };
        let indent = "  ".repeat(depth.min(8));
        let prefix = if depth > 0 { "└ " } else { "" };
        let name_display = if app.monitor.process_tree_view {
            format!("{}{}{}", indent, prefix, p.name)
        } else {
            p.name.clone()
        };
        Row::new(vec![
            Cell::from(format!("  {}", name_display)),
            Cell::from(format!("{:.1}%", p.cpu)).style(Style::default().fg(cpu_color)),
            Cell::from(format!("{:.1} MB", p.mem)),
            Cell::from(p.pid.to_string()).style(Style::default().fg(if is_selected {
                theme::selection_fg()
            } else {
                theme::monitor_label()
            })),
            Cell::from(p.status.clone()),
        ])
        .style(style)
    });
    let column_constraints = [
        Constraint::Min(35),
        Constraint::Length(10),
        Constraint::Length(15),
        Constraint::Length(10),
        Constraint::Length(15),
    ];
    let header_rects = header_rects_for(area, &column_constraints, APPLICATION_COLUMN_SPACING);

    app.monitor.process_column_bounds.clear();
    let header_cells = [
        ("  Application", ProcessColumn::Name),
        ("CPU", ProcessColumn::Cpu),
        ("Memory", ProcessColumn::Mem),
        ("PID", ProcessColumn::Pid),
        ("Status", ProcessColumn::Status),
    ]
    .iter()
    .enumerate()
    .map(|(i, (h, col))| {
        let rect = header_rects[i];
        app.monitor.process_column_bounds
            .push((Rect::new(rect.x, area.y, rect.width, 1), *col));
        let mut text = h.to_string();
        if app.monitor.process_sort_col == *col {
            text.push_str(if app.monitor.process_sort_asc {
                " 󰁝"
            } else {
                " 󰁅"
            });
        }
        Cell::from(text).style(
            Style::default()
                .fg(if app.monitor.process_sort_col == *col {
                    theme::accent_primary()
                } else {
                    theme::monitor_label()
                })
                .add_modifier(Modifier::BOLD),
        )
    });

    // Stateful so long lists scroll (the previous plain render_widget
    // clipped everything past the first screenful).
    if let Some(sel) = app.monitor.process_selected_idx {
        app.monitor.application_table_state
            .select(Some(sel.min(app_procs.len().saturating_sub(1))));
    }
    f.render_stateful_widget(
        Table::new(rows, column_constraints)
            .header(Row::new(header_cells).height(1).bottom_margin(1))
            .column_spacing(APPLICATION_COLUMN_SPACING),
        area,
        &mut app.monitor.application_table_state,
    );
}

/// Spacing between process/application columns. The header hit boxes and the
/// rendered table must use the SAME value: the header used 2 while the table
/// rendered with 1, so every clickable band drifted away from the text it
/// represented (5 cells by the last column, more once the flexible column
/// absorbed the extra width).
pub const PROCESS_COLUMN_SPACING: u16 = 1;

/// Spacing the applications table renders with; the header hit boxes use it too.
pub const APPLICATION_COLUMN_SPACING: u16 = 2;

/// Header rectangles for a monitor table, laid out exactly like that table's
/// own columns so a click lands on the column the user sees.
///
/// `spacing` must be the same value the table is rendered with.
pub fn header_rects_for(area: Rect, constraints: &[Constraint], spacing: u16) -> Vec<Rect> {
    // Mirror ratatui's `Table::get_columns_widths` exactly (ratatui-0.29
    // `widgets/table/table.rs`): it solves the columns over a ONE-ROW rect at
    // the origin — `Rect::new(0, 0, max_width, 1)` — and then paints each cell
    // at `table_area.x + column_x`. So the render height never reaches the
    // solver, and neither does `area.y`.
    //
    // Two things must match for the bands to sit on the painted captions:
    //   * the height-1 rect. `Layout` rounds its joint solution against the
    //     rect it is given, so solving over the full render area shifts a
    //     boundary by a cell at some widths (e.g. table width 80 put the
    //     STATUS band one cell right of "STATUS", table width 82 put it two
    //     cells right, inside the USER band).
    //   * `Flex::Start`, which is what `Table` uses by default; `Layout`'s own
    //     default is `Flex::Legacy`, which distributes leftover space into the
    //     last column instead of leaving it at the end.
    //
    // These tables set no `highlight_symbol`, so ratatui reserves no selection
    // gutter (`selection_width == 0`) and the column solve spans the full
    // width — the intermediate `[Length(0), Fill(0)]` split collapses to that.
    let columns_area = Rect::new(0, 0, area.width, 1);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints.to_vec())
        .flex(Flex::Start)
        .spacing(spacing)
        .split(columns_area)
        .iter()
        .map(|r| Rect::new(area.x.saturating_add(r.x), area.y, r.width, 1))
        .collect()
}

pub fn draw_processes_view(f: &mut Frame, area: Rect, app: &mut App) {
    let mut procs = app.system_state.processes.clone();
    if app.monitor.process_tree_view {
        crate::modules::system::tree_sort_processes(&mut procs, &app.system_state.process_ppid);
    }
    let column_constraints = [
        Constraint::Length(8),
        Constraint::Min(25),
        Constraint::Length(15),
        Constraint::Length(12),
        Constraint::Length(10),
        Constraint::Length(10),
    ];
    let header_rects = header_rects_for(area, &column_constraints, PROCESS_COLUMN_SPACING);

    app.monitor.process_column_bounds.clear();
    let header_cells = ["PID", "NAME", "USER", "STATUS", "CPU%", "MEM%"]
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let col = match *h {
                "PID" => ProcessColumn::Pid,
                "NAME" => ProcessColumn::Name,
                "USER" => ProcessColumn::User,
                "STATUS" => ProcessColumn::Status,
                "CPU%" => ProcessColumn::Cpu,
                "MEM%" => ProcessColumn::Mem,
                _ => ProcessColumn::Pid,
            };
            // Use the layout's own rects; accumulating widths by hand drifts
            // away from them as soon as flex or spacing change.
            let rect = header_rects[i];
            app.monitor.process_column_bounds
                .push((Rect::new(rect.x, area.y, rect.width, 1), col));
            let mut text = h.to_string();
            if app.monitor.process_sort_col == col {
                text.push_str(if app.monitor.process_sort_asc {
                    " 󰁝"
                } else {
                    " 󰁅"
                });
            }
            Cell::from(text).style(
                Style::default()
                    .fg(if app.monitor.process_sort_col == col {
                        theme::accent_primary()
                    } else {
                        theme::monitor_label()
                    })
                    .add_modifier(Modifier::BOLD),
            )
        });
    let rows = procs.iter().enumerate().map(|(i, p)| {
        let mut is_selected = false;
        let mut style = if i % 2 == 0 {
            Style::default().fg(theme::monitor_row_even())
        } else {
            Style::default().fg(theme::monitor_row_odd())
        };
        if app.monitor.process_selected_idx == Some(i) && app.monitor.monitor_subview == MonitorSubview::Processes {
            style = style
                .bg(theme::accent_primary())
                .fg(theme::selection_fg())
                .add_modifier(Modifier::BOLD);
            is_selected = true;
        }
        let cpu_color = if is_selected {
            theme::selection_fg()
        } else if p.cpu > 50.0 {
            theme::danger()
        } else {
            theme::accent_secondary()
        };
        let depth = if app.monitor.process_tree_view {
            crate::modules::system::process_tree_depth(p.pid, &app.system_state.process_ppid)
        } else {
            0
        };
        let indent = "  ".repeat(depth.min(8));
        let prefix = if depth > 0 { "└ " } else { "" };
        let name_display = if app.monitor.process_tree_view {
            format!("{}{}{}", indent, prefix, p.name)
        } else {
            p.name.clone()
        };
        Row::new(vec![
            Cell::from(format!("  {}", p.pid)).style(Style::default().fg(if is_selected {
                theme::selection_fg()
            } else {
                theme::monitor_label()
            })),
            Cell::from(name_display).style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from(p.user.clone()).style(Style::default().fg(if is_selected {
                theme::selection_fg()
            } else {
                theme::accent_primary()
            })),
            Cell::from(p.status.clone()),
            Cell::from(format!("{:.1}", p.cpu)).style(Style::default().fg(cpu_color)),
            Cell::from(format!("{:.1}", p.mem)),
        ])
        .style(style)
    });
    f.render_stateful_widget(
        Table::new(rows, column_constraints)
            .header(Row::new(header_cells).height(1).bottom_margin(1))
            .column_spacing(PROCESS_COLUMN_SPACING),
        area,
        &mut app.monitor.process_table_state,
    );
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{MonitorSubview, ProcessColumn};
    use ratatui::backend::TestBackend;
    use ratatui::widgets::{Cell, Row, Table};
    use ratatui::Terminal;

    fn process_constraints() -> [Constraint; 6] {
        [
            Constraint::Length(8),
            Constraint::Min(25),
            Constraint::Length(15),
            Constraint::Length(12),
            Constraint::Length(10),
            Constraint::Length(10),
        ]
    }

    fn application_constraints() -> [Constraint; 5] {
        [
            Constraint::Min(35),
            Constraint::Length(10),
            Constraint::Length(15),
            Constraint::Length(10),
            Constraint::Length(15),
        ]
    }

    /// Concatenate the glyphs painted on one buffer row, so a caption can be
    /// located at the x where the table actually drew it.
    fn painted_row(term: &Terminal<TestBackend>, y: u16, x0: u16, x1: u16) -> String {
        let buf = term.backend().buffer();
        (x0..x1)
            .map(|x| buf[(x, y)].symbol().to_string())
            .collect()
    }

    /// x at which `caption` is painted on row `y`, searching from `from`.
    fn painted_caption_x(
        term: &Terminal<TestBackend>,
        y: u16,
        from: u16,
        to: u16,
        caption: &str,
    ) -> Option<u16> {
        painted_row(term, y, from, to)
            .find(caption)
            .map(|i| from + i as u16)
    }

    /// Painted x of every column's header caption, read out of a REAL ratatui
    /// `Table` render of the same area/constraints/spacing. Each column carries a
    /// distinct single-glyph header instead of its caption so the probe does not
    /// depend on caption length, but the column solve is the one the table
    /// itself performs — no `Layout` re-derivation participates.
    ///
    /// `None` means the column solved to zero width and painted nothing.
    fn table_painted_column_xs(
        area: Rect,
        constraints: &[Constraint],
        spacing: u16,
    ) -> Vec<Option<u16>> {
        let markers: Vec<String> = (0..constraints.len())
            .map(|i| ((b'a' + i as u8) as char).to_string())
            .collect();
        let header = Row::new(markers.iter().cloned().map(Cell::from).collect::<Vec<_>>())
            .height(1)
            .bottom_margin(1);
        // Same widget shape the monitor views use: header row, no highlight
        // symbol, spacing supplied by the caller.
        let table = Table::new(Vec::<Row>::new(), constraints.to_vec())
            .header(header)
            .column_spacing(spacing);
        let backend = TestBackend::new(area.x + area.width + 2, area.y + area.height + 2);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| f.render_widget(table, area)).unwrap();

        let mut cursor = 0u16;
        markers
            .iter()
            .map(|m| {
                let found = painted_caption_x(&term, area.y, area.x + cursor, area.x + area.width, m);
                if let Some(x) = found {
                    cursor = x - area.x + 1;
                }
                found
            })
            .collect()
    }

    /// The header hit boxes must start exactly where the table paints that
    /// column's glyph. They used to be re-derived separately (spacing 2 vs the
    /// table's 1, then a full-area `Layout` solve instead of ratatui's height-1
    /// one), so they drifted right of the columns and clicks landed on the wrong
    /// header — 1-2 cells at the widths audited below.
    #[test]
    fn process_header_boxes_match_the_painted_table_columns() {
        // 80..=84 is the band where the old full-area solve drifted off the
        // captions; 40 and 260 bound the extremes.
        for width in 40u16..=260 {
            for origin in [(0u16, 0u16), (2, 5), (3, 1)] {
                for height in [1u16, 12, 20] {
                    let area = Rect::new(origin.0, origin.1, width, height);
                    let header =
                        header_rects_for(area, &process_constraints(), PROCESS_COLUMN_SPACING);
                    let painted =
                        table_painted_column_xs(area, &process_constraints(), PROCESS_COLUMN_SPACING);

                    assert_eq!(header.len(), painted.len(), "one box per column");
                    for (i, (rect, painted_x)) in header.iter().zip(painted.iter()).enumerate() {
                        let Some(px) = painted_x else {
                            assert_eq!(rect.width, 0, "col {i} painted nothing but has width");
                            continue;
                        };
                        assert_eq!(
                            rect.x, *px,
                            "col {i} box x vs painted x at area {area:?}"
                        );
                        assert_eq!(rect.y, area.y, "col {i} sits on the header row");
                        assert_eq!(rect.height, 1, "col {i} is one row");
                    }
                }
            }
        }
    }

    /// Same invariant for the applications table, which renders with spacing 2.
    #[test]
    fn application_header_boxes_match_the_painted_table_columns() {
        for width in 40u16..=260 {
            for origin in [(0u16, 0u16), (2, 5)] {
                for height in [1u16, 20] {
                    let area = Rect::new(origin.0, origin.1, width, height);
                    let header = header_rects_for(
                        area,
                        &application_constraints(),
                        APPLICATION_COLUMN_SPACING,
                    );
                    let painted = table_painted_column_xs(
                        area,
                        &application_constraints(),
                        APPLICATION_COLUMN_SPACING,
                    );
                    assert_eq!(header.len(), painted.len(), "one box per column");
                    for (i, (rect, painted_x)) in header.iter().zip(painted.iter()).enumerate() {
                        let Some(px) = painted_x else {
                            assert_eq!(rect.width, 0, "col {i} painted nothing but has width");
                            continue;
                        };
                        assert_eq!(
                            rect.x, *px,
                            "col {i} box x vs painted x at area {area:?}"
                        );
                    }
                }
            }
        }
    }

    /// `draw_processes_view` records its bands from `header_rects_for`; those
    /// bands must sit on the captions that the table in the same render paints.
    /// This drives the renderer's own call site through a `TestBackend`, so it
    /// fails if the helper and the table ever solve their columns differently.
    #[test]
    fn rendered_process_header_bands_line_up_with_the_table_columns() {
        let captions = ["PID", "NAME", "USER", "STATUS", "CPU%", "MEM%"];
        // Table-area widths 80..=84 are the ones the audited defect hit; the
        // rest bound the sweep.
        for width in [40u16, 60, 76, 78, 79, 80, 81, 82, 83, 84, 85, 100, 120, 200, 260] {
            for origin in [(0u16, 0u16), (2, 5)] {
                let area = Rect::new(origin.0, origin.1, width, 20);
                let mut app = App::new();
                app.monitor.monitor_subview = MonitorSubview::Processes;
                app.system_state.processes = Vec::new();

                let backend = TestBackend::new(area.x + width + 2, area.y + area.height);
                let mut term = Terminal::new(backend).unwrap();
                term.draw(|f| draw_processes_view(f, area, &mut app))
                    .unwrap();

                let recorded = app.monitor.process_column_bounds.clone();
                assert_eq!(recorded.len(), captions.len(), "one band per column");

                let mut cursor = area.x;
                for ((rect, col), caption) in recorded.iter().zip(captions) {
                    let painted_x = painted_caption_x(
                        &term,
                        area.y,
                        cursor,
                        area.x + width,
                        caption,
                    )
                    .unwrap_or_else(|| panic!("{caption} not painted at width {width}"));
                    cursor = painted_x;
                    assert_eq!(
                        rect.x, painted_x,
                        "{caption:?} ({col:?}) band x vs painted caption at width {width}, \
                         origin {origin:?}"
                    );
                    assert_eq!(rect.y, area.y, "{caption:?} band sits on the header row");
                    assert_eq!(rect.height, 1, "{caption:?} band is one row");
                }
            }
        }
    }

    /// Same check for the applications view, whose captions and spacing differ.
    #[test]
    fn rendered_application_header_bands_line_up_with_the_table_columns() {
        let captions = ["Application", "CPU", "Memory", "PID", "Status"];
        for width in [60u16, 80, 82, 100, 120, 200] {
            for origin in [(0u16, 0u16), (2, 5)] {
                let area = Rect::new(origin.0, origin.1, width, 18);
                let mut app = App::new();
                app.monitor.monitor_subview = MonitorSubview::Applications;
                app.system_state.processes = Vec::new();

                let backend = TestBackend::new(area.x + width + 2, area.y + area.height);
                let mut term = Terminal::new(backend).unwrap();
                term.draw(|f| draw_monitor_applications(f, area, &mut app))
                    .unwrap();

                let recorded = app.monitor.process_column_bounds.clone();
                assert_eq!(recorded.len(), captions.len(), "one band per column");
                let mut cursor = area.x;
                for ((rect, col), caption) in recorded.iter().zip(captions) {
                    let painted_x =
                        painted_caption_x(&term, area.y, cursor, area.x + width, caption)
                            .unwrap_or_else(|| panic!("{caption} not painted at width {width}"));
                    cursor = painted_x;
                    assert_eq!(
                        rect.x, painted_x,
                        "{caption:?} ({col:?}) band x vs painted caption at width {width}, \
                         origin {origin:?}"
                    );
                }
            }
        }
    }

    /// Clicking the first glyph of a header the user can see must select THAT
    /// column. The click points come from the rendered buffer, so this cannot
    /// pass by re-deriving the same geometry it is checking.
    #[test]
    fn a_click_on_each_painted_header_selects_that_column() {
        let captions = ["PID", "NAME", "USER", "STATUS", "CPU%", "MEM%"];
        let expected = [
            ProcessColumn::Pid,
            ProcessColumn::Name,
            ProcessColumn::User,
            ProcessColumn::Status,
            ProcessColumn::Cpu,
            ProcessColumn::Mem,
        ];
        // Real terminal sizes the audited drift reproduced at (84 and 86 lose
        // the table's own border columns, hence widths 82 and 84 here) plus
        // wider references.
        for (width, height) in [(82u16, 11u16), (84, 17), (80, 18), (120, 24)] {
            for origin in [(0u16, 0u16), (2, 5)] {
                let area = Rect::new(origin.0, origin.1, width, height);
                let mut app = App::new();
                app.monitor.monitor_subview = MonitorSubview::Processes;
                app.system_state.processes = Vec::new();

                let backend = TestBackend::new(area.x + width + 2, area.y + height);
                let mut term = Terminal::new(backend).unwrap();
                term.draw(|f| draw_processes_view(f, area, &mut app))
                    .unwrap();

                let bands = app.monitor.process_column_bounds.clone();
                let mut cursor = area.x;
                for (i, caption) in captions.iter().enumerate() {
                    let click_x = painted_caption_x(&term, area.y, cursor, area.x + width, caption)
                        .unwrap_or_else(|| panic!("{caption} not painted at {area:?}"));
                    cursor = click_x;
                    let hit = bands
                        .iter()
                        .find(|(r, _)| {
                            r.contains(ratatui::layout::Position {
                                x: click_x,
                                y: area.y,
                            })
                        })
                        .map(|(_, c)| *c);
                    assert_eq!(
                        hit,
                        Some(expected[i]),
                        "clicking the painted {caption:?} at x={click_x} (area {area:?}) \
                         must select {expected:?}, got {hit:?} from bands {:?}",
                        bands
                            .iter()
                            .map(|(r, c)| (*c, r.x, r.x + r.width))
                            .collect::<Vec<_>>()
                    );
                }
            }
        }
    }
}
