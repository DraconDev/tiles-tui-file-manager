#![allow(clippy::needless_borrow, clippy::collapsible_match)]

use std::path::PathBuf;
use crate::events::settings_handlers::{cycle_preview_max_mb, open_style_color_input, style_preset_for_index, handle_style_color_input_keys, handle_reset_settings_confirm_keys};
use crate::events::editor_modals::{handle_editor_replace_keys, handle_editor_search_keys, handle_editor_goto_keys};
use crate::events::modal_mouse::handle_modal_mouse;

use crate::app::{App, AppEvent, AppMode, ContextMenuAction, ContextMenuTarget, CurrentView, SettingsSection};
use crate::state::IconMode;
use dracon_terminal_engine::contracts::{
    InputEvent as Event, KeyCode, KeyModifiers,
};
use serde::Deserialize;
use tokio::sync::mpsc;

const STYLE_PRESET_COUNT: usize = 14;
const STYLE_COLOR_FIELD_COUNT: usize = 6;
pub const STYLE_COLOR_START_INDEX: usize = 1 + STYLE_PRESET_COUNT;
const STYLE_MAX_INDEX: usize = STYLE_COLOR_START_INDEX + STYLE_COLOR_FIELD_COUNT - 1;

pub fn handle_modal_events(evt: &Event, app: &mut App, event_tx: &mpsc::Sender<AppEvent>) -> bool {
    match evt {
        Event::Key(key) => handle_modal_keys(key, app, event_tx, evt),
        Event::Mouse(me) => handle_modal_mouse(me, app, event_tx),
        _ => false,
    }
}

fn handle_modal_keys(
    key: &dracon_terminal_engine::contracts::KeyEvent,
    app: &mut App,
    event_tx: &mpsc::Sender<AppEvent>,
    evt: &Event,
) -> bool {
    let mode = app.core.mode.clone();
    match mode {
        AppMode::ContextMenu {
            ref actions,
            ref target,
            selected_index,
            ..
        } => handle_context_menu_keys(key, app, event_tx, actions, target, selected_index),
        AppMode::DragDropMenu {
            ref sources,
            ref target,
        } => handle_drag_drop_keys(key, app, event_tx, sources, target),
        AppMode::SignalSelect { pid, name, selected_index } => {
            handle_signal_select_keys(key, app, event_tx, pid, &name, selected_index)
        }
        AppMode::EditorReplace => handle_editor_replace_keys(key, app, event_tx, evt),
        AppMode::EditorSearch => handle_editor_search_keys(key, app, event_tx, evt),
        AppMode::EditorGoToLine => handle_editor_goto_keys(key, app, event_tx, evt),
        AppMode::CommandPalette => handle_command_palette_keys(key, app, event_tx, evt),
        AppMode::AddRemote(idx) => handle_add_remote_keys(key, app, event_tx, idx, evt),
        AppMode::ImportServers => handle_import_servers_keys(key, app, event_tx, evt),
        AppMode::Highlight => handle_highlight_keys(key, app),
        AppMode::StyleColorInput => handle_style_color_input_keys(key, app),
        AppMode::ResetSettingsConfirm => handle_reset_settings_confirm_keys(key, app),
        AppMode::NewFile
        | AppMode::NewFolder
        | AppMode::Rename
        | AppMode::Delete(_)
        | AppMode::DeleteFile(_)
        | AppMode::BulkRename { .. } => handle_input_modals_keys(key, app, event_tx),
        AppMode::PathInput => handle_path_input_keys(key, app, event_tx),
        AppMode::SaveAs(_) => handle_save_as_keys(key, app, event_tx),
        AppMode::Header(idx) => handle_header_keys(key, app, event_tx, idx),
        AppMode::Hotkeys => {
            if let KeyCode::Esc | KeyCode::Enter | KeyCode::F(1) = key.code {
                app.core.mode = app.core.previous_mode.clone();
                true
            } else {
                true
            }
        }
        AppMode::Settings => handle_settings_keys(key, app, event_tx),
        AppMode::Properties => handle_properties_keys(key, app),
        AppMode::Search => handle_search_keys(key, app, event_tx),
        AppMode::OpenWith(path) => {
            match key.code {
                KeyCode::Esc => {
                    app.core.mode = AppMode::Normal;
                    true
                }
                KeyCode::Enter => {
                    if !app.core.input.value.is_empty() {
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::SpawnDetached {
                            cmd: app.core.input.value.clone(),
                            args: vec![path.to_string_lossy().to_string()],
                        });
                    }
                    app.core.mode = AppMode::Normal;
                    app.core.input.clear();
                    true
                }
                _ => {
                    let res = app.core.input.handle_event(&dracon_terminal_engine::input::mapping::to_runtime_event(evt));
                    if app.core.input.value.is_empty() && res {
                        app.core.mode = AppMode::Normal;
                    }
                    res
                }
            }
        }
        _ => false,
    }
}

fn handle_search_keys(
    key: &dracon_terminal_engine::contracts::KeyEvent,
    app: &mut App,
    event_tx: &mpsc::Sender<AppEvent>,
) -> bool {
    match key.code {
        KeyCode::Esc => {
            if let Some(fs) = app.current_file_state_mut() {
                fs.nav.search_filter.clear();
                fs.nav.search_generation += 1;
                let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(app.focused_pane_index));
            }
            app.core.mode = AppMode::Normal;
            app.core.input.clear();
            app.set_input_shield(150);
            true
        }
        KeyCode::Enter => {
            let query = app.core.input.value.clone();
            if !query.is_empty() {
                if let Some(fs) = app.current_file_state_mut() {
                    fs.nav.search_filter = query;
                    fs.nav.search_generation += 1;
                }
                let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(app.focused_pane_index));
            }
            app.core.mode = AppMode::Normal;
            app.core.input.clear();
            app.set_input_shield(150);
            true
        }
        _ => {
            // Live Update
            let handled = app.core.input
                .handle_event(&dracon_terminal_engine::input::mapping::to_runtime_event(&Event::Key(*key)));
            if handled {
                let filter = app.core.input.value.clone();
                if let Some(fs) = app.current_file_state_mut() {
                    fs.nav.search_filter = filter;
                }
                let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(app.focused_pane_index));
            }
            handled
        }
    }
}

fn handle_path_input_keys(
    key: &dracon_terminal_engine::contracts::KeyEvent,
    app: &mut App,
    event_tx: &mpsc::Sender<AppEvent>,
) -> bool {
    match key.code {
        KeyCode::Esc => {
            app.core.mode = AppMode::Normal;
            app.core.input.clear();
            app.core.input.style = ratatui::style::Style::default().fg(ratatui::style::Color::White);
            app.core.input.cursor_style = ratatui::style::Style::default()
                .bg(ratatui::style::Color::White)
                .fg(ratatui::style::Color::Black);
            app.set_input_shield(150);
            true
        }
        KeyCode::Enter => {
            match crate::nav_helpers::submit_path_input(app, event_tx) {
                Ok(()) => {
                    app.core.mode = AppMode::Normal;
                    app.core.input.clear();
                    app.core.input.style =
                        ratatui::style::Style::default().fg(ratatui::style::Color::White);
                    app.core.input.cursor_style = ratatui::style::Style::default()
                        .bg(ratatui::style::Color::White)
                        .fg(ratatui::style::Color::Black);
                }
                Err(err) => {
                    app.output.last_action_msg = Some((err, std::time::Instant::now()));
                }
            }
            true
        }
        KeyCode::Char('c') | KeyCode::Char('C')
            if key.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            match crate::clipboard::copy_text_to_clipboard(&app.core.input.value) {
                Ok(()) => {
                    app.output.last_action_msg = Some((
                        "Copied current path to clipboard".to_string(),
                        std::time::Instant::now(),
                    ));
                }
                Err(err) => {
                    app.output.last_action_msg = Some((
                        format!("Clipboard failed: {}", err),
                        std::time::Instant::now(),
                    ));
                }
            }
            true
        }
        _ => app.core.input
            .handle_event(&dracon_terminal_engine::input::mapping::to_runtime_event(&Event::Key(*key))),
    }
}

fn handle_save_as_keys(
    key: &dracon_terminal_engine::contracts::KeyEvent,
    app: &mut App,
    event_tx: &mpsc::Sender<AppEvent>,
) -> bool {
    match key.code {
        KeyCode::Esc => {
            app.core.mode = AppMode::Normal;
            app.core.input.clear();
            true
        }
        KeyCode::Enter => {
            let input = app.core.input.value.trim().to_string();
            if input.is_empty() {
                app.output.last_action_msg = Some(("Path is empty".to_string(), std::time::Instant::now()));
                return true;
            }
            if let AppMode::SaveAs(original_path) = app.core.mode.clone() {
                let target = if input.starts_with('/') {
                    PathBuf::from(&input)
                } else if let Some(parent) = original_path.parent() {
                    parent.join(&input)
                } else {
                    PathBuf::from(&input)
                };
                let content = if let Some(pane) = app.panes.get(app.focused_pane_index) {
                    pane.current_state().and_then(|fs| {
                        fs.view.preview.as_ref().and_then(|p| {
                            p.editor.as_ref().map(|e| e.get_content())
                        })
                    })
                } else {
                    None
                };
                if let Some(content) = content {
                    if let Some(pane) = app.panes.get_mut(app.focused_pane_index) {
                        if let Some(fs) = pane.current_state_mut() {
                            if let Some(preview) = &mut fs.view.preview {
                                if preview.path == *original_path {
                                    preview.path = target.clone();
                                }
                            }
                        }
                    }
                    if let Some(preview) = &mut app.editor_global.editor_state {
                        if preview.path == *original_path {
                            preview.path = target.clone();
                        }
                    }
                    let _ = crate::app::try_send_event(&event_tx, AppEvent::SaveFile(target.clone(), content));
                    app.output.last_action_msg = Some((
                        format!("Saved as: {}", target.file_name().unwrap_or_default().to_string_lossy()),
                        std::time::Instant::now(),
                    ));
                } else {
                    app.output.last_action_msg = Some(("No content to save".to_string(), std::time::Instant::now()));
                }
            }
            app.core.mode = AppMode::Normal;
            app.core.input.clear();
            true
        }
        _ => app.core.input
            .handle_event(&dracon_terminal_engine::input::mapping::to_runtime_event(&Event::Key(*key))),
    }
}

fn handle_properties_keys(key: &dracon_terminal_engine::contracts::KeyEvent, app: &mut App) -> bool {
    match key.code {
        KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => {
            app.core.mode = AppMode::Normal;
            true
        }
        _ => true,
    }
}

fn handle_context_menu_keys(
    key: &dracon_terminal_engine::contracts::KeyEvent,
    app: &mut App,
    event_tx: &mpsc::Sender<AppEvent>,
    actions: &[ContextMenuAction],
    target: &ContextMenuTarget,
    selected_index: Option<usize>,
) -> bool {
    match key.code {
        KeyCode::Esc => {
            app.core.mode = AppMode::Normal;
            true
        }
        KeyCode::Up => {
            let mut new_idx = match selected_index {
                Some(idx) => {
                    if idx > 0 {
                        idx - 1
                    } else {
                        actions.len().saturating_sub(1)
                    }
                }
                None => actions.len().saturating_sub(1),
            };
            if let Some(ContextMenuAction::Separator) = actions.get(new_idx) {
                new_idx = new_idx.saturating_sub(1);
            }
            if let AppMode::ContextMenu {
                selected_index: ref mut si,
                ..
            } = app.core.mode
            {
                *si = Some(new_idx);
            }
            true
        }
        KeyCode::Down => {
            let mut new_idx = match selected_index {
                Some(idx) => {
                    if idx < actions.len().saturating_sub(1) {
                        idx + 1
                    } else {
                        0
                    }
                }
                None => 0,
            };
            if let Some(ContextMenuAction::Separator) = actions.get(new_idx) {
                if new_idx < actions.len().saturating_sub(1) {
                    new_idx += 1;
                }
            }
            if let AppMode::ContextMenu {
                selected_index: ref mut si,
                ..
            } = app.core.mode
            {
                *si = Some(new_idx);
            }
            true
        }
        KeyCode::Enter => {
            if let Some(idx) = selected_index {
                if let Some(action) = actions.get(idx) {
                    if *action != ContextMenuAction::Separator {
                        let action = action.clone();
                        let target = target.clone();
                        let prev_mode = app.core.mode.clone();
                        crate::event_helpers::handle_context_menu_action(
                            &action,
                            &target,
                            app,
                            event_tx.clone(),
                        );
                        if matches!(prev_mode, AppMode::ContextMenu { .. })
                            && !matches!(app.core.mode, AppMode::NewFile | AppMode::NewFolder | AppMode::Rename | AppMode::Delete(_) | AppMode::DeleteFile(_))
                            {
                                app.core.mode = AppMode::Normal;
                            }
                    }
                }
            }
            true
        }
        _ => true,
    }
}

const SIGNALS: [(i32, &str); 6] = [
    (1, "SIGHUP"),
    (2, "SIGINT"),
    (9, "SIGKILL"),
    (15, "SIGTERM"),
    (18, "SIGCONT"),
    (19, "SIGSTOP"),
];

fn handle_signal_select_keys(
    key: &dracon_terminal_engine::contracts::KeyEvent,
    app: &mut App,
    _event_tx: &mpsc::Sender<AppEvent>,
    pid: u32,
    name: &str,
    selected_index: usize,
) -> bool {
    match key.code {
        KeyCode::Esc => {
            app.core.mode = AppMode::Normal;
            true
        }
        KeyCode::Up => {
            let new_idx = selected_index.saturating_sub(1);
            app.core.mode = AppMode::SignalSelect { pid, name: name.to_string(), selected_index: new_idx };
            true
        }
        KeyCode::Down => {
            let new_idx = (selected_index + 1).min(SIGNALS.len() - 1);
            app.core.mode = AppMode::SignalSelect { pid, name: name.to_string(), selected_index: new_idx };
            true
        }
        KeyCode::Enter => {
            if let Some((sig, _)) = SIGNALS.get(selected_index) {
                let _ = crate::modules::system::SystemModule::kill_process_with_signal(pid, *sig);
            }
            app.core.mode = AppMode::Normal;
            true
        }
        _ => true,
    }
}

fn handle_drag_drop_keys(
    key: &dracon_terminal_engine::contracts::KeyEvent,
    app: &mut App,
    event_tx: &mpsc::Sender<AppEvent>,
    sources: &[std::path::PathBuf],
    target: &std::path::Path,
) -> bool {
    match key.code {
        KeyCode::Char('c') | KeyCode::Char('C') => {
            for source in sources {
                let dest = target.join(
                    source
                        .file_name()
                        .unwrap_or_else(|| std::ffi::OsStr::new("root")),
                );
                let _ = crate::app::try_send_event(&event_tx, AppEvent::Copy(source.clone(), dest));
            }
            app.core.mode = AppMode::Normal;
            true
        }
        KeyCode::Char('m') | KeyCode::Char('M') => {
            for source in sources {
                let dest = target.join(
                    source
                        .file_name()
                        .unwrap_or_else(|| std::ffi::OsStr::new("root")),
                );
                let _ = crate::app::try_send_event(&event_tx, AppEvent::Rename(source.clone(), dest));
            }
            if let Some(fs) = app.current_file_state_mut() {
                fs.list.selection.clear_multi();
                fs.list.selection.anchor = None;
            }
            app.core.mode = AppMode::Normal;
            true
        }
        KeyCode::Char('l') | KeyCode::Char('L') => {
            for source in sources {
                let dest = target.join(
                    source
                        .file_name()
                        .unwrap_or_else(|| std::ffi::OsStr::new("root")),
                );
                let _ = crate::app::try_send_event(&event_tx, AppEvent::Symlink(source.clone(), dest));
            }
            app.core.mode = AppMode::Normal;
            true
        }
        KeyCode::Esc => {
            app.core.mode = AppMode::Normal;
            true
        }
        _ => true,
    }
}

fn handle_command_palette_keys(
    _key: &dracon_terminal_engine::contracts::KeyEvent,
    app: &mut App,
    event_tx: &mpsc::Sender<AppEvent>,
    evt: &Event,
) -> bool {
    match _key.code {
        KeyCode::Esc => {
            app.core.mode = AppMode::Normal;
            true
        }
        KeyCode::Enter => {
            if let Some(cmd) = app.nav.filtered_commands.get(app.nav.command_index).cloned() {
                crate::event_helpers::execute_command(cmd.action, app, event_tx.clone());
            }
            app.core.mode = AppMode::Normal;
            app.core.input.clear();
            true
        }
        _ => {
            let handled = app.core.input
                .handle_event(&dracon_terminal_engine::input::mapping::to_runtime_event(evt));
            if handled {
                crate::event_helpers::update_commands(app);
            }
            handled
        }
    }
}

fn handle_add_remote_keys(
    _key: &dracon_terminal_engine::contracts::KeyEvent,
    app: &mut App,
    _event_tx: &mpsc::Sender<AppEvent>,
    idx: usize,
    evt: &Event,
) -> bool {
    match _key.code {
        KeyCode::Esc => {
            app.core.mode = AppMode::Normal;
            app.core.input.clear();
            true
        }
        KeyCode::Tab | KeyCode::Enter => {
            let val = app.core.input.value.clone();
            match idx {
                0 => app.remote.pending_remote.name = val,
                1 => app.remote.pending_remote.host = val,
                2 => app.remote.pending_remote.user = val,
                3 => app.remote.pending_remote.port = val.parse().unwrap_or(22),
                4 => {
                    app.remote.pending_remote.key_path = if val.is_empty() {
                        None
                    } else {
                        Some(std::path::PathBuf::from(val))
                    }
                }
                _ => {}
            }
            if idx < 4 {
                app.core.mode = AppMode::AddRemote(idx + 1);
                app.core.input.set_value(String::new());
            } else {
                app.remote.remote_bookmarks.push(app.remote.pending_remote.clone());
                crate::config::save_state_quiet(app);
                app.core.mode = AppMode::Normal;
                app.core.input.clear();
            }
            true
        }
        _ => app.core.input
            .handle_event(&dracon_terminal_engine::input::mapping::to_runtime_event(evt)),
    }
}

#[derive(Deserialize)]
struct ImportServersToml {
    servers: Vec<ImportServerEntry>,
}

#[derive(Deserialize)]
struct ImportServerEntry {
    name: String,
    host: String,
    user: String,
    #[serde(default = "default_ssh_port")]
    port: u16,
    #[serde(default)]
    key_path: Option<std::path::PathBuf>,
}

const fn default_ssh_port() -> u16 {
    22
}

fn handle_import_servers_keys(
    key: &dracon_terminal_engine::contracts::KeyEvent,
    app: &mut App,
    _event_tx: &mpsc::Sender<AppEvent>,
    evt: &Event,
) -> bool {
    match key.code {
        KeyCode::Esc => {
            app.core.mode = AppMode::Normal;
            app.core.input.clear();
            true
        }
        KeyCode::Enter => {
            let path = app.core.input.value.trim().to_string();
            if path.is_empty() {
                app.output.last_action_msg = Some((
                    "Import path is empty".to_string(),
                    std::time::Instant::now(),
                ));
                return true;
            }

            let parsed = std::fs::read_to_string(&path)
                .map_err(|e| format!("Failed reading {}: {}", path, e))
                .and_then(|content| {
                    toml::from_str::<ImportServersToml>(&content)
                        .map_err(|e| format!("Invalid TOML: {}", e))
                });

            match parsed {
                Ok(data) => {
                    let mut imported = 0usize;
                    for s in data.servers {
                        let candidate = crate::state::RemoteBookmark {
                            name: s.name,
                            host: s.host,
                            user: s.user,
                            port: s.port,
                            last_path: std::path::PathBuf::from("/"),
                            key_path: s.key_path,
                        };
                        let exists = app.remote.remote_bookmarks.iter().any(|b| {
                            b.name == candidate.name
                                && b.host == candidate.host
                                && b.user == candidate.user
                                && b.port == candidate.port
                        });
                        if !exists {
                            app.remote.remote_bookmarks.push(candidate);
                            imported += 1;
                        }
                    }
                    crate::config::save_state_quiet(app);
                    app.output.last_action_msg = Some((
                        format!("Imported {} server(s)", imported),
                        std::time::Instant::now(),
                    ));
                    app.core.mode = AppMode::Normal;
                    app.core.input.clear();
                }
                Err(msg) => {
                    app.output.last_action_msg = Some((msg, std::time::Instant::now()));
                }
            }
            true
        }
        _ => app.core.input
            .handle_event(&dracon_terminal_engine::input::mapping::to_runtime_event(evt)),
    }
}

fn handle_highlight_keys(key: &dracon_terminal_engine::contracts::KeyEvent, app: &mut App) -> bool {
    if let KeyCode::Char(c) = key.code {
        if let Some(digit) = c.to_digit(10) {
            if digit <= 6 {
                let color = if digit == 0 { None } else { Some(digit as u8) };
                if let Some(fs) = app.current_file_state() {
                    let mut paths = Vec::new();
                    if !fs.list.selection.is_empty() {
                        for &idx in fs.list.selection.multi_selected_indices() {
                            if let Some(p) = fs.list.files.get(idx) {
                                paths.push(p.clone());
                            }
                        }
                    } else if let Some(idx) = fs.list.selection.selected {
                        if let Some(p) = fs.list.files.get(idx) {
                            paths.push(p.clone());
                        }
                    }
                    for p in paths {
                        if let Some(col) = color {
                            app.selection.path_colors.insert(p, col);
                        } else {
                            app.selection.path_colors.remove(&p);
                        }
                    }
                    crate::config::save_state_quiet(app);
                }
                app.core.mode = AppMode::Normal;
                true
            } else {
                false
            }
        } else {
            false
        }
    } else if key.code == KeyCode::Esc {
        app.core.mode = AppMode::Normal;
        true
    } else {
        false
    }
}

fn handle_input_modals_keys(
    key: &dracon_terminal_engine::contracts::KeyEvent,
    app: &mut App,
    event_tx: &mpsc::Sender<AppEvent>,
) -> bool {
    match key.code {
        KeyCode::Esc => {
            app.core.mode = AppMode::Normal;
            app.core.input.clear();
            app.selection.rename_selected = false;
            true
        }
        KeyCode::Enter => {
            let input = app.core.input.value.clone();
            if let AppMode::DeleteFile(ref path) = app.core.mode {
                if input.trim().to_lowercase() == "y" || !app.settings.confirm_delete {
                    let _ = crate::app::try_send_event(&event_tx, AppEvent::Delete(path.clone()));
                    app.core.mode = AppMode::Normal;
                } else {
                    app.core.mode = AppMode::Normal;
                }
                app.core.input.clear();
                return true;
            }
            let mode = app.core.mode.clone();
            if let Some(fs) = app.current_file_state() {
                let path = fs.nav.current_path.join(&input);
                match mode {
                    AppMode::NewFile => {
                        let pane_idx = app.focused_pane_index;
                        let path_clone = path.clone();
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::CreateFile(path));
                        app.core.current_view = CurrentView::Editor;
                        app.core.mode = AppMode::Normal;
                        app.core.input.clear();
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::PreviewRequested(pane_idx, path_clone));
                        return true;
                    }
                    AppMode::NewFolder => {
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::CreateFolder(path));
                    }
                    AppMode::Rename => {
                        if let Some(idx) = fs.list.selection.selected {
                            if let Some(old) = fs.list.files.get(idx) {
                                if let Some(parent) = old.parent() {
                                    let _ = crate::app::try_send_event(&event_tx, AppEvent::Rename(
                                        old.clone(),
                                        parent.join(&input),
                                    ));
                                } else {
                                    let _ = crate::app::try_send_event(&event_tx, AppEvent::StatusMsg(
                                        "Cannot rename root path".to_string(),
                                    ));
                                }
                            }
                        }
                    }
                    AppMode::Delete(ref mode) => {
                        if input.trim().to_lowercase() == "y" || input.is_empty() {
                            // Collect paths to delete
                            let mut paths = Vec::new();
                            if !fs.list.selection.is_empty() {
                                for &idx in fs.list.selection.multi_selected_indices() {
                                    if let Some(p) = fs.list.files.get(idx) {
                                        paths.push(p.clone());
                                    }
                                }
                            } else if let Some(idx) = fs.list.selection.selected {
                                if let Some(p) = fs.list.files.get(idx) {
                                    paths.push(p.clone());
                                }
                            }
                            if mode == "trash" {
                                for p in paths {
                                    let _ = crate::app::try_send_event(&event_tx, AppEvent::TrashFile(p));
                                }
                            } else {
                                for p in paths {
                                    let _ = crate::app::try_send_event(&event_tx, AppEvent::Delete(p));
                                }
                            }
                        }
                    }
                    AppMode::BulkRename { ref files, ref replacement, .. } => {
                        if !input.is_empty() {
                            let re = regex::Regex::new(&input);
                            if let Ok(re) = re {
                                for f in files {
                                    if let Some(parent) = f.parent() {
                                        let old_name = f.file_name().unwrap_or_default().to_string_lossy();
                                        let new_name = re.replace_all(&old_name, replacement.as_str()).to_string();
                                        if new_name != old_name {
                                            let _ = crate::app::try_send_event(&event_tx, AppEvent::Rename(f.clone(), parent.join(&new_name)));
                                        }
                                    }
                                }
                                let _ = crate::app::try_send_event(&event_tx, AppEvent::StatusMsg(format!(
                                    "Bulk renamed {} files", files.len()
                                )));
                            } else {
                                let _ = crate::app::try_send_event(&event_tx, AppEvent::StatusMsg(
                                    "Invalid regex pattern".to_string()
                                ));
                            }
                        }
                    }
                    _ => {}
                }
            }
            app.core.mode = AppMode::Normal;
            app.core.input.clear();
            true
        }
        _ => app.core.input
            .handle_event(&dracon_terminal_engine::input::mapping::to_runtime_event(&Event::Key(*key))),
    }
}

fn handle_header_keys(
    _key: &dracon_terminal_engine::contracts::KeyEvent,
    app: &mut App,
    _event_tx: &mpsc::Sender<AppEvent>,
    _idx: usize,
) -> bool {
    match _key.code {
        KeyCode::Esc => {
            app.core.mode = AppMode::Normal;
            true
        }
        KeyCode::Enter => {
            // Header icon logic
            app.core.mode = AppMode::Normal;
            true
        }
        KeyCode::Left => {
            if _idx > 0 {
                app.core.mode = AppMode::Header(_idx - 1);
            }
            true
        }
        KeyCode::Right => {
            app.core.mode = AppMode::Header(_idx + 1);
            true
        }
        _ => true,
    }
}

pub fn handle_settings_keys(
    key: &dracon_terminal_engine::contracts::KeyEvent,
    app: &mut App,
    event_tx: &mpsc::Sender<AppEvent>,
) -> bool {
    match key.code {
        KeyCode::Esc => {
            app.core.mode = AppMode::Normal;
            true
        }
        KeyCode::Char('1') => {
            app.settings.settings_section = SettingsSection::Columns;
            app.settings.settings_index = 0;
            true
        }
        KeyCode::Char('2') => {
            app.settings.settings_section = SettingsSection::Tabs;
            app.settings.settings_index = 0;
            true
        }
        KeyCode::Char('3') => {
            app.settings.settings_section = SettingsSection::General;
            app.settings.settings_index = 0;
            true
        }
        KeyCode::Char('4') => {
            app.settings.settings_section = SettingsSection::Style;
            app.settings.settings_index = 0;
            true
        }
        KeyCode::Char('5') => {
            app.settings.settings_section = SettingsSection::Remotes;
            app.settings.settings_index = 0;
            true
        }
        KeyCode::Char('6') => {
            app.settings.settings_section = SettingsSection::Shortcuts;
            app.settings.settings_index = 0;
            true
        }
        KeyCode::Char('r') | KeyCode::Char('R')
            if app.settings.settings_section == SettingsSection::Style =>
        {
            crate::ui::theme::set_style_settings(crate::ui::theme::ThemeStyle::default());
            crate::config::save_state_quiet(app);
            true
        }
        KeyCode::Char('e') | KeyCode::Char('E')
            if app.settings.settings_section == SettingsSection::Style =>
        {
            if app.settings.settings_index == 0 {
                crate::ui::theme::set_style_settings(
                    crate::ui::theme::ThemeStyle::default(),
                );
                crate::config::save_state_quiet(app);
            } else if let Some(preset) = style_preset_for_index(app.settings.settings_index) {
                crate::ui::theme::set_style_settings(preset);
                crate::config::save_state_quiet(app);
            } else {
                open_style_color_input(app);
            }
            true
        }
        KeyCode::Up => {
            app.settings.settings_index = app.settings.settings_index.saturating_sub(1);
            true
        }
        KeyCode::Down => {
            let max = match app.settings.settings_section {
                SettingsSection::General => 14,
                SettingsSection::Columns => 3,
                SettingsSection::Style => STYLE_MAX_INDEX,
                _ => 0,
            };
            if app.settings.settings_index < max {
                app.settings.settings_index += 1;
            }
            true
        }
        KeyCode::Enter => {
            match app.settings.settings_section {
                SettingsSection::General => {
                    match app.settings.settings_index {
                        // Index 0 = Version (read_only, no toggle action)
                        1 => {
                            app.settings.default_show_hidden = !app.settings.default_show_hidden;
                            let new_val = app.settings.default_show_hidden;
                            // Sync focused tab's show_hidden to match global setting
                            if let Some(fs) = app.current_file_state_mut() {
                                fs.nav.show_hidden = new_val;
                            }
                            let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(app.focused_pane_index));
                        }
                        2 => app.settings.confirm_delete = !app.settings.confirm_delete,
                        3 => app.settings.smart_date = !app.settings.smart_date,
                        4 => app.settings.semantic_coloring = !app.settings.semantic_coloring,
                        5 => app.settings.auto_save = !app.settings.auto_save,
                        6 => app.preview_max_mb = cycle_preview_max_mb(app.preview_max_mb),
                        7 => {
                            app.core.icon_mode = match app.core.icon_mode {
                                IconMode::Nerd => IconMode::Unicode,
                                IconMode::Unicode => IconMode::ASCII,
                                IconMode::ASCII => IconMode::Nerd,
                            }
                        }
                        // Index 8 = Sidebar Sections separator (no toggle action)
                        9 => app.sidebar.sidebar_folders = !app.sidebar.sidebar_folders,
                        10 => app.sidebar.sidebar_favorites = !app.sidebar.sidebar_favorites,
                        11 => app.sidebar.sidebar_recent = !app.sidebar.sidebar_recent,
                        12 => app.sidebar.sidebar_storage = !app.sidebar.sidebar_storage,
                        13 => app.sidebar.sidebar_remotes = !app.sidebar.sidebar_remotes,
                        14 => {
                            app.core.mode = AppMode::ResetSettingsConfirm;
                            app.core.input.clear();
                        }
                        _ => {}
                    }
                    if app.settings.settings_index != 14 {
                        crate::config::save_state_quiet(app);
                    }
                }
                SettingsSection::Columns => {
                    let col = match app.settings.settings_index {
                        0 => crate::app::FileColumn::Size,
                        1 => crate::app::FileColumn::Modified,
                        2 => crate::app::FileColumn::Created,
                        3 => crate::app::FileColumn::Permissions,
                        _ => crate::app::FileColumn::Size,
                    };
                    let target_set = match app.settings.settings_target {
                        crate::app::SettingsTarget::SingleMode => &mut app.layout.single_columns,
                        crate::app::SettingsTarget::SplitMode => &mut app.layout.split_columns,
                    };
                    if let Some(pos) = target_set.iter().position(|c| c == &col) {
                        target_set.remove(pos);
                    } else {
                        target_set.push(col);
                    }
                    crate::config::save_state_quiet(app);
                }
                SettingsSection::Style => {
                    if app.settings.settings_index == 0 {
                        crate::ui::theme::set_style_settings(
                            crate::ui::theme::ThemeStyle::default(),
                        );
                        crate::config::save_state_quiet(app);
                    } else if let Some(preset) = style_preset_for_index(app.settings.settings_index) {
                        crate::ui::theme::set_style_settings(preset);
                        crate::config::save_state_quiet(app);
                    } else {
                        open_style_color_input(app);
                    }
                }
                _ => {}
            }
            true
        }
        _ => false,
    }
}
