use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Cell, Clear, List, ListItem, Paragraph, Row, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Sparkline, Table, TableState, Tabs,
    },
    Frame,
};
use std::time::SystemTime;

use crate::app::{
    App, AppMode, CurrentView, DropTarget, FileColumn, MonitorSubview, ProcessColumn,
    SettingsSection, SettingsTarget,
};
use crate::icons::Icon;
use crate::state::ProcessInfo;
use crate::ui::theme::THEME;
use dracon_terminal_engine::layout::centered_rect;
use dracon_terminal_engine::utils::{
    format_permissions, format_size, format_time, get_visual_width, squarify, truncate_to_width,
};
use dracon_terminal_engine::widgets::HotkeyHint;
use unicode_width::UnicodeWidthStr;

pub mod panes;
pub mod theme;

pub use panes::breadcrumbs::draw_pane_breadcrumbs;

pub fn draw(f: &mut Frame, app: &mut App) {
    f.render_widget(Clear, f.area());

    if app.current_view == CurrentView::Commit {
        draw_commit_view(f, f.area(), app);
    } else if matches!(
        app.mode,
        AppMode::Editor
            | AppMode::Viewer
            | AppMode::EditorSearch
            | AppMode::EditorGoToLine
            | AppMode::EditorReplace
    ) && app.show_main_stage
        && !app.is_split_mode
    {
        // --- FULL SCREEN EDITOR VIEW (Zen Mode / Overlay) ---
        let mut header_left = Vec::new();
        let border_color = if let Some(preview) = &app.editor_state {
            if let Some(last_saved) = preview.last_saved {
                if last_saved.elapsed().as_secs() < 2 {
                    crate::ui::theme::accent_secondary()
                } else if let Some(editor) = &preview.editor {
                    if editor.modified {
                        crate::ui::theme::accent_primary()
                    } else {
                        crate::ui::theme::border_active()
                    }
                } else {
                    crate::ui::theme::border_active()
                }
            } else if let Some(editor) = &preview.editor {
                if editor.modified {
                    crate::ui::theme::accent_primary()
                } else {
                    crate::ui::theme::border_active()
                }
            } else {
                crate::ui::theme::border_active()
            }
        } else {
            crate::ui::theme::border_active()
        };

        match app.mode {
            AppMode::EditorSearch => {
                header_left.push(Span::styled(
                    "SEARCH: ",
                    Style::default()
                        .fg(crate::ui::theme::accent_primary())
                        .add_modifier(Modifier::BOLD),
                ));
                header_left.push(Span::styled(
                    &app.input.value,
                    Style::default().fg(THEME.fg),
                ));
            }
            AppMode::EditorGoToLine => {
                header_left.push(Span::styled(
                    "GO TO LINE: ",
                    Style::default()
                        .fg(crate::ui::theme::accent_primary())
                        .add_modifier(Modifier::BOLD),
                ));
                header_left.push(Span::styled(
                    &app.input.value,
                    Style::default().fg(THEME.fg),
                ));
            }
            AppMode::EditorReplace => {
                if app.replace_buffer.is_empty() {
                    header_left.push(Span::styled(
                        "REPLACE [FIND]: ",
                        Style::default()
                            .fg(crate::ui::theme::accent_secondary())
                            .add_modifier(Modifier::BOLD),
                    ));
                    header_left.push(Span::styled(
                        &app.input.value,
                        Style::default().fg(THEME.fg),
                    ));
                } else {
                    header_left.push(Span::styled(
                        "REPLACE [WITH]: ",
                        Style::default()
                            .fg(crate::ui::theme::accent_secondary())
                            .add_modifier(Modifier::BOLD),
                    ));
                    header_left.push(Span::styled(
                        &app.input.value,
                        Style::default().fg(THEME.fg),
                    ));
                }
            }
            _ => {
                header_left.extend(HotkeyHint::render(
                    "^F",
                    "Find",
                    crate::ui::theme::accent_secondary(),
                ));
                header_left.extend(HotkeyHint::render(
                    "^R/F2",
                    "Replace",
                    crate::ui::theme::accent_secondary(),
                ));
                header_left.extend(HotkeyHint::render(
                    "^G",
                    "Line",
                    crate::ui::theme::accent_secondary(),
                ));
            }
        }

        let mut header_right = Vec::new();
        header_right.extend(HotkeyHint::render("Esc", "Back", Color::Red));
        header_right.extend(HotkeyHint::render("^Q", "Quit", Color::Red));

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title_top(Line::from(header_left))
            .title_top(Line::from(header_right).alignment(ratatui::layout::Alignment::Right))
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(Color::Rgb(0, 0, 0)));

        f.render_widget(block.clone(), f.area());

        let inner_area = block.inner(f.area());
        // Fix for line number border overlap: add 1 column of padding on left
        let inner_area = ratatui::layout::Rect {
            x: inner_area.x + 1,
            width: inner_area.width.saturating_sub(1),
            ..inner_area
        };

        let footer_height = 1u16;
        let editor_area = Rect::new(
            inner_area.x,
            inner_area.y,
            inner_area.width,
            inner_area.height.saturating_sub(footer_height),
        );
        let footer_area = Rect::new(
            inner_area.x,
            inner_area.y + inner_area.height.saturating_sub(footer_height),
            inner_area.width,
            footer_height,
        );

        if let Some(preview) = &app.editor_state {
            if let Some((rgba, w, h)) = preview.image_data.as_ref() {
                let max_w = inner_area.width as usize;
                let max_h = (inner_area.height.saturating_sub(footer_height + 3)) as usize;
                let w_val = *w as usize;
                let h_val = *h as usize;
                let scale_x = if w_val > 0 { max_w as f32 / w_val as f32 } else { 1.0 };
                let scale_y = if h_val > 0 { max_h as f32 / h_val as f32 } else { 1.0 };
                let scale = scale_x.min(scale_y).max(0.1);
                let new_w = ((w_val as f32 * scale) as u16).max(1);
                let new_h = ((h_val as f32 * scale) as u16).max(1);
                let img_area = Rect::new(
                    inner_area.x.saturating_add((inner_area.width.saturating_sub(new_w)) / 2),
                    inner_area.y.saturating_add((inner_area.height.saturating_sub(new_h + footer_height)) / 2),
                    new_w,
                    new_h,
                );
                // Draw image as ASCII block characters
                let chars = ["░", "▒", "▓", "█"];
                let step_x = (w_val / new_w as usize).max(1);
                let step_y = (h_val / new_h as usize).max(1);
                let mut img_text = String::new();
                for y in (0..h_val).step_by(step_y).take(new_h as usize) {
                    for x in (0..w_val).step_by(step_x) {
                        let idx = (y * w_val + x) * 4;
                        if idx + 2 < rgba.len() {
                            let r = rgba[idx] as usize;
                            let g = rgba[idx + 1] as usize;
                            let b = rgba[idx + 2] as usize;
                            let bright = (r + g + b) / 3;
                            let c = if bright > 200 { chars[3] } else if bright > 150 { chars[2] } else if bright > 100 { chars[1] } else { chars[0] };
                            img_text.push_str(c);
                        } else {
                            img_text.push(' ');
                        }
                    }
                    img_text.push('\n');
                }
                let block = Block::default()
                    .title(format!(" Image {}x{} ", w, h))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(crate::ui::theme::border_active()));
                f.render_widget(&block, img_area);
                let inner_img = block.inner(img_area);
                f.render_widget(Paragraph::new(img_text), inner_img);
            } else if let Some(editor) = &preview.editor {
                let mut editor_clone = editor.clone();
                editor_clone.wrap = app.is_split_mode;
                f.render_widget(&editor_clone, editor_area);

                // Footer bar: Ln X, Col Y | language | ● modified | ^S Save ^↵ Run
                let cursor_row = editor.cursor_row + 1;
                let cursor_col = editor.cursor_col + 1;
                let modified_indicator = if editor.modified { " ●" } else { "" };
                let modified_color = if editor.modified {
                    crate::ui::theme::accent_primary()
                } else {
                    Color::DarkGray
                };

                let footer_line = Line::from(vec![
                    Span::raw(" "),
                    Span::styled(format!("Ln {}, Col {}", cursor_row, cursor_col), Style::default().fg(Color::DarkGray)),
                    Span::raw(" | "),
                    Span::styled(format!(" {} ", editor.language), Style::default().fg(crate::ui::theme::accent_secondary())),
                    Span::raw(" | "),
                    Span::styled(modified_indicator, Style::default().fg(modified_color)),
                    Span::raw("  "),
                    Span::styled("^S ", Style::default().fg(Color::DarkGray)),
                    Span::styled("Save", Style::default().fg(crate::ui::theme::accent_secondary())),
                    Span::raw("  "),
                    Span::styled("^R ", Style::default().fg(Color::DarkGray)),
                    Span::styled("Run", Style::default().fg(crate::ui::theme::accent_secondary())),
                ]);
                f.render_widget(Paragraph::new(footer_line).alignment(Alignment::Left), footer_area);
            }
        }

        if matches!(app.mode, AppMode::EditorSearch | AppMode::EditorGoToLine | AppMode::EditorReplace) {
            let search_footer_height = 2;
            let search_footer_area = Rect::new(
                f.area().x,
                f.area().height.saturating_sub(search_footer_height),
                f.area().width,
                search_footer_height,
            );
            draw_footer(f, search_footer_area, app);
        }
    } else if matches!(
        app.mode,
        AppMode::Settings | AppMode::StyleColorInput | AppMode::ResetSettingsConfirm
    ) {
        f.render_widget(
            Block::default().style(Style::default().bg(Color::Black)),
            f.area(),
        );
        draw_settings_modal(f, app);
    } else if matches!(
        app.current_view,
        CurrentView::Processes | CurrentView::Git | CurrentView::Debug | CurrentView::Trash | CurrentView::DiskUsage
    ) {
        f.render_widget(
            Block::default().style(Style::default().bg(Color::Black)),
            f.area(),
        );
        match app.current_view {
            CurrentView::Processes => draw_monitor_page(f, f.area(), app),
            CurrentView::Git => draw_git_page(f, f.area(), app),
            CurrentView::Debug => draw_debug_page(f, f.area(), app),
            CurrentView::Trash => draw_trash_page(f, f.area(), app),
            CurrentView::DiskUsage => draw_disk_usage_page(f, f.area(), app),
            _ => {}
        }
    } else {
        // Normal File Manager Background
        f.render_widget(
            Block::default().style(Style::default().bg(Color::Rgb(0, 0, 0))),
            f.area(),
        );

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Fill(1),
                Constraint::Length(2),
            ])
            .split(f.area());

        let workspace_constraints = if app.show_main_stage {
            if app.show_sidebar {
                [Constraint::Length(app.sidebar_width()), Constraint::Fill(1)]
            } else {
                [Constraint::Length(0), Constraint::Fill(1)]
            }
        } else {
            [Constraint::Fill(1), Constraint::Length(0)]
        };

        let workspace = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(workspace_constraints)
            .split(chunks[1]);

        draw_global_header(f, chunks[0], workspace[0].width, app);

        if app.show_sidebar || !app.show_main_stage {
            crate::ui::panes::sidebar::draw_sidebar(f, workspace[0], app);
        }

        if app.show_main_stage {
            draw_main_stage(f, workspace[1], app);
        }

        draw_footer(f, chunks[2], app);
    }

    // --- OVERLAYS ---
    if let AppMode::Hotkeys = app.mode {
        draw_hotkeys_modal(f, f.area());
    }
    if matches!(app.mode, AppMode::ContextMenu { .. }) {
        if let AppMode::ContextMenu {
            x, y, ref target, ..
        } = app.mode
        {
            draw_context_menu(f, x, y, target, app);
        }
    }
    if matches!(app.mode, AppMode::Highlight) {
        draw_highlight_modal(f, app);
    }
    if matches!(app.mode, AppMode::Rename) {
        draw_rename_modal(f, app);
    }
    if matches!(app.mode, AppMode::BulkRename { .. }) {
        draw_bulk_rename_modal(f, app);
    }
    if matches!(app.mode, AppMode::Delete(_) | AppMode::DeleteFile(_)) {
        draw_delete_modal(f, app);
    }
    if matches!(app.mode, AppMode::KillProcessConfirm(_, _)) {
        draw_kill_process_modal(f, app);
    }
    if matches!(app.mode, AppMode::Properties) {
        draw_properties_modal(f, app);
    }
    if matches!(app.mode, AppMode::EditPermissions(_)) {
        draw_edit_permissions_modal(f, app);
    }
    if matches!(app.mode, AppMode::NewFolder) {
        draw_new_folder_modal(f, app);
    }
    if matches!(app.mode, AppMode::NewFile) {
        draw_new_file_modal(f, app);
    }
    if matches!(app.mode, AppMode::SaveAs(_)) {
        draw_save_as_modal(f, app);
    }
    if matches!(app.mode, AppMode::CommandPalette) {
        draw_command_palette(f, app);
    }
    if matches!(app.mode, AppMode::StyleColorInput) {
        draw_style_color_modal(f, app);
    }
    if matches!(app.mode, AppMode::ResetSettingsConfirm) {
        draw_reset_settings_modal(f, app);
    }
    if matches!(app.mode, AppMode::AddRemote(_)) {
        draw_add_remote_modal(f, app);
    }
    if matches!(app.mode, AppMode::ImportServers) {
        draw_import_servers_modal(f, app);
    }
    if matches!(app.mode, AppMode::ImportSshConfig) {
        draw_import_ssh_config_modal(f, app);
    }
    if let AppMode::OpenWith(ref path) = app.mode {
        draw_open_with_modal(f, app, path);
    }
    if let AppMode::DragDropMenu {
        ref sources,
        ref target,
        target_is_remote,
    } = app.mode
    {
        draw_drag_drop_modal(f, app, sources, target, target_is_remote);
    }

    if let AppMode::CreateArchive(ref paths, format_idx) = app.mode {
        draw_create_archive_modal(f, app, paths, format_idx);
    }
    if let AppMode::CommandOutput(ref _cmd) = app.mode {
        draw_command_output_modal(f, app);
    }

    if app.is_dragging {
        draw_drag_ghost(f, app);
    }
}

fn draw_commit_view(f: &mut Frame, area: Rect, app: &mut App) {
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

    let content_source = app.editor_state.as_ref()
        .or_else(|| {
            let pane_idx = app.focused_pane_index;
            app.panes.get(pane_idx).and_then(|p| p.current_state().and_then(|fs| fs.preview.as_ref()))
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
        .border_style(Style::default().fg(crate::ui::theme::border_inactive()))
        .title_top(Line::from(vec![Span::styled(
            " COMMIT ",
            Style::default()
                .fg(Color::Black)
                .bg(crate::ui::theme::accent_primary())
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
                Span::styled(" Back to Git ", Style::default().fg(Color::Red)),
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
                    .bg(crate::ui::theme::accent_primary())
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
            Style::default().fg(Color::DarkGray),
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
                    .bg(crate::ui::theme::accent_secondary())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                format!(" +{} ", additions),
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                format!(" -{} ", deletions),
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Red)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                format!(" @@ {} ", hunks),
                Style::default()
                    .fg(Color::Black)
                    .bg(crate::ui::theme::header_fg())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {}", files_preview),
                Style::default().fg(Color::DarkGray),
            ),
        ])),
        layout[3],
    );

    let content_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(crate::ui::theme::border_inactive()))
        .title_top(Line::from(vec![Span::styled(
            " PATCH ",
            Style::default()
                .fg(Color::Black)
                .bg(crate::ui::theme::accent_primary())
                .add_modifier(Modifier::BOLD),
        )]));
    let content_inner = content_block.inner(layout[4]);
    f.render_widget(content_block, layout[4]);

    if let Some(preview) = &app.editor_state {
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
            if let Some(preview) = &fs.preview {
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
            .style(Style::default().fg(Color::DarkGray)),
        content_inner,
    );
}

fn draw_drag_drop_modal(
    f: &mut Frame,
    app: &App,
    sources: &[std::path::PathBuf],
    target: &std::path::Path,
    target_is_remote: bool,
) {
    let area = centered_rect(60, 20, f.area());
    let block = Block::default()
        .title(" Choice Action ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Yellow));
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

    let (mx, my) = app.mouse_pos;

    let is_hover = |bx: u16, len: u16| {
        mx >= inner.x + bx && mx < inner.x + bx + len && my == inner.y + button_y_offset as u16
    };

    let mut text = Vec::new();

    if sources.len() == 1 {
        let src_name = sources[0].file_name().unwrap_or_default().to_string_lossy();
        text.push(Line::from(vec![
            Span::raw("Item: "),
            Span::styled(
                src_name,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
    } else {
        text.push(Line::from(vec![
            Span::raw("Items: "),
            Span::styled(
                format!("{} files/folders", sources.len()),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        // List first few items
        for source in sources.iter().take(std::cmp::min(sources.len(), 3)) {
            let name = source.file_name().unwrap_or_default().to_string_lossy();
            text.push(Line::from(vec![
                Span::raw("  - "),
                Span::styled(name, Style::default().fg(Color::DarkGray)),
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
            Style::default().fg(Color::Cyan),
        ),
    ]));

    // Spacing
    text.push(Line::from(""));

    if target_is_remote {
        let upload_style = if is_hover(0, 12) {
            Style::default().bg(Color::Green).fg(Color::Black)
        } else {
            Style::default().fg(Color::Green)
        };
        let cancel_style = if is_hover(14, 14) {
            Style::default().bg(Color::Red).fg(Color::Black)
        } else {
            Style::default().fg(Color::Red)
        };
        text.push(Line::from(vec![
            Span::styled(" [U] Upload ", upload_style.add_modifier(Modifier::BOLD)),
            Span::raw("  "),
            Span::styled(" [Esc] Cancel ", cancel_style.add_modifier(Modifier::BOLD)),
        ]));
    } else {
        let copy_style = if is_hover(0, 10) {
            Style::default().bg(Color::Green).fg(Color::Black)
        } else {
            Style::default().fg(Color::Green)
        };
        let move_style = if is_hover(12, 10) {
            Style::default().bg(Color::Yellow).fg(Color::Black)
        } else {
            Style::default().fg(Color::Yellow)
        };
        let link_style = if is_hover(24, 10) {
            Style::default().bg(Color::Magenta).fg(Color::Black)
        } else {
            Style::default().fg(Color::Magenta)
        };
        let cancel_style = if is_hover(36, 14) {
            Style::default().bg(Color::Red).fg(Color::Black)
        } else {
            Style::default().fg(Color::Red)
        };
        text.push(Line::from(vec![
            Span::styled(" [C] Copy ", copy_style.add_modifier(Modifier::BOLD)),
            Span::raw("  "),
            Span::styled(" [M] Move ", move_style.add_modifier(Modifier::BOLD)),
            Span::raw("  "),
            Span::styled(" [L] Link ", link_style.add_modifier(Modifier::BOLD)),
            Span::raw("  "),
            Span::styled(" [Esc] Cancel ", cancel_style.add_modifier(Modifier::BOLD)),
        ]));
    }

    f.render_widget(Paragraph::new(text), inner);
}

fn draw_create_archive_modal(
    f: &mut Frame,
    app: &App,
    paths: &[std::path::PathBuf],
    format_idx: usize,
) {
    let area = centered_rect(50, 18, f.area());
    let block = Block::default()
        .title(" Create Archive ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(crate::ui::theme::accent_primary()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let formats = ["tar.gz", "zip"];
    
    let mut text = Vec::new();
    
    // File count
    text.push(Line::from(vec![
        Span::raw("Files: "),
        Span::styled(
            format!("{}", paths.len()),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
    ]));
    
    // Format selection
    text.push(Line::from(""));
    text.push(Line::from(vec![
        Span::styled("Format:", Style::default().fg(Color::Yellow)),
    ]));
    
    for (i, format) in formats.iter().enumerate() {
        let is_selected = i == format_idx;
        let style = if is_selected {
            Style::default().bg(crate::ui::theme::accent_primary()).fg(Color::Black)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let prefix = if is_selected { "▶ " } else { "  " };
        text.push(Line::from(vec![
            Span::styled(format!("{}{}", prefix, format), style.add_modifier(Modifier::BOLD)),
        ]));
    }
    
    // Filename
    text.push(Line::from(""));
    text.push(Line::from(vec![
        Span::raw("Filename: "),
        Span::styled(
            &app.input.value,
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
    ]));
    
    // Help
    text.push(Line::from(""));
    text.push(Line::from(vec![
        Span::styled("↑↓", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw(" Select Format  "),
        Span::styled("Enter", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw(" Create  "),
        Span::styled("Esc", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw(" Cancel"),
    ]));

    f.render_widget(Paragraph::new(text), inner);
}

fn draw_command_output_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(80, 70, f.area());
    let block = Block::default()
        .title(" Command Output ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(crate::ui::theme::accent_primary()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .split(inner);

    // Output area
    if app.command_output.is_empty() && app.command_output_status.is_none() {
        // Input phase: show prompt
        let input_text = vec![
            Line::from(""),
            Line::from(""),
            Line::from(vec![
                Span::styled("$ ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::styled(&app.input.value, Style::default().fg(Color::White)),
                Span::styled("▎", Style::default().fg(Color::White)),
            ]),
        ];
        f.render_widget(Paragraph::new(input_text), chunks[0]);
    } else if app.command_output.is_empty() && app.command_output_status.as_deref() == Some("Running...") {
        f.render_widget(
            Paragraph::new("Running...")
                .style(Style::default().fg(Color::Yellow))
                .alignment(ratatui::layout::Alignment::Center),
            chunks[0],
        );
    } else {
        let visible_height = chunks[0].height as usize;
        let total_lines = app.command_output.len();
        let start = app.command_output_scroll.min(total_lines.saturating_sub(1));
        let visible: Vec<Line> = app.command_output
            .iter()
            .skip(start)
            .take(visible_height)
            .map(|line| {
                if line.starts_with("ERR: ") {
                    Line::from(Span::styled(&line[5..], Style::default().fg(Color::Red)))
                } else {
                    Line::from(Span::raw(line))
                }
            })
            .collect();
        f.render_widget(Paragraph::new(visible), chunks[0]);
    }

    // Status bar
    let status_text = if let Some(ref status) = app.command_output_status {
        if status == "Running..." {
            format!(" {} | Lines: {} | Esc/q to close", status, app.command_output.len())
        } else {
            format!(" {} | Lines: {} | Esc/q to close", status, app.command_output.len())
        }
    } else {
        format!(" Type command, Enter to run | Esc to cancel")
    };
    f.render_widget(
        Paragraph::new(status_text)
            .style(Style::default().fg(Color::DarkGray)),
        chunks[1],
    );
}

fn draw_hotkeys_modal(f: &mut Frame, _area: Rect) {
    let area = centered_rect(70, 80, f.area());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" KEYBINDINGS ")
        .border_style(Style::default().fg(crate::ui::theme::accent_primary()));
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
            .style(Style::default().fg(Color::DarkGray))
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
                ("Ctrl + N", "Open Terminal Tab"),
                ("Ctrl + T", "New File Tab"),
                ("Ctrl + K", "Open Terminal Window"),
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
                ("Ctrl + R / F2", "Replace All"),
                ("Ctrl + G", "Go To Line"),
                ("Ctrl + C", "Copy Line"),
                ("Ctrl + X", "Cut Line / Delete Line"),
                ("Ctrl + Bksp", "Delete Word"),
                ("Esc", "Exit Editor"),
            ],
        ),
    ];

    let mut rows = Vec::new();
    for (section, items) in keys {
        rows.push(Row::new(vec![
            Cell::from(Span::styled(
                section,
                Style::default()
                    .fg(crate::ui::theme::accent_primary())
                    .add_modifier(Modifier::BOLD),
            )),
            Cell::from(""),
        ]));
        for (key, desc) in items {
            rows.push(Row::new(vec![
                Cell::from(Span::styled(
                    format!("  {}", key),
                    Style::default().fg(Color::Yellow),
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

fn draw_open_with_modal(f: &mut Frame, app: &App, path: &std::path::Path) {
    let area = centered_rect(60, 60, f.area()); // Increased height
    let block = Block::default()
        .title(" Open With... ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Yellow));
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
        .border_style(Style::default().fg(crate::ui::theme::accent_primary()));
    f.render_widget(
        Paragraph::new(app.input.value.as_str()).block(input_block),
        chunks[1],
    );

    // Simple common suggestions based on extension
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let mut suggestions = crate::event_helpers::get_open_with_suggestions(app, &ext);

    // Filter suggestions based on input
    if !app.input.value.is_empty() {
        let query = app.input.value.to_lowercase();
        suggestions.retain(|s| s.to_lowercase().contains(&query));
    }

    let (mx, my) = app.mouse_pos;
    let list_items: Vec<ListItem> = suggestions
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let item_y = chunks[2].y + i as u16;
            let is_mouse_hovered =
                mx >= chunks[2].x && mx < chunks[2].x + chunks[2].width && my == item_y;
            let is_selected = i == app.open_with_index;

            let style = if is_mouse_hovered || is_selected {
                Style::default()
                    .bg(crate::ui::theme::accent_primary())
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            ListItem::new(format!("  󰀻  {}", s)).style(style)
        })
        .collect();

    let title = if app.input.value.is_empty() {
        " Suggestions (Click to Launch) "
    } else {
        " Filtered Suggestions (Click to Launch) "
    };

    let list = List::new(list_items).block(
        Block::default()
            .title(title)
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    f.render_widget(list, chunks[2]);
}

fn draw_monitor_page(f: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::default()
        .title_top(Line::from(vec![Span::styled(
            " SYSTEM MONITOR ",
            Style::default()
                .fg(Color::Black)
                .bg(crate::ui::theme::accent_primary())
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
        .border_style(Style::default().fg(crate::ui::theme::accent_primary()));

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

    app.monitor_subview_bounds.clear();
    let mut cur_x = nav_layout[0].x;
    for (view, name) in subviews {
        let is_active = app.monitor_subview == view;
        let width = name.chars().count() as u16 + 4;
        let rect = Rect::new(cur_x, nav_layout[0].y, width, 1);

        let mut style = if is_active {
            Style::default()
                .bg(crate::ui::theme::accent_primary())
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Rgb(60, 65, 75))
        };
        if app.mouse_pos.1 == nav_layout[0].y
            && app.mouse_pos.0 >= rect.x
            && app.mouse_pos.0 < rect.x + rect.width
        {
            style = style.fg(Color::White);
        }

        f.render_widget(Paragraph::new(name).style(style), rect);
        if is_active {
            f.render_widget(
                Paragraph::new("━━━━")
                    .style(Style::default().fg(crate::ui::theme::accent_primary())),
                Rect::new(rect.x, rect.y + 1, 4, 1),
            );
        }

        app.monitor_subview_bounds.push((rect, view));
        cur_x += width + 2;
    }

    if app.monitor_subview != MonitorSubview::Overview {
        let is_searching = matches!(app.mode, AppMode::ProcessSearch);
        let search_style = if is_searching {
            Style::default()
                .fg(crate::ui::theme::accent_primary())
                .add_modifier(Modifier::BOLD)
        } else if app.process_search_filter.is_empty() {
            Style::default().fg(Color::Rgb(40, 45, 55))
        } else {
            Style::default().fg(crate::ui::theme::accent_primary())
        };
        let cursor = if is_searching { "▌" } else { "" };
        f.render_widget(
            Paragraph::new(format!(" 󰍉 {}{}{}", 
                if is_searching { "> " } else { "" },
                app.process_search_filter, 
                cursor
            )).style(search_style),
            nav_layout[1],
        );
    }

    let content_area = chunks[1].inner(ratatui::layout::Margin {
        horizontal: 1,
        vertical: 1,
    });
    match app.monitor_subview {
        MonitorSubview::Overview => draw_monitor_overview(f, content_area, app),
        MonitorSubview::Processes => draw_processes_view(f, content_area, app),
        MonitorSubview::Applications => draw_monitor_applications(f, content_area, app),
        MonitorSubview::Cpu
        | MonitorSubview::Memory
        | MonitorSubview::Disk
        | MonitorSubview::Network => draw_monitor_overview(f, content_area, app),
    }
}

fn draw_monitor_overview(f: &mut Frame, area: Rect, app: &mut App) {
    let inner = area.inner(ratatui::layout::Margin { horizontal: 1, vertical: 0 });

    // Layout: Left (sparklines + cores) | Right (system + network + disks)
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(inner);

    // ── LEFT: Resource Usage ──
    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Length(8), Constraint::Min(0)])
        .split(main_chunks[0]);

    // CPU Section: Big bar + sparkline side by side
    let cpu_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(16), Constraint::Fill(1)])
        .split(left[0]);
    
    let cpu_pct = app.system_state.cpu_usage;
    let cpu_color = if cpu_pct > 80.0 { Color::Rgb(220, 100, 100) }
        else if cpu_pct > 50.0 { Color::Rgb(220, 180, 100) }
        else { Color::Rgb(100, 200, 140) };
    
    // CPU Big Number + Bar
    let cpu_bar_len = (cpu_pct / 100.0 * (cpu_chunks[0].width.saturating_sub(4) as f32)) as usize;
    let cpu_bar = "█".repeat(cpu_bar_len) + &"░".repeat(cpu_chunks[0].width.saturating_sub(4) as usize - cpu_bar_len);
    let cpu_big = vec![
        Line::from(vec![Span::styled(format!("{:.1}%", cpu_pct), Style::default().fg(cpu_color).add_modifier(Modifier::BOLD))]),
        Line::from(vec![Span::styled(cpu_bar, Style::default().fg(cpu_color))]),
        Line::from(vec![Span::styled("CPU", Style::default().fg(Color::Rgb(80, 85, 95)))])
    ];
    f.render_widget(
        Paragraph::new(cpu_big).alignment(Alignment::Center),
        cpu_chunks[0],
    );
    
    // CPU Sparkline
    let cpu_data: Vec<u64> = app.system_state.cpu_history.iter().copied().collect();
    let cpu_spark_block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(Color::Rgb(40, 45, 55)));
    f.render_widget(
        Sparkline::default()
            .data(&cpu_data)
            .style(Style::default().fg(cpu_color))
            .max(100)
            .block(cpu_spark_block),
        cpu_chunks[1],
    );

    // Memory Section: Big bar + sparkline side by side
    let mem_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(16), Constraint::Fill(1)])
        .split(left[1]);
    
    let mem_pct = (app.system_state.mem_usage / app.system_state.total_mem.max(1.0)) * 100.0;
    let mem_color = if mem_pct > 85.0 { Color::Rgb(220, 100, 100) }
        else if mem_pct > 60.0 { Color::Rgb(220, 180, 100) }
        else { Color::Rgb(100, 180, 220) };
    
    let mem_bar_len = (mem_pct / 100.0 * (mem_chunks[0].width.saturating_sub(4) as f32)) as usize;
    let mem_bar = "█".repeat(mem_bar_len) + &"░".repeat(mem_chunks[0].width.saturating_sub(4) as usize - mem_bar_len);
    let mem_big = vec![
        Line::from(vec![Span::styled(format!("{:.0}%", mem_pct), Style::default().fg(mem_color).add_modifier(Modifier::BOLD))]),
        Line::from(vec![Span::styled(mem_bar, Style::default().fg(mem_color))]),
        Line::from(vec![Span::styled(format!("{:.1}G", app.system_state.mem_usage / 1024.0), Style::default().fg(Color::Rgb(80, 85, 95)))])
    ];
    f.render_widget(
        Paragraph::new(mem_big).alignment(Alignment::Center),
        mem_chunks[0],
    );
    
    // Memory Sparkline
    let mem_data: Vec<u64> = app.system_state.mem_history.iter().copied().collect();
    let mem_spark_block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(Color::Rgb(40, 45, 55)));
    f.render_widget(
        Sparkline::default()
            .data(&mem_data)
            .style(Style::default().fg(mem_color))
            .max(app.system_state.total_mem as u64)
            .block(mem_spark_block),
        mem_chunks[1],
    );

    // CPU Cores - Smooth averaged values with clean visual
    if !app.system_state.cpu_cores.is_empty() {
        let cores = &app.system_state.cpu_cores;
        let history = &app.system_state.core_history;
        
        // Compute 5-sample moving average for smoother display
        let smoothed: Vec<f32> = cores.iter().enumerate().map(|(i, _)| {
            if let Some(hist) = history.get(i) {
                let samples: Vec<u64> = hist.iter().rev().take(5).copied().collect();
                if !samples.is_empty() {
                    samples.iter().sum::<u64>() as f32 / samples.len() as f32
                } else {
                    0.0
                }
            } else {
                0.0
            }
        }).collect();
        
        // Use 4 columns for compact grid
        let cols = 4usize;
        let rows = (cores.len() + cols - 1) / cols;
        
        let core_area = left[2];
        let label_height = 1u16;
        let grid_height = core_area.height.saturating_sub(label_height);
        let row_height = (grid_height / rows as u16).max(1);
        let cell_width = core_area.width / cols as u16;
        
        // Label
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("CPU Cores (5s avg)", Style::default().fg(Color::Rgb(80, 85, 95)).add_modifier(Modifier::BOLD))
            ])),
            Rect::new(core_area.x, core_area.y, core_area.width, label_height),
        );
        
        let grid_area = Rect::new(core_area.x, core_area.y + label_height, core_area.width, grid_height);
        
        for (i, usage) in smoothed.iter().enumerate() {
            let col = i % cols;
            let row = i / cols;
            let x = grid_area.x + col as u16 * cell_width;
            let y = grid_area.y + row as u16 * row_height;
            let area = Rect::new(x, y, cell_width, row_height);
            
            let intensity = (*usage / 100.0).clamp(0.0, 1.0);
            let color = if intensity > 0.8 {
                Color::Rgb(255, 100, 100)
            } else if intensity > 0.5 {
                Color::Rgb(255, 200, 80)
            } else if intensity > 0.2 {
                Color::Rgb(100, 220, 140)
            } else {
                Color::Rgb(100, 105, 115)
            };
            
            // Compact bar: 4 chars wide
            let filled = ((4.0 * intensity) as usize).max(if intensity > 0.02 { 1 } else { 0 });
            let bar = "█".repeat(filled) + &"░".repeat(4 - filled);
            
            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled(format!("{:02} ", i), Style::default().fg(Color::Rgb(80, 85, 95))),
                    Span::styled(bar, Style::default().fg(color)),
                    Span::styled(format!(" {:>3.0}%", usage), Style::default().fg(color)),
                ])),
                area,
            );
        }
    }

    // ── RIGHT: System Info ──
    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Length(7), Constraint::Min(0)])
        .split(main_chunks[1]);

    // System Info
    let sys_lines = vec![
        Line::from(vec![Span::styled(&app.system_state.hostname, Style::default().fg(crate::ui::theme::accent_primary()).add_modifier(Modifier::BOLD))]),
        Line::from(vec![
            Span::styled("OS ", Style::default().fg(Color::Rgb(80, 85, 95))),
            Span::styled(&app.system_state.os_name, Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("UP ", Style::default().fg(Color::Rgb(80, 85, 95))),
            Span::styled(format_uptime(app.system_state.uptime), Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("KR ", Style::default().fg(Color::Rgb(80, 85, 95))),
            Span::styled(&app.system_state.kernel_version, Style::default().fg(Color::Rgb(180, 185, 190))),
        ]),
        Line::from(vec![
            Span::styled("PR ", Style::default().fg(Color::Rgb(80, 85, 95))),
            Span::styled(format!("{} processes", app.system_state.processes.len()), Style::default().fg(crate::ui::theme::accent_secondary())),
        ]),
    ];
    f.render_widget(
        Paragraph::new(sys_lines).block(
            Block::default()
                .title(Span::styled(" System ", Style::default().fg(crate::ui::theme::header_fg())))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Rgb(50, 55, 65)))
        ),
        right[0],
    );

    // Network
    let rx_rate = app.system_state.net_in.saturating_sub(app.system_state.last_net_in);
    let tx_rate = app.system_state.net_out.saturating_sub(app.system_state.last_net_out);
    
    let net_lines = vec![
        Line::from(vec![
            Span::styled("▼ RX ", Style::default().fg(Color::Rgb(100, 200, 140))),
            Span::styled(format_bytes(rx_rate) + "/s", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("▲ TX ", Style::default().fg(Color::Rgb(220, 180, 100))),
            Span::styled(format_bytes(tx_rate) + "/s", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]),
    ];
    f.render_widget(
        Paragraph::new(net_lines).block(
            Block::default()
                .title(Span::styled(" Network ", Style::default().fg(crate::ui::theme::header_fg())))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Rgb(50, 55, 65)))
        ),
        right[1],
    );

    // Disks
    if !app.system_state.disks.is_empty() {
        let disk_lines: Vec<Line> = app.system_state.disks.iter().map(|disk| {
            let pct = if disk.total_space > 0.0 {
                (disk.used_space / disk.total_space * 100.0) as u8
            } else { 0 };
            let color = if pct > 85 { Color::Rgb(220, 100, 100) }
                else if pct > 60 { Color::Rgb(220, 180, 100) }
                else { Color::Rgb(100, 200, 140) };
            
            Line::from(vec![
                Span::styled(format!("{:<10}", &disk.name), Style::default().fg(Color::Rgb(140, 145, 155))),
                Span::styled(format!("{:>3}%", pct), Style::default().fg(color).add_modifier(Modifier::BOLD)),
                Span::styled(format!("  {:.1}G / {:.0}G", disk.used_space / 1e9, disk.total_space / 1e9), Style::default().fg(Color::Rgb(100, 105, 115))),
            ])
        }).collect();
        
        f.render_widget(
            Paragraph::new(disk_lines).block(
                Block::default()
                    .title(Span::styled(" Storage ", Style::default().fg(crate::ui::theme::header_fg())))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Rgb(50, 55, 65)))
            ),
            right[2],
        );
    }
}
fn format_uptime(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let mins = (seconds % 3600) / 60;
    if days > 0 {
        format!("{}d {:02}h {:02}m", days, hours, mins)
    } else if hours > 0 {
        format!("{}h {:02}m", hours, mins)
    } else {
        format!("{}m", mins)
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_000_000_000 {
        format!("{:.1}GB", bytes as f64 / 1e9)
    } else if bytes >= 1_000_000 {
        format!("{:.1}MB", bytes as f64 / 1e6)
    } else if bytes >= 1_000 {
        format!("{:.1}KB", bytes as f64 / 1e3)
    } else {
        format!("{}B", bytes)
    }
}

fn draw_monitor_applications(f: &mut Frame, area: Rect, app: &mut App) {
    let current_user = std::env::var("USER").unwrap_or_else(|_| "dracon".to_string());
    let app_procs: Vec<(usize, &ProcessInfo)> = app
        .system_state
        .processes
        .iter()
        .enumerate()
        .filter(|(_, p)| {
            let matches = if app.process_search_filter.is_empty() {
                true
            } else {
                p.name.to_lowercase().contains(&app.process_search_filter.to_lowercase())
                    || p.user.to_lowercase().contains(&app.process_search_filter.to_lowercase())
            };
            p.user == current_user
                && !p.name.starts_with('[')
                && !p.name.contains("kworker")
                && matches
        })
        .collect();

    // Layout: summary (1-2) + table + footer (1)
    let has_filter = !app.process_search_filter.is_empty();
    let summary_height = if has_filter { 2 } else { 1 };
    let footer_height = 1;
    let table_height = area.height.saturating_sub(summary_height + footer_height);

    if table_height == 0 {
        return;
    }

    let summary_area = Rect::new(area.x, area.y, area.width, summary_height);
    let table_area = Rect::new(area.x, area.y + summary_height, area.width, table_height);
    let footer_area = Rect::new(area.x, area.y + summary_height + table_height, area.width, footer_height);

    // ── SUMMARY BAR ──
    let total_cpu: f32 = app_procs.iter().map(|(_, p)| p.cpu).sum();
    let total_mem: f32 = app_procs.iter().map(|(_, p)| p.mem).sum();
    let total_count = app_procs.len();

    let summary_spans = vec![
        Span::styled(" 󰀲 ", Style::default().fg(crate::ui::theme::accent_secondary())),
        Span::styled(format!("{} applications", total_count), Style::default().fg(Color::Rgb(180, 185, 190))),
        Span::styled(" | ", Style::default().fg(Color::Rgb(60, 65, 75))),
        Span::styled("CPU: ", Style::default().fg(Color::Rgb(100, 150, 180))),
        Span::styled(format!("{:.1}%", total_cpu), Style::default().fg(if total_cpu > 50.0 { Color::Rgb(220, 100, 100) } else if total_cpu > 20.0 { Color::Rgb(220, 180, 100) } else { Color::Rgb(100, 200, 140) })),
        Span::styled(" | ", Style::default().fg(Color::Rgb(60, 65, 75))),
        Span::styled("MEM: ", Style::default().fg(Color::Rgb(100, 150, 180))),
        Span::styled(format!("{:.1}G", total_mem / 1024.0), Style::default().fg(Color::Rgb(100, 180, 220))),
    ];

    if has_filter {
        let filter_line = Line::from(vec![
            Span::styled(" 󰈲 Filter: '", Style::default().fg(crate::ui::theme::accent_secondary())),
            Span::styled(&app.process_search_filter, Style::default().fg(Color::White)),
            Span::styled("'", Style::default().fg(crate::ui::theme::accent_secondary())),
        ]);
        f.render_widget(Paragraph::new(vec![Line::from(summary_spans), filter_line]), summary_area);
    } else {
        f.render_widget(Paragraph::new(Line::from(summary_spans)), summary_area);
    }

    // ── TABLE ──
    let name_width = table_area.width.saturating_sub(3 + 10 + 14 + 16 + 12) as usize;

    let header_cells = vec![
        Cell::from("").style(Style::default().fg(Color::Rgb(100, 105, 115)).add_modifier(Modifier::BOLD)),
        Cell::from("Application").style(Style::default().fg(Color::Rgb(100, 105, 115)).add_modifier(Modifier::BOLD)),
        Cell::from("CPU").style(Style::default().fg(Color::Rgb(100, 105, 115)).add_modifier(Modifier::BOLD)),
        Cell::from("Memory").style(Style::default().fg(Color::Rgb(100, 105, 115)).add_modifier(Modifier::BOLD)),
        Cell::from("PID").style(Style::default().fg(Color::Rgb(100, 105, 115)).add_modifier(Modifier::BOLD)),
    ];

    let rows: Vec<Row> = app_procs.iter().enumerate().map(|(row_idx, (_, p))| {
        let is_selected = app.process_selected_idx == Some(row_idx);

        // Status icon
        let status_color = process_status_color(&p.status);
        let status_icon = "●";

        // CPU display
        let cpu_pct = p.cpu as f64;
        let cpu_bar = mini_bar(cpu_pct, 6);
        let cpu_color = if cpu_pct > 50.0 {
            Color::Rgb(220, 100, 100)
        } else if cpu_pct > 20.0 {
            Color::Rgb(220, 180, 100)
        } else {
            Color::Rgb(100, 200, 140)
        };

        // MEM display
        let mem_pct = (p.mem / app.system_state.total_mem.max(1.0) as f32 * 100.0) as f64;
        let mem_bar = mini_bar(mem_pct, 6);
        let mem_color = if p.mem > 2048.0 {
            Color::Rgb(220, 100, 100)
        } else if p.mem > 512.0 {
            Color::Rgb(220, 180, 100)
        } else {
            Color::Rgb(100, 180, 220)
        };

        // Truncate name
        let name_display = if p.name.len() > name_width.saturating_sub(2) {
            format!("{}..", &p.name[..name_width.saturating_sub(4)])
        } else {
            p.name.clone()
        };

        let cells = vec![
            Cell::from(status_icon).style(Style::default().fg(status_color)),
            Cell::from(name_display).style(Style::default().fg(Color::Rgb(220, 225, 230)).add_modifier(Modifier::BOLD)),
            Cell::from(format!("{:>5.1}% {}", cpu_pct, cpu_bar)).style(Style::default().fg(cpu_color)),
            Cell::from(format!("{:>6} {}", format_memory_mib(p.mem as f64), mem_bar)).style(Style::default().fg(mem_color)),
            Cell::from(format!("{}", p.pid)).style(Style::default().fg(Color::Rgb(120, 125, 135))),
        ];

        let style = if is_selected {
            Style::default().bg(crate::ui::theme::accent_primary()).fg(Color::Black)
        } else if row_idx % 2 == 0 {
            Style::default().bg(Color::Rgb(25, 28, 32))
        } else {
            Style::default().bg(Color::Rgb(20, 23, 27))
        };

        Row::new(cells).style(style).height(1)
    }).collect();

    let table = Table::new(rows, [
        Constraint::Length(3),
        Constraint::Min(name_width as u16),
        Constraint::Length(14),
        Constraint::Length(16),
        Constraint::Length(10),
    ])
    .header(Row::new(header_cells).height(1).style(Style::default().fg(Color::Rgb(100, 105, 115))))
    .row_highlight_style(Style::default().bg(crate::ui::theme::accent_primary()).fg(Color::Black))
    .highlight_symbol("▶ ");

    let table_block = Block::default()
        .borders(Borders::TOP | Borders::BOTTOM)
        .border_style(Style::default().fg(Color::Rgb(40, 45, 55)));

    let mut table_state = TableState::default();
    if let Some(idx) = app.process_selected_idx {
        table_state.select(Some(idx));
    }

    f.render_stateful_widget(table.block(table_block), table_area, &mut table_state);

    // ── FOOTER ──
    let mut footer_spans = vec![
        Span::styled("↑↓", Style::default().fg(crate::ui::theme::accent_primary()).add_modifier(Modifier::BOLD)),
        Span::styled(" Navigate  ", Style::default().fg(Color::Rgb(100, 105, 115))),
        Span::styled("k", Style::default().fg(crate::ui::theme::accent_primary()).add_modifier(Modifier::BOLD)),
        Span::styled(" Kill  ", Style::default().fg(Color::Rgb(100, 105, 115))),
        Span::styled("c", Style::default().fg(crate::ui::theme::accent_primary()).add_modifier(Modifier::BOLD)),
        Span::styled(" Copy PID  ", Style::default().fg(Color::Rgb(100, 105, 115))),
        Span::styled("/", Style::default().fg(crate::ui::theme::accent_primary()).add_modifier(Modifier::BOLD)),
        Span::styled(" Search", Style::default().fg(Color::Rgb(100, 105, 115))),
    ];

    if has_filter {
        footer_spans.push(Span::styled("  |  ", Style::default().fg(Color::Rgb(60, 65, 75))));
        footer_spans.push(Span::styled("Esc", Style::default().fg(crate::ui::theme::accent_primary()).add_modifier(Modifier::BOLD)));
        footer_spans.push(Span::styled(" Clear filter", Style::default().fg(Color::Rgb(100, 105, 115))));
    }

    f.render_widget(Paragraph::new(Line::from(footer_spans)), footer_area);
}

fn format_memory_mib(mem_mib: f64) -> String {
    if mem_mib >= 1024.0 {
        format!("{:.1}G", mem_mib / 1024.0)
    } else {
        format!("{:.0}M", mem_mib)
    }
}

fn mini_bar(percent: f64, width: usize) -> String {
    let filled = ((percent / 100.0) * width as f64).round() as usize;
    let filled = filled.min(width);
    let empty = width.saturating_sub(filled);
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

fn process_status_color(status: &str) -> Color {
    match status {
        "Running" => Color::Rgb(100, 200, 100),
        "Sleeping" | "Idle" | "Waiting" | "Parked" => Color::Rgb(100, 150, 220),
        "Stopped" | "Traced" | "Stopped (Signal)" => Color::Rgb(220, 200, 100),
        "Zombie" | "Dead" => Color::Rgb(220, 80, 80),
        _ => Color::Gray,
    }
}

fn draw_processes_view(f: &mut Frame, area: Rect, app: &mut App) {
    // Layout: header (1) + list (fill) + footer (1)
    let header_height = 1;
    let footer_height = 1;
    let list_height = area.height.saturating_sub(header_height + footer_height);
    
    let header_area = Rect::new(area.x, area.y, area.width, header_height);
    let list_area = Rect::new(area.x, area.y + header_height, area.width, list_height);
    let footer_area = Rect::new(area.x, area.y + header_height + list_height, area.width, footer_height);

    // Filter processes
    let filtered_procs: Vec<(usize, &ProcessInfo)> = app.system_state.processes.iter().enumerate().filter(|(_, p)| {
        if app.process_search_filter.is_empty() {
            true
        } else {
            p.name.to_lowercase().contains(&app.process_search_filter.to_lowercase())
                || p.pid.to_string().contains(&app.process_search_filter)
                || p.user.to_lowercase().contains(&app.process_search_filter.to_lowercase())
        }
    }).collect();

    // ── HEADER ──
    let total_count = app.system_state.processes.len();
    let filtered_count = filtered_procs.len();
    
    let mut header_spans = vec![
        Span::styled(format!("{} processes", total_count), Style::default().fg(Color::Rgb(140, 145, 155))),
    ];
    if !app.process_search_filter.is_empty() {
        header_spans.push(Span::styled(" | ", Style::default().fg(Color::Rgb(60, 65, 75))));
        header_spans.push(Span::styled(format!("{} shown", filtered_count), Style::default().fg(crate::ui::theme::accent_primary())));
        header_spans.push(Span::styled(" | ", Style::default().fg(Color::Rgb(60, 65, 75))));
        header_spans.push(Span::styled(format!("filter: '{}'", app.process_search_filter), Style::default().fg(Color::Rgb(180, 185, 190))));
    }
    f.render_widget(Paragraph::new(Line::from(header_spans)), header_area);

    // ── PROCESS TABLE ──
    if filtered_procs.is_empty() {
        f.render_widget(
            Paragraph::new("No processes found")
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Rgb(80, 85, 95))),
            Rect::new(list_area.x, list_area.y + list_area.height / 2, list_area.width, 1),
        );
    } else {
        // Column widths: PID(8) Name(fill) User(10) Status(10) CPU(12) MEM(14)
        let name_width = list_area.width.saturating_sub(8 + 10 + 10 + 12 + 14 + 10) as usize;
        let constraints = [
            Constraint::Length(8),
            Constraint::Min(name_width.max(10) as u16),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(12),
            Constraint::Length(14),
        ];

        // Header row
        let headers = ["PID", "NAME", "USER", "STATUS", "CPU", "MEM"];
        let cols = [ProcessColumn::Pid, ProcessColumn::Name, ProcessColumn::User, ProcessColumn::Status, ProcessColumn::Cpu, ProcessColumn::Mem];
        
        app.process_column_bounds.clear();
        let mut current_x = list_area.x;
        let header_cells: Vec<Cell> = headers.iter().zip(cols.iter()).map(|(h, col)| {
            let width = match *col {
                ProcessColumn::Name => name_width.max(10) as u16,
                _ => match *h {
                    "PID" => 8, "USER" => 10, "STATUS" => 10, "CPU" => 12, "MEM" => 14,
                    _ => 10,
                },
            };
            app.process_column_bounds.push((Rect::new(current_x, list_area.y, width, 1), *col));
            current_x += width + 2;
            
            let is_sorted = app.process_sort_col == *col;
            let text = if is_sorted {
                format!("{} {}", h, if app.process_sort_asc { "▲" } else { "▼" })
            } else {
                h.to_string()
            };
            Cell::from(text).style(Style::default()
                .fg(if is_sorted { crate::ui::theme::accent_primary() } else { Color::Rgb(80, 85, 95) })
                .add_modifier(Modifier::BOLD))
        }).collect();

        // Data rows
        let selected_idx = app.process_selected_idx.unwrap_or(0);
        let visible_count = list_height as usize;
        let scroll_offset = if selected_idx >= visible_count {
            selected_idx.saturating_sub(visible_count / 2)
        } else {
            0
        };

        let rows: Vec<Row> = filtered_procs.iter().enumerate().skip(scroll_offset).take(visible_count).map(|(row_idx, (orig_idx, p))| {
            let is_selected = app.process_selected_idx == Some(*orig_idx);
            
            let cpu_color = if p.cpu > 50.0 { Color::Rgb(220, 100, 100) }
                else if p.cpu > 20.0 { Color::Rgb(220, 180, 100) }
                else { Color::Rgb(100, 200, 140) };
            let mem_color = if p.mem > 2048.0 { Color::Rgb(220, 100, 100) }
                else if p.mem > 512.0 { Color::Rgb(220, 180, 100) }
                else { Color::Rgb(100, 180, 220) };
            
            let name_display = if p.name.len() > name_width.saturating_sub(1) {
                format!("{}..", &p.name[..name_width.saturating_sub(3)])
            } else {
                p.name.clone()
            };

            let style = if is_selected {
                Style::default().bg(crate::ui::theme::accent_primary()).fg(Color::Black)
            } else if row_idx % 2 == 0 {
                Style::default().bg(Color::Rgb(25, 28, 32))
            } else {
                Style::default().bg(Color::Rgb(20, 23, 27))
            };

            Row::new(vec![
                Cell::from(format!("{}", p.pid)).style(Style::default().fg(Color::Rgb(120, 125, 135))),
                Cell::from(name_display).style(Style::default().fg(Color::Rgb(220, 225, 230)).add_modifier(Modifier::BOLD)),
                Cell::from(p.user.clone()).style(Style::default().fg(Color::Rgb(140, 145, 155))),
                Cell::from(p.status.clone()).style(Style::default().fg(process_status_color(&p.status))),
                Cell::from(format!("{:>5.1}% {}", p.cpu, mini_bar(p.cpu as f64, 4))).style(Style::default().fg(cpu_color)),
                Cell::from(format!("{:>6} {}", format_memory_mib(p.mem as f64), mini_bar((p.mem / app.system_state.total_mem.max(1.0) * 100.0) as f64, 4))).style(Style::default().fg(mem_color)),
            ]).style(style).height(1)
        }).collect();

        let table = Table::new(rows, constraints)
            .header(Row::new(header_cells).height(1).bottom_margin(0))
            .column_spacing(2);

        f.render_widget(table, list_area);
        
        // Update table state
        app.process_table_state.select(Some(selected_idx.saturating_sub(scroll_offset)));
    }

    // ── FOOTER ──
    let footer_spans = vec![
        Span::styled("↑↓", Style::default().fg(crate::ui::theme::accent_primary()).add_modifier(Modifier::BOLD)),
        Span::styled(" Navigate  ", Style::default().fg(Color::Rgb(100, 105, 115))),
        Span::styled("k", Style::default().fg(crate::ui::theme::accent_primary()).add_modifier(Modifier::BOLD)),
        Span::styled(" Kill  ", Style::default().fg(Color::Rgb(100, 105, 115))),
        Span::styled("c", Style::default().fg(crate::ui::theme::accent_primary()).add_modifier(Modifier::BOLD)),
        Span::styled(" Copy PID  ", Style::default().fg(Color::Rgb(100, 105, 115))),
        Span::styled("/", Style::default().fg(crate::ui::theme::accent_primary()).add_modifier(Modifier::BOLD)),
        Span::styled(" Search  ", Style::default().fg(Color::Rgb(100, 105, 115))),
        Span::styled("Click", Style::default().fg(crate::ui::theme::accent_primary()).add_modifier(Modifier::BOLD)),
        Span::styled(" Sort", Style::default().fg(Color::Rgb(100, 105, 115))),
    ];
    f.render_widget(Paragraph::new(Line::from(footer_spans)), footer_area);
}

fn draw_global_header(f: &mut Frame, area: Rect, sidebar_width: u16, app: &mut App) {
    let _now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    let pane_count = app.panes.len();

    // Toolbar Icons Cluster (Far Left)
    let back_icon = Icon::Back.get(app.icon_mode);
    let forward_icon = Icon::Forward.get(app.icon_mode);
    let split_icon = Icon::Split.get(app.icon_mode);
    let burger_icon = Icon::Burger.get(app.icon_mode);

    let monitor_icon = Icon::Monitor.get(app.icon_mode);
    let git_icon = Icon::Git.get(app.icon_mode);
    let editor_icon = Icon::Document.get(app.icon_mode);
    app.header_icon_bounds.clear();
    let mut cur_icon_x = area.x + 2;

    let show_icons = app.show_sidebar;

    if show_icons {
        let icons = [
            (burger_icon, "burger"),
            (back_icon, "back"),
            (forward_icon, "forward"),
            (monitor_icon, "monitor"),
            (git_icon, "git"),
            (editor_icon, "project"),
        ];

        for (i, (icon, id)) in icons.into_iter().enumerate() {
            let width = icon.width() as u16;
            let rect = Rect::new(cur_icon_x, area.y, width, 1);

            let mut style = Style::default().fg(crate::ui::theme::accent_secondary());
            if let AppMode::Header(idx) = app.mode {
                if idx == i {
                    style = style
                        .bg(crate::ui::theme::accent_primary())
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD);
                }
            }

            f.render_widget(Paragraph::new(icon).style(style), rect);
            app.header_icon_bounds.push((rect, id.to_string()));
            cur_icon_x += width + 2;
        }

        // Split icon at far right
        let split_width = split_icon.width() as u16;
        let split_x = area.x + area.width - split_width - 2;
        let split_rect = Rect::new(split_x, area.y, split_width, 1);
        let mut split_style = Style::default().fg(crate::ui::theme::accent_secondary());
        if let AppMode::Header(idx) = app.mode {
            if idx == 7 {
                split_style = split_style
                    .bg(crate::ui::theme::accent_primary())
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD);
            }
        }
        f.render_widget(Paragraph::new(split_icon).style(split_style), split_rect);
        app.header_icon_bounds.push((split_rect, "split".to_string()));
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

    app.tab_bounds.clear();
    let mut global_tab_idx = if show_icons { 6 } else { 0 };
    for (p_i, pane) in app.panes.iter().enumerate() {
        let chunk = pane_chunks[p_i];
        let mut current_x = chunk.x;

        if app.current_view == CurrentView::Editor {
            if pane.tabs.is_empty() {
                continue;
            }
            for (t_i, tab) in pane.tabs.iter().enumerate() {
                let is_active_tab = t_i == pane.active_tab_index;
                let is_focused_pane = p_i == app.focused_pane_index && !app.sidebar_focus;

                let is_modified = tab
                    .preview
                    .as_ref()
                    .and_then(|p| p.editor.as_ref())
                    .map(|e| e.modified)
                    .unwrap_or(false);

                let base_style = if is_active_tab {
                    if is_focused_pane {
                        Style::default()
                            .fg(crate::ui::theme::accent_primary())
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(crate::ui::theme::accent_primary())
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

                // Prefix remote server name for remote tabs
                let display_name = if let Some(ref remote) = tab.remote_session {
                    format!("{} {}", Icon::Remote.get(app.icon_mode), remote.name)
                } else {
                    base_name
                };

                let mut spans = vec![Span::styled(format!(" {}", display_name), base_style)];
                if is_modified {
                    spans.push(Span::styled(
                        " ●",
                        Style::default().fg(crate::ui::theme::accent_primary()),
                    ));
                }

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
                    app.tab_bounds.push((rect, p_i, t_i));
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

            // Prefix remote server name for remote tabs
            let display_name = if let Some(ref remote) = tab.remote_session {
                format!("{} {}", Icon::Remote.get(app.icon_mode), remote.name)
            } else {
                base_name
            };

            let is_active_tab = t_i == pane.active_tab_index;
            let is_focused_pane = p_i == app.focused_pane_index && !app.sidebar_focus;

            let mut base_style = if is_active_tab {
                if is_focused_pane {
                    Style::default()
                        .fg(crate::ui::theme::accent_primary())
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(crate::ui::theme::accent_primary())
                }
            } else {
                Style::default().fg(Color::DarkGray)
            };

            if let AppMode::Header(idx) = app.mode {
                if idx == global_tab_idx {
                    base_style = base_style
                        .bg(crate::ui::theme::accent_primary())
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD);
                }
            }

            spans.push(Span::styled(format!(" {} ", display_name), base_style));

            if matches!(app.current_view, CurrentView::Files | CurrentView::Git) {
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
            app.tab_bounds.push((rect, p_i, t_i));
            current_x += width + 1;
            global_tab_idx += 1;
        }
    }
}

fn draw_main_stage(f: &mut Frame, area: Rect, app: &mut App) {
    match app.current_view {
        CurrentView::Files => {
            let pane_count = app.panes.len();
            if pane_count == 0 {
                return;
            }

            let constraints = vec![Constraint::Fill(1); pane_count];
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(constraints)
                .spacing(1) // Add 1-column gap between panes to prevent bleed-through
                .split(area);
            for i in 0..pane_count {
                let is_focused = i == app.focused_pane_index && !app.sidebar_focus;
                let borders = Borders::ALL;
                draw_file_view(f, chunks[i], app, i, is_focused, borders);
            }
        }
        CurrentView::Editor => {
            crate::ui::panes::editor::draw_ide_editor(f, area, app);
        }
        _ => {}
    }
}

fn parse_commit_refs(decorations: &str) -> Vec<String> {
    decorations
        .trim()
        .trim_matches(|c| c == '(' || c == ')')
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

fn style_for_ref_label(label: &str) -> Style {
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

fn refs_line(refs: &[String], max_refs: usize) -> Line<'static> {
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

fn format_relative_time(date_str: &str) -> String {
    // If already relative (contains "ago"), shorten it
    if date_str.contains("ago") {
        return date_str
            .replace(" years", "y")
            .replace(" year", "y")
            .replace(" months", "mo")
            .replace(" month", "mo")
            .replace(" weeks", "w")
            .replace(" week", "w")
            .replace(" days", "d")
            .replace(" day", "d")
            .replace(" hours", "h")
            .replace(" hour", "h")
            .replace(" minutes", "m")
            .replace(" minute", "m")
            .replace(" seconds", "s")
            .replace(" second", "s")
            .replace(" ago", "")
            .replace("ago", "");
    }

    // Try to parse as a datetime
    let now = chrono::Local::now();
    let dt = chrono::DateTime::parse_from_rfc3339(date_str)
        .map(|d| d.with_timezone(&chrono::Local))
        .or_else(|_| chrono::DateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S %z").map(|d| d.with_timezone(&chrono::Local)))
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S").map(|d| d.and_local_timezone(chrono::Local).unwrap()))
        .or_else(|_| chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d").map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_local_timezone(chrono::Local).unwrap()));

    if let Ok(dt) = dt {
        let duration = now.signed_duration_since(dt);
        let secs = duration.num_seconds();
        if secs < 60 {
            return format!("{}s", secs);
        } else if secs < 3600 {
            return format!("{}m", secs / 60);
        } else if secs < 86400 {
            return format!("{}h", secs / 3600);
        } else if secs < 604800 {
            return format!("{}d", secs / 86400);
        } else if secs < 2592000 {
            return format!("{}w", secs / 604800);
        } else if secs < 31536000 {
            return format!("{}mo", secs / 2592000);
        } else {
            return format!("{}y", secs / 31536000);
        }
    }

    // Fallback: return truncated original
    if date_str.len() > 12 {
        format!("{}..", &date_str[..10])
    } else {
        date_str.to_string()
    }
}

fn author_color(author: &str) -> Color {
    let colors = [
        Color::Cyan,
        Color::Magenta,
        Color::Yellow,
        Color::Green,
        Color::Blue,
        Color::LightCyan,
        Color::LightMagenta,
        Color::LightYellow,
        Color::LightGreen,
        Color::LightBlue,
    ];
    let mut hash = 0u64;
    for byte in author.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
    }
    colors[(hash % colors.len() as u64) as usize]
}

fn draw_git_page(f: &mut Frame, area: Rect, app: &mut App) {
    let pane_idx = app.focused_pane_index;
    let tab_idx = if let Some(pane) = app.panes.get(pane_idx) {
        pane.active_tab_index
    } else {
        0
    };

    let Some(tab) = app.panes.get(pane_idx).and_then(|p| p.tabs.get(tab_idx)) else {
        return;
    };

    let branch_name = tab.git_branch.as_deref().unwrap_or("HEAD");
    let summary_text = tab.git_summary.as_deref().unwrap_or("");
    let current_path_label = tab.current_path.to_string_lossy().to_string();
    let history_len = tab.git_history.len();
    let pending_len = tab.git_pending.len();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(crate::ui::theme::accent_primary()))
        .style(Style::default().bg(Color::Rgb(0, 0, 0)))
        .title_top(Line::from(vec![
            Span::styled(
                " GIT HUB ",
                Style::default()
                    .fg(Color::Black)
                    .bg(crate::ui::theme::accent_primary())
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
                Style::default().fg(crate::ui::theme::accent_secondary()),
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

    // Status bar: ahead/behind, stash count
    let status_bar_h = 1u16;
    let status_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(status_bar_h), Constraint::Min(0)])
        .split(inner);

    let status_area = status_chunks[0];
    let content_area = status_chunks[1];

    // Reserve space for search bar when in Search mode
    let is_searching = matches!(app.mode, crate::state::AppMode::Search);
    let search_h = if is_searching { 1u16 } else { 0u16 };
    let content_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(search_h)])
        .split(content_area);

    let main_area = content_chunks[0];
    let search_area = content_chunks[1];

    // Build status line with stable layout
    // Left side: branch, ahead/behind, stash
    let mut left_spans = vec![
        Span::styled(
            "  ",
            Style::default().fg(crate::ui::theme::accent_primary()),
        ),
        Span::styled(
            branch_name,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ];

    if tab.git_ahead > 0 || tab.git_behind > 0 {
        left_spans.push(Span::raw("  " ));
        if tab.git_ahead > 0 {
            left_spans.push(Span::styled(
                format!("↑{} ", tab.git_ahead),
                Style::default().fg(Color::Green),
            ));
        }
        if tab.git_behind > 0 {
            left_spans.push(Span::styled(
                format!("↓{} ", tab.git_behind),
                Style::default().fg(Color::Red),
            ));
        }
    }

    if !tab.git_stashes.is_empty() {
        left_spans.push(Span::styled(
            format!(" 󰆓 {} ", tab.git_stashes.len()),
            Style::default().fg(Color::Magenta),
        ));
    }

    // Right side: platform icons (always rendered in fixed positions)
    let mut right_spans = Vec::new();
    
    // Detect platforms from remotes
    let mut has_github = false;
    let mut has_gitlab = false;
    let mut has_codeberg = false;
    
    for remote in &tab.git_remotes {
        let parts: Vec<&str> = remote.split_whitespace().collect();
        if parts.len() >= 2 {
            let url = parts[1];
            if url.contains("github.com") {
                has_github = true;
            } else if url.contains("gitlab.com") {
                has_gitlab = true;
            } else if url.contains("codeberg.org") {
                has_codeberg = true;
            }
        }
    }
    
    // Always render icons in fixed order with consistent spacing
    if has_github {
        right_spans.push(Span::styled("", Style::default().fg(Color::White)));
    } else {
        right_spans.push(Span::raw("  ")); // Reserve space
    }
    right_spans.push(Span::raw(" "));
    
    if has_gitlab {
        right_spans.push(Span::styled("", Style::default().fg(Color::Rgb(226, 67, 41))));
    } else {
        right_spans.push(Span::raw("  ")); // Reserve space
    }
    right_spans.push(Span::raw(" "));
    
    if has_codeberg {
        right_spans.push(Span::styled("󰚾", Style::default().fg(Color::Rgb(30, 160, 90))));
    } else {
        right_spans.push(Span::raw("  ")); // Reserve space
    }

    // Render in a horizontal layout: left | right
    let status_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Fill(1), Constraint::Length(8)])
        .split(status_area);

    f.render_widget(
        Paragraph::new(Line::from(left_spans)),
        status_chunks[0],
    );
    
    f.render_widget(
        Paragraph::new(Line::from(right_spans)).alignment(Alignment::Right),
        status_chunks[1],
    );

    // Check if we have an inline diff to show
    let (diff_loaded, diff_content) = app
        .panes
        .get(pane_idx)
        .and_then(|p| p.tabs.get(tab_idx))
        .map(|t| {
            if let Some(pending_idx) = t.git_pending_state.selected() {
                if let Some(change) = t.git_pending.get(pending_idx) {
                    if t.git_diff_for_path.as_ref() == Some(&change.path) {
                        if let Some(diff) = &t.git_pending_diff {
                            return (true, Some(diff.clone()));
                        }
                    }
                }
            }
            (false, None)
        })
        .unwrap_or((false, None));

    // Stable layout: always use split when there are pending changes
    let top_h = if pending_len == 0 {
        0
    } else {
        (pending_len as u16 + 2).max(10).min(main_area.height / 3)
    };

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(top_h), Constraint::Min(0)])
        .split(main_area);

    let top_area = main_chunks[0];
    let history_area = main_chunks[1];

    if top_h > 0 {
        let active_area = top_area;

        if pending_len > 0 {
            let active_title = format!(" ACTIVE ({} Affected) ", pending_len);

            // Build tree-structured rows
            let mut pending_rows: Vec<Row> = Vec::new();
            let mut last_dir: Option<String> = None;

            if let Some(t) = app.panes.get(pane_idx).and_then(|p| p.tabs.get(tab_idx)) {
                // Sort by path for tree grouping
                let mut sorted: Vec<_> = t.git_pending.iter().collect();
                sorted.sort_by_key(|p| &p.path);

                for p in sorted {
                    let status_color = match p.status.as_str() {
                        "M" => Color::Yellow,
                        "A" | "??" => Color::Green,
                        "D" => Color::Red,
                        "R" => Color::Cyan,
                        _ => Color::White,
                    };

                    // Split path into dir and filename
                    let path_obj = std::path::Path::new(&p.path);
                    let parent = path_obj.parent()
                        .map(|d| d.to_string_lossy().to_string())
                        .filter(|d| !d.is_empty());
                    let filename = path_obj.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| p.path.clone());

                    // Show directory header when it changes
                    if let Some(ref dir) = parent {
                        if last_dir.as_ref() != Some(dir) {
                            pending_rows.push(Row::new(vec![
                                Cell::from(""),
                                Cell::from(Line::from(vec![
                                    Span::styled(" ", Style::default().fg(Color::DarkGray)),
                                    Span::styled(dir.clone(), Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD)),
                                ])),
                                Cell::from(""),
                            ]));
                            last_dir = Some(dir.clone());
                        }
                    } else if last_dir.is_some() {
                        // Root-level file after directory files
                        last_dir = None;
                    }

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

                    pending_rows.push(Row::new(vec![
                        Cell::from(format!(" {} ", p.status)).style(
                            Style::default()
                                .bg(status_color)
                                .fg(Color::Black)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Cell::from(Line::from(vec![
                            Span::styled("  ", Style::default()),
                            Span::styled(filename, Style::default().fg(THEME.fg)),
                        ])),
                        Cell::from(Line::from(stats_spans)),
                    ]));
                }
            }

            let pending_table = Table::new(
                pending_rows,
                [
                    Constraint::Length(6),
                    Constraint::Fill(1),
                    Constraint::Length(15),
                ],
            )
            .row_highlight_style(
                Style::default()
                    .bg(Color::Rgb(40, 40, 50))
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );

            if let Some(pane) = app.panes.get_mut(pane_idx) {
                if let Some(tab) = pane.tabs.get_mut(tab_idx) {
                    // Always split: file list (38%) | diff/placeholder (62%)
                    let h_chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
                        .split(active_area);

                    let file_list_area = h_chunks[0];
                    let diff_area = h_chunks[1];

                    // File list with title and right border
                    let file_list_block = Block::default()
                        .title(active_title)
                        .border_style(Style::default().fg(Color::Rgb(40, 45, 55)))
                        .borders(Borders::RIGHT);
                    let file_list_inner = file_list_block.inner(file_list_area);
                    f.render_widget(file_list_block, file_list_area);
                    f.render_stateful_widget(
                        pending_table,
                        file_list_inner,
                        &mut tab.git_pending_state,
                    );

                    // Diff preview or placeholder
                    if diff_loaded {
                        let diff_lines: Vec<Line> = diff_content
                            .unwrap_or_default()
                            .lines()
                            .take(diff_area.height as usize)
                            .map(|line| {
                                let style = if line.starts_with('+') && !line.starts_with("+++") {
                                    Style::default().fg(Color::Green)
                                } else if line.starts_with('-') && !line.starts_with("---") {
                                    Style::default().fg(Color::Red)
                                } else if line.starts_with("@@") {
                                    Style::default().fg(Color::Cyan)
                                } else if line.starts_with("diff ") || line.starts_with("index ") || line.starts_with("--- ") || line.starts_with("+++ ") {
                                    Style::default().fg(Color::DarkGray)
                                } else {
                                    Style::default().fg(THEME.fg)
                                };
                                Line::from(Span::styled(line.to_string(), style))
                            })
                            .collect();

                        let diff_block = Block::default()
                            .title(" DIFF ")
                            .border_style(Style::default().fg(Color::Rgb(40, 45, 55)))
                            .borders(Borders::LEFT);
                        let diff_inner = diff_block.inner(diff_area);
                        f.render_widget(diff_block, diff_area);
                        f.render_widget(
                            Paragraph::new(diff_lines)
                                .wrap(ratatui::widgets::Wrap { trim: false }),
                            diff_inner,
                        );
                    } else {
                        // Placeholder: stable empty state
                        let placeholder = if tab.git_pending_state.selected().is_some() {
                            vec![Line::from(vec![
                                Span::styled("󰔚 ", Style::default().fg(Color::DarkGray)),
                                Span::styled("Loading diff...", Style::default().fg(Color::DarkGray)),
                            ])]
                        } else {
                            vec![
                                Line::from(""),
                                Line::from(vec![
                                    Span::styled("󰈈 ", Style::default().fg(Color::DarkGray)),
                                    Span::styled("Select a file to view diff", Style::default().fg(Color::DarkGray)),
                                ]),
                                Line::from(""),
                                Line::from(vec![
                                    Span::styled(" ↑/↓ ", Style::default().fg(Color::Cyan)),
                                    Span::styled("Navigate  ", Style::default().fg(Color::DarkGray)),
                                    Span::styled("Enter ", Style::default().fg(Color::Cyan)),
                                    Span::styled("Full preview", Style::default().fg(Color::DarkGray)),
                                ]),
                                Line::from(vec![
                                    Span::styled(" s ", Style::default().fg(Color::Green)),
                                    Span::styled("Stage  ", Style::default().fg(Color::DarkGray)),
                                    Span::styled("u ", Style::default().fg(Color::Red)),
                                    Span::styled("Unstage  ", Style::default().fg(Color::DarkGray)),
                                    Span::styled("S ", Style::default().fg(Color::Green)),
                                    Span::styled("Stage all", Style::default().fg(Color::DarkGray)),
                                ]),
                                Line::from(vec![
                                    Span::styled(" U ", Style::default().fg(Color::Red)),
                                    Span::styled("Unstage all", Style::default().fg(Color::DarkGray)),
                                ]),
                            ]
                        };
                        let diff_block = Block::default()
                            .title(" DIFF ")
                            .border_style(Style::default().fg(Color::Rgb(40, 45, 55)))
                            .borders(Borders::LEFT);
                        let diff_inner = diff_block.inner(diff_area);
                        f.render_widget(diff_block, diff_area);
                        f.render_widget(
                            Paragraph::new(placeholder)
                                .alignment(Alignment::Center),
                            diff_inner,
                        );
                    }
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
        let (search_filter, is_searching) = app
            .panes
            .get(pane_idx)
            .and_then(|p| p.tabs.get(tab_idx))
            .map(|t| (t.git_search_filter.clone(), !t.git_search_filter.is_empty()))
            .unwrap_or_default();

        let filter_lower = search_filter.to_lowercase();

        let rows: Vec<_> = app
            .panes
            .get(pane_idx)
            .and_then(|p| p.tabs.get(tab_idx))
            .map(|t| {
                t.git_history
                    .iter()
                    .filter(|act| {
                        if filter_lower.is_empty() {
                            true
                        } else {
                            act.message.to_lowercase().contains(&filter_lower)
                                || act.author.to_lowercase().contains(&filter_lower)
                                || act.hash.to_lowercase().starts_with(&filter_lower)
                        }
                    })
                    .map(|act| {
                        let h_short = act.hash.chars().take(7).collect::<String>();
                        let refs = parse_commit_refs(&act.decorations);
                        let refs_compact = refs_line(&refs, 2);

                        let mut stats_cells = Vec::new();
                        if act.files_changed > 0 {
                            stats_cells.push(
                                Cell::from(format!("{}", act.files_changed))
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
                            Cell::from(act.graph.clone()).style(
                                Style::default().fg(Color::DarkGray)
                            ),
                            Cell::from(format_relative_time(&act.date))
                                .style(Style::default().fg(Color::DarkGray)),
                            Cell::from(h_short).style(
                                Style::default()
                                    .fg(crate::ui::theme::accent_secondary())
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Cell::from(refs_compact),
                            Cell::from(act.author.clone()).style(Style::default().fg(author_color(&act.author))),
                            Cell::from(act.message.clone()).style(Style::default().fg(THEME.fg)),
                        ];
                        row_cells.extend(stats_cells);

                        Row::new(row_cells)
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let title = if is_searching {
            format!(" HISTORY ({} / {}) — 󰈲 '{}' ", rows.len(), history_len, search_filter)
        } else {
            " HISTORY ".to_string()
        };

        let table = Table::new(
            rows,
            [
                Constraint::Length(12),
                Constraint::Length(6),
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
                "GRAPH", "DATE", "HASH", "REFS", "AUTHOR", "MESSAGE", "FILES", "ADD", "DEL",
            ])
            .style(
                Style::default()
                    .fg(crate::ui::theme::accent_secondary())
                    .add_modifier(Modifier::BOLD),
            )
            .bottom_margin(1),
        )
        .block(
            Block::default()
                .title(title)
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Color::Rgb(40, 45, 55))),
        )
        .row_highlight_style(
            Style::default()
                .bg(Color::Rgb(40, 40, 50))
                .fg(crate::ui::theme::accent_secondary())
                .add_modifier(Modifier::BOLD),
        );

        if let Some(pane) = app.panes.get_mut(pane_idx) {
            if let Some(tab) = pane.tabs.get_mut(tab_idx) {
                f.render_stateful_widget(table, history_area, &mut tab.git_history_state);
            }
        }
    }

    // Render search bar at bottom
    if is_searching {
        let search_spans = vec![
            Span::styled(" 󰈲 ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(
                app.input.value.clone(),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" _", Style::default().fg(Color::Cyan)),
        ];
        f.render_widget(
            Paragraph::new(Line::from(search_spans))
                .style(Style::default().bg(Color::Rgb(30, 30, 40))),
            search_area,
        );
    }
}

fn draw_file_view(
    f: &mut Frame,
    area: Rect,
    app: &mut App,
    pane_idx: usize,
    is_focused: bool,
    borders: Borders,
) {
    if let Some(pane) = app.panes.get_mut(pane_idx) {
        if let Some(fs) = pane.current_state_mut() {
            if let Some(preview) = &mut fs.preview {
            let block = Block::default()
                .borders(borders)
                .border_type(BorderType::Rounded)
                .title(format!(" Preview: {} ", preview.path.display()))
                .border_style(if is_focused {
                    Style::default().fg(crate::ui::theme::border_active())
                } else {
                    Style::default().fg(crate::ui::theme::border_inactive())
                });

            if let Some((rgba, w, h)) = preview.image_data.as_ref() {
                let max_w = area.width as usize;
                let max_h = (area.height.saturating_sub(3)) as usize;
                let w_val = *w as usize;
                let h_val = *h as usize;
                let scale_x = if w_val > 0 { max_w as f32 / w_val as f32 } else { 1.0 };
                let scale_y = if h_val > 0 { max_h as f32 / h_val as f32 } else { 1.0 };
                let scale = scale_x.min(scale_y).max(0.1);
                let new_w = ((w_val as f32 * scale) as u16).max(1);
                let new_h = ((h_val as f32 * scale) as u16).max(1);
                let img_area = Rect::new(
                    area.x.saturating_add((area.width.saturating_sub(new_w)) / 2),
                    area.y.saturating_add((area.height.saturating_sub(new_h + 3)) / 2),
                    new_w,
                    new_h,
                );
                // Draw image as ASCII block characters (fallback for all terminals)
                let chars = ["░", "▒", "▓", "█"];
                let step_x = (w_val / new_w as usize).max(1);
                let step_y = (h_val / new_h as usize).max(1);
                let mut img_text = String::new();
                for y in (0..h_val).step_by(step_y).take(new_h as usize) {
                    for x in (0..w_val).step_by(step_x) {
                        let idx = (y * w_val + x) * 4;
                        if idx + 2 < rgba.len() {
                            let r = rgba[idx] as usize;
                            let g = rgba[idx + 1] as usize;
                            let b = rgba[idx + 2] as usize;
                            let bright = (r + g + b) / 3;
                            let c = if bright > 200 { chars[3] } else if bright > 150 { chars[2] } else if bright > 100 { chars[1] } else { chars[0] };
                            img_text.push_str(c);
                        } else {
                            img_text.push(' ');
                        }
                    }
                    img_text.push('\n');
                }
                let img_block = Block::default()
                    .title(format!(" Image {}x{} ", w, h))
                    .borders(borders)
                    .border_type(BorderType::Rounded)
                    .border_style(if is_focused {
                        Style::default().fg(crate::ui::theme::border_active())
                    } else {
                        Style::default().fg(crate::ui::theme::border_inactive())
                    });
                f.render_widget(&img_block, img_area);
                let inner_img = img_block.inner(img_area);
                f.render_widget(Paragraph::new(img_text), inner_img);

                // Queue terminal-native image render if protocol is available
                if app.graphics_protocol != crate::term_graphics::GraphicsProtocol::None {
                    app.pending_image_render = Some(crate::term_graphics::PendingImageRender {
                        rgba: rgba.clone(),
                        width: *w,
                        height: *h,
                        area: inner_img,
                    });
                }
                return;
            }

            let lines = if let Some(cached) = &preview.highlighted_lines {
                cached.clone()
            } else {
                let language = preview
                    .path
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");

                // PERFORMANCE OPTIMIZATION: Only highlight what's likely to be visible + some buffer
                // This is a PREVIEW, so full file highlighting is overkill for large files.
                let content_to_highlight = if preview.content.lines().count() > 500 {
                    preview
                        .content
                        .lines()
                        .take(500)
                        .collect::<Vec<_>>()
                        .join("\n")
                } else {
                    preview.content.clone()
                };

                let highlighted = dracon_terminal_engine::utils::highlight_code(&content_to_highlight, language);
                let mut lines = Vec::new();
                for (i, line) in highlighted.iter().enumerate() {
                    let mut spans = line
                        .spans
                        .iter()
                        .map(|s| Span::styled(s.content.to_string(), s.style))
                        .collect::<Vec<_>>();
                    // Prepend line number gutter
                    let num = format!("{:>3} │ ", i + 1);
                    spans.insert(
                        0,
                        Span::styled(num, Style::default().fg(Color::Rgb(60, 60, 70))),
                    );
                    lines.push(Line::from(spans));
                }
                preview.highlighted_lines = Some(lines.clone());
                lines
            };

            let text = Paragraph::new(lines)
                .wrap(ratatui::widgets::Wrap { trim: false })
                .block(block);

            f.render_widget(text, area);
            return;
        }
        }
    }

    // --- BORDER & BACKGROUND (Rendered FIRST to create base) ---

    let border_style = if is_focused {
        let pulse = ((SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            % 1500) as f32
            / 1500.0
            * std::f32::consts::PI
            * 2.0)
            .sin()
            * 0.5
            + 0.5;

        let (base_r, base_g, base_b) = match crate::ui::theme::border_active() {
            Color::Rgb(r, g, b) => (r as f32, g as f32, b as f32),
            _ => (0.0, 150.0, 255.0),
        };
        let intensity = 0.7 + 0.3 * pulse;
        let r = (base_r * intensity) as u8;
        let g = (base_g * intensity) as u8;
        let b = (base_b * intensity) as u8;

        Style::default()
            .fg(Color::Rgb(r, g, b))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(crate::ui::theme::border_inactive())
    };


    let main_block = Block::default()
        .borders(borders)
        .border_type(BorderType::Rounded)
        .border_style(border_style);

    f.render_widget(main_block, area);

    draw_pane_breadcrumbs(f, area, app, pane_idx);

    if let Some(file_state) = app
        .panes
        .get_mut(pane_idx)
        .and_then(|p| p.current_state_mut())
    {
        file_state.view_height = area.height as usize;

        let mut render_state = TableState::default();

        if let Some(sel) = file_state.selection.selected {
            let offset = file_state.table_state.offset();

            let capacity = file_state.view_height.saturating_sub(3);

            if sel >= offset && sel < offset + capacity {
                render_state.select(Some(sel));
            }
        }

        *render_state.offset_mut() = file_state.table_state.offset();

        let mut display_columns = Vec::new();

        for col in &file_state.columns {
            match col {
                FileColumn::Name => display_columns.push(FileColumn::Name),
                FileColumn::Size if area.width > 40 => display_columns.push(FileColumn::Size),
                FileColumn::Modified if area.width > 70 => {
                    display_columns.push(FileColumn::Modified)
                }
                FileColumn::Created if area.width > 90 => display_columns.push(FileColumn::Created),
                FileColumn::Permissions if area.width > 110 => {
                    display_columns.push(FileColumn::Permissions)
                }
                _ => {}
            }
        }
        // Ensure Name is always there as a safety fallback
        if !display_columns.contains(&FileColumn::Name) {
            display_columns.insert(0, FileColumn::Name);
        }

        let constraints: Vec<Constraint> = display_columns
            .iter()
            .map(|c| match c {
                FileColumn::Name => Constraint::Fill(1),
                FileColumn::Size => Constraint::Length(12),
                FileColumn::Modified => Constraint::Length(20),
                FileColumn::Created => Constraint::Length(20),
                FileColumn::Permissions => Constraint::Length(12),
            })
            .collect();

        let dummy_block = Block::default().borders(borders);
        let inner_area = dummy_block.inner(area);
        let column_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(constraints.clone())
            .spacing(0)
            .split(inner_area);

        let header_lines: Vec<Line> = display_columns
            .iter()
            .map(|c| {
                let base_name = match c {
                    FileColumn::Name => "Name",
                    FileColumn::Size => "Size",
                    FileColumn::Modified => "Modified",
                    FileColumn::Created => "Created",
                    FileColumn::Permissions => "Permissions",
                };
                let name = if *c == file_state.sort_column {
                    if file_state.sort_ascending {
                        format!("{} ▲", base_name)
                    } else {
                        format!("{} ▼", base_name)
                    }
                } else {
                    base_name.to_string()
                };
                Line::from(vec![Span::styled(
                    name,
                    Style::default()
                        .fg(crate::ui::theme::header_fg())
                        .add_modifier(Modifier::BOLD),
                )])
            })
            .collect();

        // --- ABSOLUTE CELL ISOLATION RENDERING ---
        file_state.column_bounds.clear();
        let header_y = inner_area.y;
        let content_y = header_y + 1;
        let visible_height = inner_area.height.saturating_sub(1) as usize;

        // 1. Render Headers
        for (col_idx, rect) in column_layout.iter().enumerate() {
            if let Some(col_type) = display_columns.get(col_idx) {
                file_state.column_bounds.push((*rect, *col_type));
                let header_line = header_lines.get(col_idx).cloned().unwrap_or(Line::from(""));
                let header_rect = Rect::new(rect.x, header_y, rect.width, 1);
                let alignment = match col_type {
                    FileColumn::Name => ratatui::layout::Alignment::Left,
                    _ => ratatui::layout::Alignment::Right,
                };
                f.render_widget(
                    Paragraph::new(header_line).alignment(alignment),
                    header_rect,
                );
            }
        }

        // 2. Render Rows
        let offset_val = file_state.table_state.offset();
        let total_files = file_state.files.len();
        for i in 0..visible_height {
            let file_idx = offset_val + i;
            if file_idx >= total_files {
                break;
            }
            let row_y = content_y + i as u16;
            let path = &file_state.files[file_idx];
            let is_selected = file_state.selection.selected == Some(file_idx);
            let is_multi_selected = file_state.selection.multi.contains(&file_idx);

            let mut row_bg_style = Style::default();
            let is_hovered_drop =
                matches!(&app.hovered_drop_target, Some(DropTarget::Folder(p)) if p == path);

            if is_selected {
                row_bg_style = row_bg_style.bg(crate::ui::theme::selection_bg());
            } else if is_multi_selected {
                row_bg_style = row_bg_style.bg(Color::Rgb(78, 58, 112));
            } else if is_hovered_drop {
                row_bg_style = row_bg_style.bg(crate::ui::theme::accent_secondary());
            } else if let Some(&c) = app.path_colors.get(path) {
                let color = match c {
                    1 => Color::Red,
                    2 => Color::Green,
                    3 => Color::Yellow,
                    4 => Color::Blue,
                    5 => Color::Magenta,
                    6 => Color::Cyan,
                    _ => Color::Reset,
                };
                if color != Color::Reset {
                    row_bg_style = row_bg_style.bg(color);
                }
            }
            if row_bg_style.bg.is_some() {
                f.render_widget(
                    Block::default().style(row_bg_style),
                    Rect::new(inner_area.x, row_y, inner_area.width, 1),
                );
            }

            let metadata = file_state.metadata.get(path);
            for (col_idx, col_rect) in column_layout.iter().enumerate() {
                if let Some(col_type) = display_columns.get(col_idx) {
                    let cell_rect = Rect::new(col_rect.x, row_y, col_rect.width, 1);
                    let mut cell_style = if is_selected {
                        Style::default()
                            .fg(Color::Black)
                            .add_modifier(Modifier::BOLD)
                    } else if is_multi_selected {
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD)
                    } else if is_hovered_drop || app.path_colors.contains_key(path) {
                        Style::default()
                            .fg(Color::Black)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(THEME.fg)
                    };

                    let content = match col_type {
                        FileColumn::Name => {
                            if path.to_string_lossy() == "__DIVIDER__" {
                                cell_style = Style::default()
                                    .fg(Color::Yellow)
                                    .add_modifier(Modifier::BOLD);
                                "> Global results".to_string()
} else {
                                    let name =
                                    path.file_name().and_then(|n| n.to_str()).unwrap_or("..");
                                    let is_dir = metadata.map(|m| m.is_dir).unwrap_or(false);
                                    let is_symlink = metadata.map(|m| m.is_symlink).unwrap_or(false);
                                    let link_target = metadata.and_then(|m| m.link_target.as_ref());
                                    let cat = crate::modules::files::get_file_category(path);
                                    let icon_str = if is_symlink {
                                        "🔗 ".to_string()
                                    } else {
                                        Icon::get_for_path(path, cat, is_dir, app.icon_mode).to_string()
                                    };

                                    let depth = file_state.tree_file_depths.get(file_idx).copied().unwrap_or(0) as usize;
                                    let indent = "  ".repeat(depth);
                                    let is_expanded = is_dir && app.expanded_folders.contains(path);
                                    
                                    // Check if folder has children
                                    // Expanded: children are visible in tree_file_depths
                                    // Collapsed: need to check filesystem
                                    let has_children = if is_dir {
                                        if is_expanded {
                                            // Tree view expanded: next entry deeper means children exist
                                            let my_depth = file_state.tree_file_depths.get(file_idx).copied().unwrap_or(0);
                                            file_state.tree_file_depths.get(file_idx + 1)
                                                .map(|&d| d > my_depth)
                                                .unwrap_or(false)
                                        } else if file_state.remote_session.is_some() {
                                            // Remote: can't check without round-trip; don't mislead with arrows
                                            false
                                        } else {
                                            // Local: check if directory actually has any items
                                            !std::fs::read_dir(path)
                                                .map(|mut d| d.next().is_none())
                                                .unwrap_or(true)
                                        }
                                    } else {
                                        false
                                    };
                                    
                                    let marker = if is_dir {
                                        if is_expanded && has_children { "▾ " } else if has_children { "▸ " } else { "  " }
                                    } else {
                                        "  "
                                    };
                                    let (depth_indent, expand_marker) = (
                                        format!("{}{}", indent, marker),
                                        is_dir && !marker.is_empty(),
                                    );

                                    let mut suffix = String::new();
                                    if app.starred.contains(path) {
                                        suffix.push_str(" [*]");
                                    }
                                    if !is_selected
                                        && !is_multi_selected
                                        && !app.path_colors.contains_key(path)
                                        && !is_hovered_drop
                                        && app.semantic_coloring
                                    {
                                        if is_symlink {
                                            cell_style = cell_style.fg(Color::Cyan);
                                        } else if is_dir {
                                            cell_style =
                                                cell_style.fg(crate::ui::theme::accent_secondary());
                                        } else {
                                            cell_style = cell_style.fg(cat.cyber_color());
                                        }
                                    }
                                    let icon_w = icon_str.chars().map(get_visual_width).sum::<usize>();
                                    let marker_w = if expand_marker { 2 } else { 0 };
                                    // 12 = leading space (1) + minimal trailing pad + room for "[*]" suffix (4)
                                    const CELL_TEXT_RESERVE: usize = 12;
                                    let available_width =
                                        (col_rect.width as usize).saturating_sub(icon_w + marker_w + CELL_TEXT_RESERVE);

                                    let display_name = if file_idx > file_state.local_count {
                                        let full_str = path.to_string_lossy();
                                        let home = dirs::home_dir()
                                            .map(|p| p.to_string_lossy().to_string())
                                            .unwrap_or_else(|| "/root".to_string());
                                        if full_str.starts_with(&home) {
                                            full_str.replacen(&home, "~", 1)
                                        } else {
                                            full_str.to_string()
                                        }
                                    } else {
                                        name.to_string()
                                    };
                                    let display_name = squarify(&display_name);

                                    // Append symlink target if there's room
                                    let name_with_link = if let Some(target) = link_target {
                                        format!("{} -> {}", display_name, target)
                                    } else {
                                        display_name.clone()
                                    };

                                    let truncated_name =
                                        truncate_to_width(&name_with_link, available_width, "..");
                                    let cell_text = if depth_indent.is_empty() {
                                        format!(" {} {}{}", icon_str, truncated_name, suffix)
                                    } else {
                                        format!("{}{} {}{}", depth_indent, icon_str, truncated_name, suffix)
                                    };

                                    cell_text
                            }
                        }
                        FileColumn::Size => {
                            let is_dir = metadata.map(|m| m.is_dir).unwrap_or(false);
                            let size = if is_dir {
                                // Use computed folder size if available
                                file_state.folder_sizes.get(path).copied()
                                    .or_else(|| metadata.map(|m| m.size))
                                    .unwrap_or(0)
                            } else {
                                metadata.map(|m| m.size).unwrap_or(0)
                            };
                            let text = if is_dir && size == 0 {
                                "<DIR>".to_string()
                            } else {
                                format_size(size)
                            };
                            truncate_to_width(
                                &text,
                                (col_rect.width as usize).saturating_sub(2),
                                "",
                            )
                        }
                        FileColumn::Modified => {
                            let text = format_modified_time(
                                metadata
                                    .map(|m| m.modified)
                                    .unwrap_or(SystemTime::UNIX_EPOCH),
                                app.smart_date,
                            );
                            truncate_to_width(
                                &text,
                                (col_rect.width as usize).saturating_sub(2),
                                "",
                            )
                        }
                        FileColumn::Permissions => {
                            let text =
                                format_permissions(metadata.map(|m| m.permissions).unwrap_or(0));
                            truncate_to_width(
                                &text,
                                (col_rect.width as usize).saturating_sub(2),
                                "",
                            )
                        }
                        FileColumn::Created => {
                            let text = format_modified_time(
                                metadata
                                    .map(|m| m.created)
                                    .unwrap_or(SystemTime::UNIX_EPOCH),
                                app.smart_date,
                            );
                            truncate_to_width(
                                &text,
                                (col_rect.width as usize).saturating_sub(2),
                                "",
                            )
                        }
                    };
                    let alignment = match col_type {
                        FileColumn::Name => ratatui::layout::Alignment::Left,
                        _ => ratatui::layout::Alignment::Right,
                    };
                    f.render_widget(
                        Paragraph::new(Span::styled(content, cell_style)).alignment(alignment),
                        cell_rect,
                    );
                }
            }
        }

        if total_files > area.height.saturating_sub(4) as usize {
            let scrollbar = Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .end_symbol(Some("▼"));

            let mut scroll_state = ScrollbarState::new(file_state.files.len())
                .position(file_state.table_state.offset())
                .viewport_content_length(inner_area.height as usize);

            f.render_stateful_widget(scrollbar, area, &mut scroll_state);
        }
    }
}

fn draw_stat_bar(
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

fn draw_footer(f: &mut Frame, area: Rect, app: &mut App) {
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
    if let Some((msg, time)) = &app.last_action_msg {
        if time.elapsed().as_secs() < 5 {
            left_spans.push(Span::styled(
                format!(" [ SYSTEM ] {} ", msg),
                Style::default()
                    .fg(crate::ui::theme::accent_secondary())
                    .bg(Color::Rgb(20, 25, 30)),
            ));
            showing_log = true;
        }
    }

    if app.is_dragging {
        if let Some(src) = &app.drag_source {
            let name = src.file_name().and_then(|n| n.to_str()).unwrap_or("...");
            left_spans.push(Span::styled(
                " DRAGGING ",
                Style::default()
                    .fg(Color::Black)
                    .bg(crate::ui::theme::accent_primary())
                    .add_modifier(Modifier::BOLD),
            ));
            left_spans.push(Span::styled(
                format!(" {} ", name),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ));

            if let Some(target) = &app.hovered_drop_target {
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
                        .fg(crate::ui::theme::accent_secondary())
                        .add_modifier(Modifier::BOLD),
                ));
            }
            showing_log = true; // Use this to skip shortcuts
        }
    }

    if !showing_log {
        left_spans.extend(HotkeyHint::render("^Q", "Quit", Color::Red));

        let hidden_on = if let Some(fs) = app.current_file_state() {
            fs.show_hidden
        } else {
            app.default_show_hidden
        };

        let mut shortcuts = Vec::new();
        if app.current_view == CurrentView::Editor {
            shortcuts.extend(HotkeyHint::render("Esc", "Back", Color::Red));
            shortcuts.extend(HotkeyHint::render(
                "^B",
                "Sidebar",
                crate::ui::theme::accent_secondary(),
            ));
            shortcuts.extend(HotkeyHint::render(
                "^P",
                "Split",
                crate::ui::theme::accent_secondary(),
            ));
            shortcuts.extend(HotkeyHint::render(
                "^F",
                "Find",
                crate::ui::theme::accent_secondary(),
            ));
            shortcuts.extend(HotkeyHint::render(
                "^R",
                "Replace",
                crate::ui::theme::accent_secondary(),
            ));
            shortcuts.extend(HotkeyHint::render(
                "^G",
                "GoTo",
                crate::ui::theme::accent_secondary(),
            ));
            shortcuts.extend(HotkeyHint::render(
                "^↵",
                "Run",
                crate::ui::theme::accent_secondary(),
            ));
        } else {
            shortcuts.extend(HotkeyHint::render(
                "^P",
                "Split",
                crate::ui::theme::accent_secondary(),
            ));
            shortcuts.extend(HotkeyHint::render(
                "^T",
                "Tab",
                crate::ui::theme::accent_secondary(),
            ));
            shortcuts.extend(HotkeyHint::render(
                "^N",
                "TermTab",
                crate::ui::theme::accent_secondary(),
            ));
            shortcuts.extend(HotkeyHint::render(
                "^K",
                "TermWin",
                crate::ui::theme::accent_secondary(),
            ));
            shortcuts.extend(HotkeyHint::render(
                "^R",
                "Run",
                crate::ui::theme::accent_secondary(),
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

        // Add Remote Status Badge - show server name for active pane
        if let Some(fs) = app.current_file_state() {
            if let Some(remote) = &fs.remote_session {
                let health_color = if let Some((healthy, last_check)) = app.remote_health.get(&remote.name) {
                    if !healthy {
                        Color::Red
                    } else if last_check.elapsed().as_secs() > 60 {
                        Color::Yellow
                    } else {
                        Color::Green
                    }
                } else {
                    Color::Gray
                };
                left_spans.push(Span::raw(" │ "));
                left_spans.push(Span::styled(
                    "●",
                    Style::default().fg(health_color).add_modifier(Modifier::BOLD),
                ));
                left_spans.push(Span::styled(
                    format!(
                        " {} {} ",
                        Icon::Remote.get(app.icon_mode),
                        remote.name
                    ),
                    Style::default()
                        .bg(crate::ui::theme::accent_secondary())
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                ));
            }
        }
    }

    f.render_widget(
        Paragraph::new(Line::from(left_spans)).wrap(ratatui::widgets::Wrap { trim: false }),
        top_chunks[0],
    );

    // 2. Center Section: Selection Summary (Only in Files view)
    if app.current_view == CurrentView::Files {
        if let Some(fs) = app.current_file_state() {
            let sel_count = if !fs.selection.is_empty() {
                fs.selection.multi.len()
            } else if fs.selection.selected.is_some() {
                1
            } else {
                0
            };
            let total_count = fs.files.len();
            let selected_bytes = if !fs.selection.is_empty() {
                let mut sum = 0u64;
                for &idx in fs.selection.multi_selected_indices() {
                    if let Some(path) = fs.files.get(idx) {
                        if let Some(meta) = fs.metadata.get(path) {
                            if !meta.is_dir {
                                sum = sum.saturating_add(meta.size);
                            }
                        }
                    }
                }
                sum
            } else if let Some(idx) = fs.selection.selected {
                if let Some(path) = fs.files.get(idx) {
                    fs.metadata
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
            let summary_style = if app.sidebar_focus {
                Style::default()
                    .bg(Color::Rgb(85, 80, 20))
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .bg(crate::ui::theme::accent_primary())
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
            Style::default().fg(crate::ui::theme::accent_primary()),
        ),
    ];

    f.render_widget(
        Paragraph::new(Line::from(pulse_spans)).alignment(ratatui::layout::Alignment::Right),
        stats_layout[2],
    );

    // 5. Bottom Line: Background Tasks
    let mut task_spans = Vec::new();
    for task in &app.background_tasks {
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

fn draw_context_menu(
    f: &mut Frame,
    x: u16,
    y: u16,
    target: &crate::app::ContextMenuTarget,
    app: &App,
) {
    use crate::app::ContextMenuAction;
    let mut items = Vec::new();

    let actions = if let AppMode::ContextMenu { actions, .. } = &app.mode {
        actions.clone()
    } else {
        vec![]
    };

    let selected_idx = if let AppMode::ContextMenu { selected_index, .. } = &app.mode {
        *selected_index
    } else {
        None
    };

    for (i, action) in actions.iter().enumerate() {
        let label = match action {
            ContextMenuAction::Open => format!(" {} Open", Icon::Folder.get(app.icon_mode)),
            ContextMenuAction::OpenNewTab => {
                format!(" {} Open in New Tab", Icon::Split.get(app.icon_mode))
            }
            ContextMenuAction::OpenWith => {
                format!(" {} Open With...", Icon::Split.get(app.icon_mode))
            }
            ContextMenuAction::Edit => format!(" {} Edit", Icon::Document.get(app.icon_mode)),
            ContextMenuAction::Run => format!(" {} Run", Icon::Video.get(app.icon_mode)),
            ContextMenuAction::RunTerminal => {
                format!(" {} Run in Terminal", Icon::Script.get(app.icon_mode))
            }
            ContextMenuAction::ExtractHere => {
                format!(" {} Extract Here", Icon::Archive.get(app.icon_mode))
            }
            ContextMenuAction::NewFolder => {
                format!(" {} New Folder", Icon::Folder.get(app.icon_mode))
            }
            ContextMenuAction::NewFile => format!(" {} New File", Icon::File.get(app.icon_mode)),
            ContextMenuAction::Cut => format!(" {} Cut", Icon::Cut.get(app.icon_mode)),
            ContextMenuAction::Copy => format!(" {} Copy", Icon::Copy.get(app.icon_mode)),
            ContextMenuAction::CopyPath => format!(" {} Copy Path", Icon::Copy.get(app.icon_mode)),
            ContextMenuAction::CopyName => format!(" {} Copy Name", Icon::Copy.get(app.icon_mode)),
            ContextMenuAction::Paste => format!(" {} Paste", Icon::Paste.get(app.icon_mode)),
            ContextMenuAction::Rename => format!(" {} Rename", Icon::Rename.get(app.icon_mode)),
            ContextMenuAction::Duplicate => {
                format!(" {} Duplicate", Icon::Duplicate.get(app.icon_mode))
            }
            ContextMenuAction::Compress => {
                format!(" {} Compress", Icon::Archive.get(app.icon_mode))
            }
            ContextMenuAction::Delete => format!(" {} Delete", Icon::Delete.get(app.icon_mode)),
            ContextMenuAction::AddToFavorites => {
                format!(" {} Add to Favorites", Icon::Star.get(app.icon_mode))
            }
            ContextMenuAction::RemoveFromFavorites => {
                format!(" {} Remove from Favorites", Icon::Star.get(app.icon_mode))
            }
            ContextMenuAction::Properties => {
                format!(" {} Properties", Icon::Document.get(app.icon_mode))
            }
            ContextMenuAction::TerminalWindow => {
                format!(" {} New Terminal Window", Icon::Script.get(app.icon_mode))
            }
            ContextMenuAction::TerminalTab => {
                format!(" {} New Terminal Tab", Icon::Script.get(app.icon_mode))
            }
            ContextMenuAction::Refresh => format!(" {} Refresh", Icon::Refresh.get(app.icon_mode)),
            ContextMenuAction::SelectAll => {
                format!(" {} Select All", Icon::SelectAll.get(app.icon_mode))
            }
            ContextMenuAction::ToggleHidden => {
                format!(" {} Toggle Hidden", Icon::ToggleHidden.get(app.icon_mode))
            }
            ContextMenuAction::ConnectRemote => {
                format!(" {} Connect", Icon::Remote.get(app.icon_mode))
            }
            ContextMenuAction::DeleteRemote => {
                format!(" {} Delete Bookmark", Icon::Delete.get(app.icon_mode))
            }
            ContextMenuAction::Mount => format!(" {} Mount", Icon::Storage.get(app.icon_mode)),
            ContextMenuAction::Unmount => format!(" {} Unmount", Icon::Storage.get(app.icon_mode)),
            ContextMenuAction::SetWallpaper => {
                format!(" {} Set as Wallpaper", Icon::Image.get(app.icon_mode))
            }
            ContextMenuAction::GitInit => format!(" {} Git Init", Icon::Git.get(app.icon_mode)),
            ContextMenuAction::GitStatus => format!(" {} Git Status", Icon::Git.get(app.icon_mode)),
            ContextMenuAction::SystemMonitor => {
                format!(" {} System Monitor", Icon::Monitor.get(app.icon_mode))
            }
            ContextMenuAction::CollapseAll => {
                format!(" {} Collapse All", Icon::Folder.get(app.icon_mode))
            }
            ContextMenuAction::Compare => format!(" {} Compare", Icon::Document.get(app.icon_mode)),
            ContextMenuAction::Download => format!(" {} Download", Icon::File.get(app.icon_mode)),
            ContextMenuAction::SetColor(_) => {
                format!(" {} Highlight...", Icon::Image.get(app.icon_mode))
            }
            ContextMenuAction::SortBy(col) => {
                let name = match col {
                    crate::app::FileColumn::Name => "Name",
                    crate::app::FileColumn::Size => "Size",
                    crate::app::FileColumn::Modified => "Date",
                    _ => "Unknown",
                };
                let mut label = format!(" 󰒺 Sort by {}", name);
                if let Some(fs) = app.current_file_state() {
                    if fs.sort_column == *col {
                        label.push_str(if fs.sort_ascending {
                            " (▲)"
                        } else {
                            " (▼)"
                        });
                    }
                }
                label
            }
            ContextMenuAction::Save => format!(" {} Save", Icon::Document.get(app.icon_mode)),
            ContextMenuAction::EditorCut => format!(" {} Cut", Icon::Cut.get(app.icon_mode)),
            ContextMenuAction::EditorCopy => format!(" {} Copy", Icon::Copy.get(app.icon_mode)),
            ContextMenuAction::EditorPaste => format!(" {} Paste", Icon::Paste.get(app.icon_mode)),
            ContextMenuAction::EditorUndo => format!(" {} Undo", Icon::Refresh.get(app.icon_mode)),
            ContextMenuAction::EditorRedo => format!(" {} Redo", Icon::Refresh.get(app.icon_mode)),
            ContextMenuAction::EditorSelectAll => {
                format!(" {} Select All", Icon::SelectAll.get(app.icon_mode))
            }
            ContextMenuAction::Undo => format!(" {} Undo", Icon::Refresh.get(app.icon_mode)),
            ContextMenuAction::Redo => format!(" {} Redo", Icon::Refresh.get(app.icon_mode)),
            ContextMenuAction::Separator => " ────────────────".to_string(),
            ContextMenuAction::Drag => " 󰓂 Drag".to_string(),
        };

        let style = if Some(i) == selected_idx {
            Style::default()
                .bg(crate::ui::theme::accent_primary())
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(THEME.fg)
        };

        let mut item = ListItem::new(label).style(style);
        if (*action == ContextMenuAction::Paste) && app.clipboard.is_none() {
            item = item.style(Style::default().fg(Color::DarkGray));
        }
        if *action == ContextMenuAction::Separator {
            item = item.style(Style::default().fg(Color::DarkGray));
        }
        items.push(item);
    }

    let title = match target {
        crate::app::ContextMenuTarget::File(_) => " File ",
        crate::app::ContextMenuTarget::Folder(_) => " Folder ",
        crate::app::ContextMenuTarget::EmptySpace => " View ",
        crate::app::ContextMenuTarget::SidebarFavorite(_) => " Favorite ",
        crate::app::ContextMenuTarget::SidebarRemote(_) => " Remote ",
        crate::app::ContextMenuTarget::SidebarStorage(_) => " Storage ",
        crate::app::ContextMenuTarget::ProjectTree(_) => " Project ",
        crate::app::ContextMenuTarget::Process(_) => " Process ",
        crate::app::ContextMenuTarget::Editor => " Editor ",
    };

    let menu_width = 30;
    let menu_height = items.len() as u16 + 2;
    let mut draw_x = x;
    let mut draw_y = y;
    if draw_x + menu_width > f.area().width {
        draw_x = f.area().width.saturating_sub(menu_width);
    }
    if draw_y + menu_height > f.area().height {
        draw_y = f.area().height.saturating_sub(menu_height);
    }

    let area = Rect::new(draw_x, draw_y, menu_width, menu_height);

    let menu_block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(crate::ui::theme::accent_secondary()));

    // Use full width of inner area, just offset X by 1 for padding
    let inner_area = menu_block.inner(area);
    let padded_area = Rect::new(
        inner_area.x,
        inner_area.y,
        inner_area.width,
        inner_area.height,
    );

    f.render_widget(menu_block, area);
    f.render_widget(List::new(items), padded_area);
}

fn draw_import_servers_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 20, f.area());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Import Servers (TOML) ")
        .border_style(Style::default().fg(crate::ui::theme::accent_primary()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .split(inner);

    f.render_widget(
        Paragraph::new("Enter path to server configuration file:"),
        chunks[0],
    );

    let input_area = chunks[1];
    f.render_widget(
        Paragraph::new("> ").style(Style::default().fg(crate::ui::theme::accent_secondary())),
        Rect::new(input_area.x, input_area.y, 2, 1),
    );
    f.render_widget(
        &app.input,
        Rect::new(
            input_area.x + 2,
            input_area.y,
            input_area.width.saturating_sub(2),
            1,
        ),
    );

    let example_toml = r#"Example format:
[[servers]]
name = "Production"
host = "192.168.1.10"
user = "admin"
port = 22"#;

    f.render_widget(
        Paragraph::new(example_toml).style(Style::default().fg(Color::DarkGray)),
        chunks[2],
    );

    let mut footer_text = Vec::new();
    footer_text.extend(HotkeyHint::render("Esc", "Cancel", Color::Red));
    footer_text.extend(HotkeyHint::render("Enter", "Import", Color::Green));

    f.render_widget(Paragraph::new(Line::from(footer_text)), chunks[3]);
}

fn draw_import_ssh_config_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(65, 22, f.area());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Import from SSH Config ")
        .border_style(Style::default().fg(crate::ui::theme::accent_secondary()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .split(inner);

    f.render_widget(
        Paragraph::new("Enter path to SSH config file (e.g., ~/.ssh/config):"),
        chunks[0],
    );

    let input_area = chunks[1];
    f.render_widget(
        Paragraph::new("> ").style(Style::default().fg(crate::ui::theme::accent_secondary())),
        Rect::new(input_area.x, input_area.y, 2, 1),
    );
    f.render_widget(
        &app.input,
        Rect::new(
            input_area.x + 2,
            input_area.y,
            input_area.width.saturating_sub(2),
            1,
        ),
    );

    let example = r#"Parses Host entries:
Host myserver
    HostName 192.168.1.1
    User admin
    Port 2222
    IdentityFile ~/.ssh/id_rsa"#;

    f.render_widget(
        Paragraph::new(example).style(Style::default().fg(Color::DarkGray)),
        chunks[2],
    );

    let mut footer_text = Vec::new();
    footer_text.extend(HotkeyHint::render("Esc", "Cancel", Color::Red));
    footer_text.extend(HotkeyHint::render("Enter", "Import", Color::Green));

    f.render_widget(Paragraph::new(Line::from(footer_text)), chunks[3]);
}

fn draw_command_palette(f: &mut Frame, app: &mut App) {
    let area = centered_rect(60, 40, f.area());
    let inner = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Command Palette ")
        .border_style(Style::default().fg(Color::Magenta))
        .inner(area);
    f.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" Command Palette ")
            .border_style(Style::default().fg(Color::Magenta)),
        area,
    );

    f.render_widget(
        Paragraph::new("> ").style(Style::default().fg(Color::Yellow)),
        Rect::new(inner.x, inner.y, 2, 1),
    );
    f.render_widget(
        &app.input,
        Rect::new(inner.x + 2, inner.y, inner.width.saturating_sub(2), 1),
    );

    let items: Vec<ListItem> = app
        .filtered_commands
        .iter()
        .enumerate()
        .map(|(i, cmd)| {
            let style = if i == app.command_index {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else {
                Style::default()
            };
            ListItem::new(cmd.desc.clone()).style(style)
        })
        .collect();
    f.render_widget(
        List::new(items),
        Rect::new(inner.x, inner.y + 1, inner.width, inner.height - 1),
    );
}

fn draw_rename_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(40, 10, f.area());
    let block = Block::default()
        .title(" Rename ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Yellow));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.rename_selected {
        let text = if let Some(idx) = app.input.value.rfind('.') {
            if idx > 0 {
                let stem_part = &app.input.value[..idx];
                let ext_part = &app.input.value[idx..];
                Line::from(vec![
                    Span::styled(
                        stem_part,
                        Style::default()
                            .bg(crate::ui::theme::accent_primary())
                            .fg(Color::Black),
                    ),
                    Span::raw(ext_part),
                ])
            } else {
                Line::from(vec![Span::styled(
                    &app.input.value,
                    Style::default()
                        .bg(crate::ui::theme::accent_primary())
                        .fg(Color::Black),
                )])
            }
        } else {
            Line::from(vec![Span::styled(
                &app.input.value,
                Style::default()
                    .bg(crate::ui::theme::accent_primary())
                    .fg(Color::Black),
            )])
        };
        f.render_widget(Paragraph::new(text), inner);
    } else {
        f.render_widget(&app.input, inner);
    }
}

fn draw_new_folder_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(40, 10, f.area());
    let block = Block::default()
        .title(" New Folder ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Green));
    let inner = block.inner(area);
    f.render_widget(block, area);
    f.render_widget(&app.input, inner);
}

fn draw_new_file_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(40, 10, f.area());
    let block = Block::default()
        .title(" New File ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Green));
    let inner = block.inner(area);
    f.render_widget(block, area);
    f.render_widget(&app.input, inner);
}

fn draw_bulk_rename_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 20, f.area());

    let block = Block::default()
        .title(" Bulk Rename ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let label_style = Style::default().fg(Color::DarkGray);
    let input_style = Style::default().fg(THEME.fg);

    let file_count = if let AppMode::BulkRename { ref files, .. } = app.mode {
        files.len()
    } else {
        0
    };

    let mut content = Vec::new();
    content.push(Line::from(vec![Span::styled(format!("{} files selected - Enter to apply", file_count), Style::default().fg(Color::Cyan))]));
    content.push(Line::from(vec![Span::raw("")]));
    content.push(Line::from(vec![Span::styled("Pattern: ", label_style)]));
    content.push(Line::from(vec![Span::styled(&app.input.value, input_style)]));
    content.push(Line::from(vec![Span::raw("")]));
    content.push(Line::from(vec![Span::styled("Preview (first 5):", label_style)]));

    let mut preview_lines: Vec<String> = Vec::new();
    if let AppMode::BulkRename { ref files, ref pattern, ref replacement, .. } = app.mode {
        let re = regex::Regex::new(pattern);
        for (i, f) in files.iter().take(5).enumerate() {
            let name_str = f.file_name().unwrap_or_default().to_string_lossy().into_owned();
            let new_name = if let Ok(ref re) = re {
                re.replace_all(&name_str, replacement.as_str()).to_string()
            } else {
                name_str.clone()
            };
            let changed = if new_name != name_str { " → " } else { "   " };
            preview_lines.push(format!("  {} {}{}{}", i + 1, name_str, changed, new_name));
        }
        if files.len() > 5 {
            preview_lines.push(format!("  ... and {} more", files.len() - 5));
        }
    }
    for line in preview_lines {
        content.push(Line::from(vec![Span::styled(line, Style::default().fg(Color::DarkGray))]));
    }

    f.render_widget(Paragraph::new(content), inner);

    let hint_style = Style::default().fg(Color::DarkGray);
    f.render_widget(
        Paragraph::new("Esc = Cancel  Enter = Apply").style(hint_style),
        Rect::new(inner.x, inner.y + inner.height.saturating_sub(1), inner.width, 1),
    );
}

fn draw_save_as_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(50, 10, f.area());
    let block = Block::default()
        .title(" Save As ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Yellow));
    let inner = block.inner(area);
    f.render_widget(block, area);
    f.render_widget(&app.input, inner);
}

fn draw_delete_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(50, 12, f.area());

    let (title, message, item_name) = match &app.mode {
        AppMode::DeleteFile(ref path) => {
            let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            (" Delete File ".to_string(), "This file will be permanently deleted:", name)
        }
        AppMode::Delete(ref mode) if mode == "trash" => {
            (" Trash Items ".to_string(), "Selected items will be moved to trash:", "Multiple items".to_string())
        }
        _ => {
            (" Delete Items ".to_string(), "Selected items will be permanently deleted:", "Multiple items".to_string())
        }
    };

    let border_color = match &app.mode {
        AppMode::Delete(ref mode) if mode == "trash" => Color::Yellow,
        _ => Color::Red,
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Warning icon + message
    let warning_icon = Icon::Delete.get(app.icon_mode);
    let header_lines = vec![
        Line::from(vec![
            Span::styled(warning_icon, Style::default().fg(border_color).add_modifier(Modifier::BOLD)),
            Span::raw(" "),
            Span::styled(message, Style::default().fg(THEME.fg)),
        ]),
    ];
    f.render_widget(
        Paragraph::new(header_lines).alignment(Alignment::Center),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );

    // Item name (highlighted)
    f.render_widget(
        Paragraph::new(format!("'{}'", item_name))
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Rect::new(inner.x, inner.y + 3, inner.width, 1),
    );

    // Confirm prompt
    f.render_widget(
        Paragraph::new("Press [Y] to confirm or [N]/Esc to cancel")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray)),
        Rect::new(inner.x, inner.y + 5, inner.width, 1),
    );

    // Buttons
    let (mx, my) = app.mouse_pos;
    let button_y = inner.y + inner.height.saturating_sub(2);

    let no_x = inner.x + 2;
    let yes_x = inner.x + inner.width.saturating_sub(11);

    let is_hover = |bx: u16, len: u16| mx >= bx && mx < bx + len && my == button_y;

    let no_style = if is_hover(no_x, 8) {
        Style::default()
            .bg(Color::White)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let yes_style = if is_hover(yes_x, 9) {
        Style::default()
            .bg(border_color)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(border_color).add_modifier(Modifier::BOLD)
    };

    f.render_widget(
        Paragraph::new(" [ NO ] ").style(no_style),
        Rect::new(no_x, button_y, 8, 1),
    );

    f.render_widget(
        Paragraph::new(" [ YES ] ").style(yes_style),
        Rect::new(yes_x, button_y, 9, 1),
    );
}

fn draw_kill_process_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(50, 12, f.area());

    let (pid, name) = match &app.mode {
        AppMode::KillProcessConfirm(pid, name) => (*pid, name.clone()),
        _ => return,
    };

    let block = Block::default()
        .title(" Kill Process ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Red));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Message
    let message = format!("Kill process '{}' (PID {})?", name, pid);
    f.render_widget(
        Paragraph::new(message).alignment(Alignment::Center),
        Rect::new(inner.x, inner.y + 2, inner.width, 1),
    );

    f.render_widget(
        Paragraph::new("Press [Y] to confirm or [N]/Esc to cancel").alignment(Alignment::Center),
        Rect::new(inner.x, inner.y + 4, inner.width, 1),
    );

    // Buttons
    let (mx, my) = app.mouse_pos;
    let button_y = inner.y + inner.height.saturating_sub(2);

    let no_x = inner.x + 2;
    let yes_x = inner.x + inner.width.saturating_sub(11);

    let is_hover = |bx: u16, len: u16| mx >= bx && mx < bx + len && my == button_y;

    let no_style = if is_hover(no_x, 8) {
        Style::default()
            .bg(Color::White)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let yes_style = if is_hover(yes_x, 9) {
        Style::default()
            .bg(Color::Red)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    };

    f.render_widget(
        Paragraph::new(" [ NO ] ").style(no_style),
        Rect::new(no_x, button_y, 8, 1),
    );

    f.render_widget(
        Paragraph::new(" [ YES ] ").style(yes_style),
        Rect::new(yes_x, button_y, 9, 1),
    );
}

fn draw_properties_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(50, 50, f.area());

    let mut text = Vec::new();

    if let Some(fs) = app.current_file_state() {
        let target_path = fs
            .selection
            .selected
            .and_then(|idx| fs.files.get(idx))
            .unwrap_or(&fs.current_path);

        let name = target_path
            .file_name()
            .map(|n: &std::ffi::OsStr| n.to_string_lossy().to_string())
            .unwrap_or_else(|| target_path.to_string_lossy().to_string());
        let parent = target_path
            .parent()
            .map(|p: &std::path::Path| p.to_string_lossy().to_string())
            .unwrap_or_default();

        text.push(Line::from(vec![
            Span::styled(
                "Name: ",
                Style::default().fg(crate::ui::theme::accent_secondary()),
            ),
            Span::raw(name),
        ]));
        text.push(Line::from(vec![
            Span::styled(
                "Location: ",
                Style::default().fg(crate::ui::theme::accent_secondary()),
            ),
            Span::raw(parent),
        ]));
        text.push(Line::from(""));

        if let Some(meta) = fs.metadata.get(target_path) {
            let type_str = if meta.is_dir { "Folder" } else { "File" };
            text.push(Line::from(vec![
                Span::styled(
                    "Type: ",
                    Style::default().fg(crate::ui::theme::accent_secondary()),
                ),
                Span::raw(type_str),
            ]));
            text.push(Line::from(vec![
                Span::styled(
                    "Size: ",
                    Style::default().fg(crate::ui::theme::accent_secondary()),
                ),
                Span::raw(format_size(meta.size)),
            ]));
            text.push(Line::from(vec![
                Span::styled(
                    "Modified: ",
                    Style::default().fg(crate::ui::theme::accent_secondary()),
                ),
                Span::raw(format_time(meta.modified)),
            ]));
            text.push(Line::from(vec![
                Span::styled(
                    "Created: ",
                    Style::default().fg(crate::ui::theme::accent_secondary()),
                ),
                Span::raw(format_time(meta.created)),
            ]));
            text.push(Line::from(vec![
                Span::styled(
                    "Permissions: ",
                    Style::default().fg(crate::ui::theme::accent_secondary()),
                ),
                Span::raw(format_permissions(meta.permissions)),
            ]));
} else if fs.remote_session.is_none() {
                if let Some(m) = fs.metadata.get(target_path) {
                    let is_dir = m.is_dir;
                    text.push(Line::from(vec![
                        Span::styled(
                            "Type: ",
                            Style::default().fg(crate::ui::theme::accent_secondary()),
                        ),
                        Span::raw(if is_dir { "Folder" } else { "File" }),
                    ]));
                    text.push(Line::from(vec![
                        Span::styled(
                            "Size: ",
                            Style::default().fg(crate::ui::theme::accent_secondary()),
                        ),
                        Span::raw(format_size(m.size)),
                    ]));
                    text.push(Line::from(vec![
                        Span::styled(
                            "Modified: ",
                            Style::default().fg(crate::ui::theme::accent_secondary()),
                        ),
                        Span::raw(format_time(m.modified)),
                    ]));
                } else {
                    text.push(Line::from(Span::styled(
                        "No metadata available",
                        Style::default().fg(Color::DarkGray),
                    )));
                }
        } else {
            text.push(Line::from(Span::styled(
                "No metadata available (Remote)",
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    // Show cached checksums if available
    if let Some(fs) = app.current_file_state() {
        let target_path = fs
            .selection
            .selected
            .and_then(|idx| fs.files.get(idx))
            .unwrap_or(&fs.current_path);
        if let Some((md5, sha256)) = app.checksum_cache.get(target_path) {
            text.push(Line::from(""));
            if !md5.is_empty() {
                text.push(Line::from(vec![
                    Span::styled("MD5:    ", Style::default().fg(crate::ui::theme::accent_secondary())),
                    Span::raw(&md5[..std::cmp::min(md5.len(), 32)]),
                ]));
            }
            if !sha256.is_empty() {
                text.push(Line::from(vec![
                    Span::styled("SHA256: ", Style::default().fg(crate::ui::theme::accent_secondary())),
                    Span::raw(&sha256[..std::cmp::min(sha256.len(), 32)]),
                ]));
            }
        }
    }

    text.push(Line::from(""));
    text.push(Line::from(vec![
        Span::styled(" [E] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("Edit Permissions   ", Style::default().fg(Color::DarkGray)),
        Span::styled(" [C] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("Checksums   ", Style::default().fg(Color::DarkGray)),
        Span::styled(" [Esc] ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        Span::styled("Close", Style::default().fg(Color::DarkGray)),
    ]));

    let block = Block::default()
        .title(" Properties ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(crate::ui::theme::accent_primary()));
    f.render_widget(Paragraph::new(text).block(block), area);
}

fn draw_edit_permissions_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(40, 20, f.area());

    let mut text = Vec::new();
    text.push(Line::from(vec![
        Span::styled("Edit Permissions (octal):", Style::default().fg(crate::ui::theme::accent_secondary())),
    ]));
    text.push(Line::from(""));
    text.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(&app.input.value, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled("_", Style::default().fg(Color::White)),
    ]));
    text.push(Line::from(""));
    text.push(Line::from(vec![
        Span::styled("Examples: ", Style::default().fg(Color::DarkGray)),
        Span::raw("644=file rw-, 755=dir rwx, 600=private"),
    ]));

    let help = vec![
        Line::from(vec![
            Span::styled(" [Enter] ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw("Apply    "),
            Span::styled(" [Esc] ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::raw("Cancel"),
        ]),
    ];

    let block = Block::default()
        .title(" Permissions ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(crate::ui::theme::accent_primary()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(inner);

    f.render_widget(Paragraph::new(text), chunks[0]);
    f.render_widget(Paragraph::new(help).alignment(Alignment::Center), chunks[1]);
}

fn draw_settings_modal(f: &mut Frame, app: &App) {
    let area = f.area();

    let block = Block::default()
        .title_top(Line::from(vec![Span::styled(
            " SETTINGS ",
            Style::default()
                .fg(Color::Black)
                .bg(crate::ui::theme::accent_primary())
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
        .border_style(Style::default().fg(crate::ui::theme::accent_primary()))
        .style(Style::default().bg(Color::Rgb(0, 0, 0)));

    let inner = block.inner(area);

    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(20), Constraint::Min(0)])
        .split(inner);

    let sections = vec![
        ListItem::new(" 󰟜  Columns "),
        ListItem::new(" 󰓩  Tabs "),
        ListItem::new(" 󰒓  General "),
        ListItem::new(" 󰸌  Style "),
        ListItem::new(" 󰒍  Remotes "),
        ListItem::new(" 󰌌  Shortcuts "),
    ];

    let sel = match app.settings_section {
        SettingsSection::Columns => 0,
        SettingsSection::Tabs => 1,
        SettingsSection::General => 2,
        SettingsSection::Style => 3,
        SettingsSection::Remotes => 4,
        SettingsSection::Shortcuts => 5,
    };
    let items: Vec<ListItem> = sections
        .into_iter()
        .enumerate()
        .map(|(i, item)| {
            if i == sel {
                item.style(
                    Style::default()
                        .bg(crate::ui::theme::accent_primary())
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                item
            }
        })
        .collect();
    f.render_widget(
        List::new(items).block(
            Block::default()
                .borders(Borders::RIGHT)
                .border_style(Style::default().fg(Color::DarkGray)),
        ),
        chunks[0],
    );
    match app.settings_section {
        SettingsSection::Columns => draw_column_settings(f, chunks[1], app),
        SettingsSection::Tabs => draw_tab_settings(f, chunks[1], app),
        SettingsSection::General => draw_general_settings(f, chunks[1], app),
        SettingsSection::Style => draw_style_settings(f, chunks[1], app),
        SettingsSection::Remotes => draw_remote_settings(f, chunks[1], app),
        SettingsSection::Shortcuts => draw_shortcuts_settings(f, chunks[1], app),
    }
}

fn draw_shortcuts_settings(f: &mut Frame, area: Rect, _app: &App) {
    let shortcuts = vec![
        (
            "General",
            vec![
                ("Ctrl + q", "Quit Application"),
                ("Ctrl + g", "Open Settings"),
                ("Ctrl + d", "Open/Close Debug Screen"),
                ("4 (in Settings)", "Open Style Section"),
                ("Ctrl + Space", "Open Command Palette"),
                ("Ctrl + b", "Toggle Sidebar"),
                ("Ctrl + m", "Toggle Main Stage"),
                ("Ctrl + l", "Open Git View"),
                ("Ctrl + i", "Information"),
            ],
        ),
        (
            "Navigation",
            vec![
                ("↑ / ↓", "Move Selection"),
                ("Home / End", "Jump to First / Last Item"),
                ("PgUp / PgDn", "Jump by Visible Page"),
                ("Left / Right", "Change Pane / Enter/Leave Sidebar"),
                ("Enter", "Open Directory / File"),
                ("Shift + Enter", "Open Folder in New Tab"),
                ("Backspace", "Go to Parent Directory"),
                ("Alt + Left / Right", "Back / Forward in History"),
                ("~", "Go to Home Directory"),
                ("Middle Click / Space", "Expand / Edit"),
            ],
        ),
        (
            "View & Tabs",
            vec![
                ("Ctrl + p", "Toggle Split View"),
                ("Ctrl + t", "New Duplicate Tab"),
                ("Ctrl + h", "Toggle Hidden Files"),
                ("Ctrl + b", "Toggle Sidebar"),
                ("Ctrl + u / Ctrl + w", "Clear Search / Delete Search Word"),
                ("Ctrl + z / Ctrl + y", "Undo / Redo (File Operations)"),
                ("Ctrl + Shift + z", "Redo Alternative"),
                ("?", "Show this Help"),
                ("Esc / Ctrl + [", "Back / Exit Mode"),
                ("Enter/E (Style row)", "Edit color as #RRGGBB or R,G,B"),
                ("General: Reset All Settings", "Type RESET to confirm"),
            ],
        ),
        (
            "File Operations",
            vec![
                ("Ctrl + c / Ins", "Copy Selected"),
                ("Ctrl + x", "Cut Selected"),
                ("Ctrl + v", "Paste Selected"),
                ("Ctrl + a", "Select All"),
                ("F2", "Rename Selected"),
                ("Ctrl + R", "Run Selected File"),
                ("Delete", "Delete to Trash"),
                ("Alt + Enter", "Show Properties"),
            ],
        ),
        (
            "Editor",
            vec![
                ("Alt + Up/Down", "Move Line Up/Down"),
                ("Ctrl + Bksp / W", "Delete Word Backward"),
                ("Ctrl + Delete", "Delete Word Forward"),
                ("Ctrl + G", "Go to Line"),
                ("Ctrl + F", "Find in File"),
                ("Ctrl + R / F2", "Replace"),
                ("Ctrl + Z / Ctrl + Y", "Undo / Redo"),
                ("Ctrl + Shift + Z", "Redo Alternative"),
                ("Double Click", "Select Word"),
                ("Triple Click", "Select Line"),
                ("Drag Selection", "Move Text Block"),
            ],
        ),
        (
            "Terminal",
            vec![
                ("Ctrl + n", "Open Terminal Tab"),
                ("Ctrl + k", "New Terminal Window"),
            ],
        ),
    ];

    let mut rows = Vec::new();
    for (category, items) in shortcuts {
        rows.push(Row::new(vec![
            Cell::from(Span::styled(
                category,
                Style::default()
                    .fg(crate::ui::theme::accent_primary())
                    .add_modifier(Modifier::BOLD),
            )),
            Cell::from(""),
        ]));
        for (key, desc) in items {
            rows.push(Row::new(vec![
                Cell::from(Span::styled(key, Style::default().fg(Color::Yellow))),
                Cell::from(desc),
            ]));
        }
        rows.push(Row::new(vec![Cell::from(""), Cell::from("")])); // Spacer
    }

    let table = Table::new(rows, [Constraint::Length(20), Constraint::Min(0)]).block(
        Block::default()
            .title(" Keyboard Shortcuts ")
            .borders(Borders::NONE),
    );

    f.render_widget(table, area);
}

fn draw_column_settings(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);
    let titles = vec![" [Single] ", " [Split] "];
    let sel = match app.settings_target {
        SettingsTarget::SingleMode => 0,
        SettingsTarget::SplitMode => 1,
    };
    f.render_widget(
        Tabs::new(titles)
            .block(
                Block::default()
                    .borders(Borders::BOTTOM)
                    .title(" Configure Mode "),
            )
            .select(sel)
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        chunks[0],
    );
    let options = [
        (FileColumn::Size, "Size (s)"),
        (FileColumn::Modified, "Modified (m)"),
        (FileColumn::Created, "Created (c)"),
        (FileColumn::Permissions, "Permissions (p)"),
    ];
    let target = match app.settings_target {
        SettingsTarget::SingleMode => &app.single_columns,
        SettingsTarget::SplitMode => &app.split_columns,
    };
    let items: Vec<ListItem> = options
        .iter()
        .enumerate()
        .map(|(i, (col, label))| {
            let prefix = if target.contains(col) { "[x] " } else { "[ ] " };
            let mut style = Style::default().fg(THEME.fg);
            if i == app.settings_index && app.settings_section == SettingsSection::Columns {
                style = Style::default()
                    .bg(crate::ui::theme::accent_primary())
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD);
            }
            ListItem::new(format!("{}{}", prefix, label)).style(style)
        })
        .collect();
    f.render_widget(
        List::new(items).block(
            Block::default()
                .title(" Visible Columns ")
                .borders(Borders::NONE),
        ),
        chunks[1],
    );
}

fn draw_tab_settings(f: &mut Frame, area: Rect, app: &App) {
    let mut rows = Vec::new();
    let mut tab_counter = 0;

    for (p_idx, pane) in app.panes.iter().enumerate() {
        rows.push(Row::new(vec![
            Cell::from(Span::styled(
                format!("PANE {}", p_idx + 1),
                Style::default()
                    .fg(crate::ui::theme::accent_secondary())
                    .add_modifier(Modifier::BOLD),
            )),
            Cell::from(""),
            Cell::from(""),
        ]));

        for (t_idx, tab) in pane.tabs.iter().enumerate() {
            let is_selected =
                tab_counter == app.settings_index && app.settings_section == SettingsSection::Tabs;
            let mut style = Style::default().fg(THEME.fg);
            if is_selected {
                style = style
                    .bg(crate::ui::theme::accent_primary())
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD);
            }

            let is_active = t_idx == pane.active_tab_index;
            let status = if is_active {
                " [ACTIVE] "
            } else {
                "          "
            };
            let status_style = if is_active {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            };

            rows.push(Row::new(vec![
                Cell::from(format!("  Tab {}", t_idx + 1)).style(style),
                Cell::from(tab.current_path.to_string_lossy().to_string()).style(style),
                Cell::from(status).style(if is_selected { style } else { status_style }),
            ]));
            tab_counter += 1;
        }
        rows.push(Row::new(vec![
            Cell::from(""),
            Cell::from(""),
            Cell::from(""),
        ])); // Spacer
    }

    let table = Table::new(
        rows,
        [
            Constraint::Length(10),
            Constraint::Fill(1),
            Constraint::Length(12),
        ],
    )
    .header(
        Row::new(vec![" TAB ", " PATH ", " STATUS "]).style(
            Style::default()
                .fg(crate::ui::theme::accent_secondary())
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::default()
            .title(" OPEN TABS MANAGEMENT ")
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::Rgb(40, 45, 55))),
    )
    .column_spacing(2);

    f.render_widget(table, area);
}

fn draw_general_settings(f: &mut Frame, area: Rect, app: &App) {
    struct GeneralOption {
        label: &'static str,
        status: String,
        key: &'static str,
        bool_state: Option<bool>,
    }
    let options = [
        GeneralOption {
            label: "Show Hidden Files",
            status: if app.default_show_hidden {
                "ENABLED ".to_string()
            } else {
                "DISABLED".to_string()
            },
            key: "h",
            bool_state: Some(app.default_show_hidden),
        },
        GeneralOption {
            label: "Confirm Delete",
            status: if app.confirm_delete {
                "ENABLED ".to_string()
            } else {
                "DISABLED".to_string()
            },
            key: "d",
            bool_state: Some(app.confirm_delete),
        },
        GeneralOption {
            label: "Smart Date Formatting",
            status: if app.smart_date {
                "ENABLED ".to_string()
            } else {
                "DISABLED".to_string()
            },
            key: "t",
            bool_state: Some(app.smart_date),
        },
        GeneralOption {
            label: "Semantic Coloring",
            status: if app.semantic_coloring {
                "ENABLED ".to_string()
            } else {
                "DISABLED".to_string()
            },
            key: "s",
            bool_state: Some(app.semantic_coloring),
        },
        GeneralOption {
            label: "Auto Save",
            status: if app.auto_save {
                "ENABLED ".to_string()
            } else {
                "DISABLED".to_string()
            },
            key: "a",
            bool_state: Some(app.auto_save),
        },
        GeneralOption {
            label: "Preview Max Size",
            status: format!("{} MB", app.preview_max_mb),
            key: "p",
            bool_state: None,
        },
        GeneralOption {
            label: "Icon Mode",
            status: format!("{:?}", app.icon_mode),
            key: "i",
            bool_state: None,
        },
        GeneralOption {
            label: "─── Sidebar Sections ───",
            status: "".to_string(),
            key: "",
            bool_state: None,
        },
        GeneralOption {
            label: "Sidebar Folders",
            status: if app.sidebar_folders {
                "ENABLED ".to_string()
            } else {
                "DISABLED".to_string()
            },
            key: "f",
            bool_state: Some(app.sidebar_folders),
        },
        GeneralOption {
            label: "Sidebar Favorites",
            status: if app.sidebar_favorites {
                "ENABLED ".to_string()
            } else {
                "DISABLED".to_string()
            },
            key: "v",
            bool_state: Some(app.sidebar_favorites),
        },
        GeneralOption {
            label: "Sidebar Recent",
            status: if app.sidebar_recent {
                "ENABLED ".to_string()
            } else {
                "DISABLED".to_string()
            },
            key: "c",
            bool_state: Some(app.sidebar_recent),
        },
        GeneralOption {
            label: "Sidebar Storage",
            status: if app.sidebar_storage {
                "ENABLED ".to_string()
            } else {
                "DISABLED".to_string()
            },
            key: "g",
            bool_state: Some(app.sidebar_storage),
        },
        GeneralOption {
            label: "Sidebar Remotes",
            status: if app.sidebar_remotes {
                "ENABLED ".to_string()
            } else {
                "DISABLED".to_string()
            },
            key: "m",
            bool_state: Some(app.sidebar_remotes),
        },
        GeneralOption {
            label: "Reset All Settings",
            status: "CONFIRM".to_string(),
            key: "!",
            bool_state: None,
        },
    ];

    let rows: Vec<_> = options
        .iter()
        .enumerate()
        .map(|(i, opt)| {
            let is_selected =
                i == app.settings_index && app.settings_section == SettingsSection::General;
            let mut style = Style::default().fg(THEME.fg);
            let mut status_style = match opt.bool_state {
                Some(true) => Style::default().fg(Color::Green),
                Some(false) => Style::default().fg(Color::Red),
                None => Style::default().fg(Color::Cyan),
            };

            if is_selected {
                style = style
                    .bg(crate::ui::theme::accent_primary())
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD);
                status_style = status_style
                    .bg(crate::ui::theme::accent_primary())
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD);
            }

            Row::new(vec![
                Cell::from(format!("  {}", opt.label)).style(style),
                Cell::from(format!(" [ {} ] ", opt.status)).style(status_style),
                Cell::from(format!("({})", opt.key)).style(if is_selected {
                    style
                } else {
                    Style::default().fg(Color::DarkGray)
                }),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Fill(1),
            Constraint::Length(15),
            Constraint::Length(5),
        ],
    )
    .block(
        Block::default()
            .title(" SYSTEM PARAMETERS ")
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::Rgb(40, 45, 55))),
    )
    .column_spacing(2);

    f.render_widget(table, area);
}

fn draw_style_settings(f: &mut Frame, area: Rect, app: &App) {
    let style = crate::ui::theme::style_settings();
    const STYLE_PRESET_ROWS: usize = 6;
    const STYLE_COLOR_START_INDEX: usize = 1 + STYLE_PRESET_ROWS;
    let color_rows = [
        ("Accent Primary", style.accent_primary),
        ("Accent Secondary", style.accent_secondary),
        ("Selection Background", style.selection_bg),
        ("Border Active", style.border_active),
        ("Border Inactive", style.border_inactive),
        ("Header Accent", style.header_fg),
    ];

    let mut rows: Vec<Row> = Vec::new();
    let reset_selected = app.settings_index == 0 && app.settings_section == SettingsSection::Style;
    let reset_style = if reset_selected {
        Style::default()
            .bg(crate::ui::theme::accent_primary())
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    };
    rows.push(Row::new(vec![
        Cell::from("  Reset To Default Theme").style(reset_style),
        Cell::from("↺").style(reset_style),
        Cell::from("restore baseline").style(reset_style),
    ]));

    let preset_rows = [
        ("Warm", "amber + mint", Color::Yellow),
        ("Cool", "violet + ice", Color::Cyan),
        ("Forest", "moss + pine", Color::Green),
        ("Sunset", "coral + plum", Color::LightRed),
        ("Mono", "steel grayscale", Color::Gray),
        ("Legacy Red", "classic red accent", Color::Red),
    ];
    for (i, (name, desc, color)) in preset_rows.iter().enumerate() {
        let row_idx = i + 1;
        let is_selected =
            row_idx == app.settings_index && app.settings_section == SettingsSection::Style;
        let row_style = if is_selected {
            Style::default()
                .bg(crate::ui::theme::accent_primary())
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(*color)
        };
        rows.push(Row::new(vec![
            Cell::from(format!("  Preset: {}", name)).style(row_style),
            Cell::from("●").style(row_style),
            Cell::from(*desc).style(row_style),
        ]));
    }

    rows.extend(
        color_rows
            .iter()
            .enumerate()
            .map(|(i, (label, rgb))| {
                let row_idx = i + STYLE_COLOR_START_INDEX;
                let is_selected =
                    row_idx == app.settings_index && app.settings_section == SettingsSection::Style;
                let mut left_style = Style::default().fg(THEME.fg);
                let mut value_style = Style::default().fg(Color::Rgb(rgb.r, rgb.g, rgb.b));
                if is_selected {
                    left_style = left_style
                        .bg(crate::ui::theme::accent_primary())
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD);
                    value_style = value_style
                        .bg(crate::ui::theme::accent_primary())
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD);
                }
                Row::new(vec![
                    Cell::from(format!("  {}", label)).style(left_style),
                    Cell::from("■").style(value_style),
                    Cell::from(format!("rgb({}, {}, {})", rgb.r, rgb.g, rgb.b)).style(value_style),
                ])
            })
            .collect::<Vec<_>>(),
    );

    let table = Table::new(
        rows,
        [
            Constraint::Fill(1),
            Constraint::Length(3),
            Constraint::Length(20),
        ],
    )
    .block(
        Block::default()
            .title(" STYLE (Preset themes + custom colors) ")
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::Rgb(40, 45, 55))),
    )
    .column_spacing(1);

    f.render_widget(table, area);
}

fn draw_style_color_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(64, 9, f.area());

    const STYLE_COLOR_START_INDEX: usize = 7;
    let field_name = match app.settings_index.saturating_sub(STYLE_COLOR_START_INDEX) {
        0 => "Accent Primary",
        1 => "Accent Secondary",
        2 => "Selection Background",
        3 => "Border Active",
        4 => "Border Inactive",
        5 => "Header Accent",
        _ => "Accent Primary",
    };

    let color = {
        let style = crate::ui::theme::style_settings();
        match app.settings_index.saturating_sub(STYLE_COLOR_START_INDEX) {
            0 => style.accent_primary,
            1 => style.accent_secondary,
            2 => style.selection_bg,
            3 => style.border_active,
            4 => style.border_inactive,
            5 => style.header_fg,
            _ => style.accent_primary,
        }
    };

    let block = Block::default()
        .title(format!(" Edit {} ", field_name))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(crate::ui::theme::accent_primary()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines = vec![
        Line::from(vec![
            Span::raw("Current: "),
            Span::styled(
                "■",
                Style::default()
                    .fg(Color::Rgb(color.r, color.g, color.b))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("  rgb({}, {}, {})", color.r, color.g, color.b)),
        ]),
        Line::from("Input: #RRGGBB or R,G,B"),
    ];
    f.render_widget(
        Paragraph::new(lines).style(Style::default().fg(THEME.fg)),
        Rect::new(inner.x, inner.y, inner.width, 2),
    );

    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(crate::ui::theme::accent_secondary()));
    f.render_widget(
        Paragraph::new(app.input.value.as_str()).block(input_block),
        Rect::new(inner.x, inner.y + 2, inner.width, 3),
    );

    let footer = Line::from(vec![
        Span::styled(" Esc ", Style::default().fg(Color::Black).bg(Color::Red)),
        Span::raw(" cancel  "),
        Span::styled(
            " Enter ",
            Style::default().fg(Color::Black).bg(Color::Green),
        ),
        Span::raw(" apply"),
    ]);
    f.render_widget(
        Paragraph::new(footer),
        Rect::new(inner.x, inner.y + 6, inner.width, 1),
    );

    if let Some((msg, time)) = &app.last_action_msg {
        if time.elapsed().as_secs() < 5 && msg.starts_with("Invalid color for ") {
            f.render_widget(
                Paragraph::new(msg.as_str()).style(Style::default().fg(Color::Red)),
                Rect::new(inner.x, inner.y + 7, inner.width, 1),
            );
        }
    }
}

fn draw_reset_settings_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(56, 12, f.area());
    let block = Block::default()
        .title(" Reset All Settings ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Red));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let text = vec![
        Line::from("This resets global settings to defaults."),
        Line::from("Bookmarks and remotes are kept."),
        Line::from(""),
        Line::from(vec![
            Span::raw("Type "),
            Span::styled(
                "RESET",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" and press Enter."),
        ]),
    ];
    f.render_widget(
        Paragraph::new(text).style(Style::default().fg(THEME.fg)),
        Rect::new(inner.x, inner.y, inner.width, 5),
    );

    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(crate::ui::theme::accent_primary()));
    f.render_widget(
        Paragraph::new(app.input.value.as_str()).block(input_block),
        Rect::new(inner.x, inner.y + 5, inner.width, 3),
    );

    let footer = Line::from(vec![
        Span::styled(" Esc ", Style::default().fg(Color::Black).bg(Color::Red)),
        Span::raw(" cancel  "),
        Span::styled(
            " Enter ",
            Style::default().fg(Color::Black).bg(Color::Green),
        ),
        Span::raw(" apply"),
    ]);
    f.render_widget(
        Paragraph::new(footer),
        Rect::new(inner.x, inner.y + 9, inner.width, 1),
    );
}

fn draw_debug_page(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title_top(Line::from(vec![Span::styled(
            " DEBUG ",
            Style::default()
                .fg(Color::Black)
                .bg(crate::ui::theme::accent_primary())
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
        .border_style(Style::default().fg(crate::ui::theme::border_inactive()))
        .style(Style::default().bg(Color::Rgb(8, 8, 12)));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let pane_idx = app.focused_pane_index;
    let (path, filter, remote) = app
        .current_file_state()
        .map(|fs| {
            (
                fs.current_path.display().to_string(),
                fs.search_filter.clone(),
                fs.remote_session.is_some(),
            )
        })
        .unwrap_or_else(|| ("-".to_string(), "".to_string(), false));

    let lines = vec![
        Line::from(format!("view={:?} mode={:?}", app.current_view, app.mode)),
        Line::from(format!(
            "pane={} sidebar_focus={}",
            pane_idx, app.sidebar_focus
        )),
        Line::from(format!(
            "split={} sidebar={} stage={}",
            app.is_split_mode, app.show_sidebar, app.show_main_stage
        )),
        Line::from(format!("remote={} filter='{}'", remote, filter)),
        Line::from(format!(
            "path={}",
            truncate_to_width(&path, inner.width.saturating_sub(8) as usize, "...")
        )),
        Line::from("Open/Close: Ctrl+D"),
    ];
    f.render_widget(
        Paragraph::new(lines).style(Style::default().fg(Color::Rgb(190, 190, 200))),
        inner,
    );
}

fn draw_remote_settings(f: &mut Frame, area: Rect, app: &App) {
    let rows: Vec<_> = app
        .servers
        .iter()
        .enumerate()
        .map(|(i, b)| {
            let is_selected =
                i == app.settings_index && app.settings_section == SettingsSection::Remotes;
            let mut style = Style::default().fg(THEME.fg);
            if is_selected {
                style = style
                    .bg(crate::ui::theme::accent_primary())
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD);
            }

            let icon = Icon::Remote.get(app.icon_mode);
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
                .fg(crate::ui::theme::accent_secondary())
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::default()
            .title(" REMOTE SERVER BOOKMARKS ")
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::Rgb(40, 45, 55))),
    )
    .column_spacing(2);

    let text = vec![
        Line::from("Manage your remote server bookmarks here."),
        Line::from(vec![
            Span::styled("[A]", Style::default().fg(crate::ui::theme::accent_secondary()).add_modifier(Modifier::BOLD)),
            Span::raw("dd "),
            Span::styled("[E]", Style::default().fg(crate::ui::theme::accent_secondary()).add_modifier(Modifier::BOLD)),
            Span::raw("dit "),
            Span::styled("[D]", Style::default().fg(crate::ui::theme::accent_secondary()).add_modifier(Modifier::BOLD)),
            Span::raw("elete "),
            Span::styled("[I]", Style::default().fg(crate::ui::theme::accent_secondary()).add_modifier(Modifier::BOLD)),
            Span::raw("mport TOML "),
            Span::styled("[S]", Style::default().fg(crate::ui::theme::accent_secondary()).add_modifier(Modifier::BOLD)),
            Span::raw("SH Config "),
            Span::styled("[X]", Style::default().fg(crate::ui::theme::accent_secondary()).add_modifier(Modifier::BOLD)),
            Span::raw("port "),
            Span::styled("[T]", Style::default().fg(crate::ui::theme::accent_secondary()).add_modifier(Modifier::BOLD)),
            Span::raw("OML "),
            Span::styled("[Enter]", Style::default().fg(crate::ui::theme::accent_secondary()).add_modifier(Modifier::BOLD)),
            Span::raw(" Connect"),
        ]),
        Line::from(""),
    ];

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    f.render_widget(Paragraph::new(text), chunks[0]);

    if app.servers.is_empty() {
        f.render_widget(
            Paragraph::new("\n (No remote servers configured)")
                .style(Style::default().fg(Color::DarkGray)),
            chunks[1],
        );
    } else {
        f.render_widget(table, chunks[1]);
    }
}

fn draw_add_remote_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 50, f.area());
    
    let is_editing = app.open_with_index != usize::MAX && app.open_with_index < app.servers.len();
    let title = if is_editing { " Edit Remote Server " } else { " Add Remote Server " };
    let border_color = if is_editing { Color::Yellow } else { Color::Green };
    
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color));
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

    let active_idx = if let AppMode::AddRemote(idx) = app.mode {
        idx
    } else {
        0
    };

    let fields = [
        ("Name", &app.pending_server.name),
        ("Host", &app.pending_server.host),
        ("User", &app.pending_server.user),
        ("Port", &app.pending_server.port.to_string()),
        (
            "Key Path",
            &app.pending_server
                .key_path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default(),
        ),
    ];

    for (i, (label, value)) in fields.iter().enumerate() {
        let is_active = i == active_idx;
        let mut style = Style::default().fg(Color::DarkGray);
        if is_active {
            style = Style::default().fg(Color::Yellow);
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", label))
            .border_style(style);
        let field_area = chunks[i];

        if is_active {
            f.render_widget(
                Paragraph::new(app.input.value.as_str()).block(block),
                field_area,
            );
        } else {
            f.render_widget(Paragraph::new(value.as_str()).block(block), field_area);
        }
    }

    let help_text = vec![
        Line::from(vec![
            // Negative action on LEFT
            Span::styled(
                " [Esc] ",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::raw("Cancel    "),
            // Positive action on RIGHT
            Span::styled(
                " [Tab/Enter] ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Next Field"),
        ]),
        Line::from("On the last field, [Enter] will save the bookmark."),
    ];
    f.render_widget(Paragraph::new(help_text), chunks[5]);
}

fn draw_highlight_modal(f: &mut Frame, _app: &App) {
    // Actually let's use absolute sizing for palette
    let area = Rect::new(
        (f.area().width.saturating_sub(34)) / 2,
        (f.area().height.saturating_sub(5)) / 2,
        34,
        5,
    );

    let block = Block::default()
        .title(" Highlight ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(crate::ui::theme::accent_primary()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let colors = [
        (1, " R ", Color::Red),
        (2, " G ", Color::Green),
        (3, " Y ", Color::Yellow),
        (4, " B ", Color::Blue),
        (5, " M ", Color::Magenta),
        (6, " C ", Color::Cyan),
        (0, " X ", Color::Reset),
    ];

    let mut spans = Vec::new();
    for (i, (code, label, color)) in colors.iter().enumerate() {
        let style = if *code == 0 {
            Style::default().bg(Color::DarkGray).fg(Color::White)
        } else {
            Style::default().bg(*color).fg(Color::Black)
        };
        spans.push(Span::styled(*label, style));
        if i < colors.len() - 1 {
            spans.push(Span::raw(" "));
        }
    }

    f.render_widget(
        Paragraph::new(Line::from(spans)).alignment(ratatui::layout::Alignment::Center),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );
    f.render_widget(
        Paragraph::new("1   2   3   4   5   6   0")
            .alignment(ratatui::layout::Alignment::Center)
            .style(Style::default().fg(Color::DarkGray)),
        Rect::new(inner.x, inner.y + 2, inner.width, 1),
    );
}

fn draw_trash_page(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title_top(Line::from(vec![Span::styled(
            " TRASH ",
            Style::default()
                .fg(Color::Black)
                .bg(crate::ui::theme::accent_primary())
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
        .border_style(Style::default().fg(crate::ui::theme::border_inactive()))
        .style(Style::default().bg(Color::Rgb(8, 8, 12)));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let items = match trash::os_limited::list() {
        Ok(items) => items,
        Err(e) => {
            let lines = vec![
                Line::from("Failed to list trash items").style(Style::default().fg(Color::Red)),
                Line::from(format!("Error: {}", e)).style(Style::default().fg(Color::DarkGray)),
            ];
            f.render_widget(Paragraph::new(lines), inner);
            return;
        }
    };

    if items.is_empty() {
        let lines = vec![
            Line::from("Trash is empty").style(Style::default().fg(Color::DarkGray)),
        ];
        f.render_widget(Paragraph::new(lines), inner);
        return;
    }

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(vec![
        Span::styled(" NAME ", Style::default().fg(crate::ui::theme::accent_secondary()).add_modifier(Modifier::BOLD)),
        Span::styled(" ORIGINAL LOCATION ", Style::default().fg(crate::ui::theme::accent_secondary()).add_modifier(Modifier::BOLD)),
        Span::styled(" DELETED ", Style::default().fg(crate::ui::theme::accent_secondary()).add_modifier(Modifier::BOLD)),
    ]));

    for item in &items {
        let name = item.name.to_string_lossy().to_string();
        let orig = item.original_parent.to_string_lossy().to_string();
        let time = if item.time_deleted >= 0 {
            let dt = chrono::DateTime::from_timestamp(item.time_deleted, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "unknown".to_string());
            dt
        } else {
            "unknown".to_string()
        };
        lines.push(Line::from(vec![
            Span::raw(format!(" {} ", truncate_to_width(&name, 30, "..."))),
            Span::raw(format!(" {} ", truncate_to_width(&orig, 30, "..."))),
            Span::raw(format!(" {} ", time)),
        ]));
    }

    f.render_widget(
        Paragraph::new(lines).style(Style::default().fg(Color::Rgb(190, 190, 200))),
        inner,
    );
}

fn draw_disk_usage_page(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title_top(Line::from(vec![Span::styled(
            " DISK USAGE ",
            Style::default()
                .fg(Color::Black)
                .bg(crate::ui::theme::accent_secondary())
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
        .border_style(Style::default().fg(crate::ui::theme::border_inactive()))
        .style(Style::default().bg(Color::Rgb(8, 8, 12)));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let scan_path = app.current_file_state()
        .map(|fs| fs.current_path.clone())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    // Simple disk usage: top-level items with sizes
    let mut entries: Vec<(PathBuf, u64)> = Vec::new();
    if let Ok(reader) = std::fs::read_dir(&scan_path) {
        for entry in reader.flatten() {
            let path = entry.path();
            let size = if path.is_dir() {
                crate::modules::files::folder_size(&path)
            } else {
                std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0)
            };
            entries.push((path, size));
        }
    }

    entries.sort_by(|a, b| b.1.cmp(&a.1));

    if entries.is_empty() {
        let lines = vec![
            Line::from("No entries found").style(Style::default().fg(Color::DarkGray)),
        ];
        f.render_widget(Paragraph::new(lines), inner);
        return;
    }

    let total: u64 = entries.iter().map(|(_, s)| s).sum();

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(vec![
        Span::styled(format!(" {} ", scan_path.display()), Style::default().fg(crate::ui::theme::accent_primary()).add_modifier(Modifier::BOLD)),
        Span::styled(format!(" Total: {} ", format_size(total)), Style::default().fg(crate::ui::theme::accent_secondary()).add_modifier(Modifier::BOLD)),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(" NAME ", Style::default().fg(crate::ui::theme::accent_secondary()).add_modifier(Modifier::BOLD)),
        Span::styled(" SIZE ", Style::default().fg(crate::ui::theme::accent_secondary()).add_modifier(Modifier::BOLD)),
        Span::styled(" % ", Style::default().fg(crate::ui::theme::accent_secondary()).add_modifier(Modifier::BOLD)),
        Span::styled(" BAR ", Style::default().fg(crate::ui::theme::accent_secondary()).add_modifier(Modifier::BOLD)),
    ]));

    let bar_width = (inner.width as usize).saturating_sub(50).max(10);
    for (path, size) in &entries {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("..");
        let pct = if total > 0 { (*size as f64 / total as f64) * 100.0 } else { 0.0 };
        let filled = if total > 0 { ((*size as f64 / total as f64) * bar_width as f64) as usize } else { 0 };
        let bar = format!("{:=<filled$}{: <width$}", "", "", filled = filled, width = bar_width - filled);
        lines.push(Line::from(vec![
            Span::raw(format!(" {} ", truncate_to_width(name, 20, "..."))),
            Span::raw(format!(" {:>8} ", format_size(*size))),
            Span::raw(format!(" {:>5.1}% ", pct)),
            Span::styled(bar, Style::default().fg(crate::ui::theme::accent_primary())),
        ]));
    }

    f.render_widget(
        Paragraph::new(lines).style(Style::default().fg(Color::Rgb(190, 190, 200))),
        inner,
    );
}

fn format_modified_time(time: SystemTime, smart: bool) -> String {
    use chrono::{DateTime, Local};
    let dt: DateTime<Local> = time.into();
    let now = Local::now();

    if smart {
        let duration = now.signed_duration_since(dt);
        let days = duration.num_days();
        if days == 0 {
            if duration.num_hours() == 0 {
                if duration.num_minutes() == 0 {
                    "just now".to_string()
                } else {
                    format!("{}m ago", duration.num_minutes())
                }
            } else {
                format!("{}h ago", duration.num_hours())
            }
        } else if days == 1 {
            "yesterday".to_string()
        } else if days < 7 {
            format!("{}d ago", days)
        } else if days < 30 {
            format!("{}w ago", days / 7)
        } else if days < 365 {
            format!("{}mo ago", days / 30)
        } else {
            format!("{}y ago", days / 365)
        }
    } else if dt.date_naive() == now.date_naive() {
        dt.format("%H:%M:%S").to_string()
    } else {
        dt.format("%Y-%m-%d").to_string()
    }
}

fn draw_drag_ghost(f: &mut Frame, app: &App) {
    if let Some(path) = &app.drag_source {
        let (col, row) = app.mouse_pos;
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        // Truncate name if too long
        let max_len = 20;
        let display_name = if name.len() > max_len {
            format!("{}...", &name[..max_len])
        } else {
            name.to_string()
        };

        let text = format!(" {} ", display_name);
        let width = text.len() as u16;

        // Draw slightly offset from cursor
        let x = col
            .saturating_add(2)
            .min(f.area().width.saturating_sub(width));
        let y = row.saturating_add(1).min(f.area().height.saturating_sub(1));

        let area = Rect::new(x, y, width, 1);

        f.render_widget(
            Paragraph::new(Span::styled(
                text,
                Style::default()
                    .bg(crate::ui::theme::accent_primary())
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            )),
            area,
        );
    }
}
