#![allow(unused_imports)]

//! Monitor view rendering — system overview, applications, processes.
//! Extracted from ui/mod.rs (Phase 3).

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
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
                .fg(Color::Black)
                .bg(theme::accent_primary())
                .add_modifier(Modifier::BOLD),
        )]))
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
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Rgb(60, 65, 75))
        };
        if app.core.mouse_pos.1 == nav_layout[0].y
            && app.core.mouse_pos.0 >= rect.x
            && app.core.mouse_pos.0 < rect.x + rect.width
        {
            style = style.fg(Color::White);
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
            Style::default().fg(Color::Rgb(40, 45, 55))
        } else {
            Style::default().fg(theme::accent_primary())
        };
        let tree_style = if app.monitor.process_tree_view {
            Style::default().fg(theme::accent_primary())
        } else {
            Style::default().fg(Color::Rgb(40, 45, 55))
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
        Span::styled(format!("  {:>3.0}%", app.system_state.cpu_usage), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
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
        sys_info_parts.push(Span::styled(format!("{:>4.1} GHz", freq), Style::default().fg(Color::White)));
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
        Span::styled("─".repeat(w), sep),
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
            Style::default().fg(Color::White),
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
            Style::default().fg(Color::White),
        ),
    ]));

    if !app.system_state.mem_history.is_empty() {
        let spark = Sparkline::new(app.system_state.mem_history.iter().copied(), mem_bar_w).color(mem_color).render();
        lines.push(Line::from(vec![Span::raw("│  "), Span::styled(spark.to_string(), Style::default().fg(mem_color))]));
    }

    lines.push(Line::from(vec![
        Span::styled("╰", sep),
        Span::styled("─".repeat(w), sep),
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
            Span::styled(format!("{:14}", &disk.name), Style::default().fg(Color::White)),
            Span::styled(gauge_bar(ratio as f32, disk_bar_w), Style::default().fg(color)),
            Span::styled(
                format!("  {:.0}G / {:.0}G [{:.0}%]", disk.used_space, disk.total_space, ratio * 100.0),
                Style::default().fg(dim),
            ),
        ]));
    }

    lines.push(Line::from(vec![
        Span::styled("╰", sep),
        Span::styled("─".repeat(w), sep),
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
        Span::styled(format!("{:>6.1} MB/s", read_mbps), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::raw("     "),
        Span::styled("W ◀ ", Style::default().fg(theme::accent_primary())),
        Span::styled(format!("{:>6.1} MB/s", write_mbps), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
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
        Span::styled("─".repeat(w), sep),
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
        Span::styled(format_size(rx), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::raw("     "),
        Span::styled("TX ▲ ", Style::default().fg(theme::accent_primary())),
        Span::styled(format_size(tx), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
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
        Span::styled(format_size(app.system_state.net_in), Style::default().fg(Color::White)),
        Span::raw("   ↑ "),
        Span::styled(format_size(app.system_state.net_out), Style::default().fg(Color::White)),
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
            Span::styled(format!("{:10}", iface.name), Style::default().fg(Color::White)),
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
                Span::styled(format!("{:10}", ""), Style::default().fg(Color::White)),
                Span::styled(r_spark.to_string(), Style::default().fg(theme::accent_secondary())),
                Span::raw("  "),
                Span::styled(t_spark.to_string(), Style::default().fg(theme::accent_primary())),
            ]));
        }
    }

    lines.push(Line::from(vec![
        Span::styled("╰", sep),
        Span::styled("─".repeat(w), sep),
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
        Span::styled(info, Style::default().fg(Color::White)),
    ]));

    lines.push(Line::from(vec![
        Span::styled("╰", sep),
        Span::styled("─".repeat(w), sep),
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
    let current_user = std::env::var("USER").unwrap_or_else(|_| "dracon".to_string());
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
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD);
            is_selected = true;
        }
        let cpu_color = if is_selected {
            Color::Black
        } else if p.cpu > 50.0 {
            Color::Red
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
                Color::Black
            } else {
                Color::Rgb(60, 65, 75)
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
    let num_cols = 5;
    let spacing = 2;
    let total_spacing = (num_cols - 1) * spacing;
    let effective_width = area.width.saturating_sub(total_spacing);

    let header_rects = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(column_constraints)
        .split(Rect::new(area.x, area.y, effective_width, 1));

    app.monitor.process_column_bounds.clear();
    let mut current_col_x = area.x;
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
        let width = header_rects[i].width;
        app.monitor.process_column_bounds
            .push((Rect::new(current_col_x, area.y, width, 1), *col));
        current_col_x += width + spacing;
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
                    Color::Rgb(60, 65, 75)
                })
                .add_modifier(Modifier::BOLD),
        )
    });

    f.render_widget(
        Table::new(rows, column_constraints)
            .header(Row::new(header_cells).height(1).bottom_margin(1))
            .column_spacing(2),
        area,
    );
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
    let num_cols = 6;
    let spacing = 2;
    let total_spacing = (num_cols - 1) * spacing;
    let effective_width = area.width.saturating_sub(total_spacing);

    app.monitor.process_column_bounds.clear();
    let header_rects = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(column_constraints)
        .split(Rect::new(area.x, area.y, effective_width, 1));
    let mut current_col_x = area.x;
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
            let width = header_rects[i].width;
            app.monitor.process_column_bounds
                .push((Rect::new(current_col_x, area.y, width, 1), col));
            current_col_x += width + spacing;
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
                        Color::Rgb(60, 65, 75)
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
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD);
            is_selected = true;
        }
        let cpu_color = if is_selected {
            Color::Black
        } else if p.cpu > 50.0 {
            Color::Red
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
                Color::Black
            } else {
                Color::Rgb(60, 65, 75)
            })),
            Cell::from(name_display).style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from(p.user.clone()).style(Style::default().fg(if is_selected {
                Color::Black
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
            .column_spacing(1),
        area,
        &mut app.monitor.process_table_state,
    );
}
