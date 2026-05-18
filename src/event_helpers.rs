#![allow(clippy::needless_borrow)]

use crate::app::{
    App, AppEvent, AppMode, CommandAction, CommandItem, ContextMenuAction, ContextMenuTarget,
    CurrentView,
};
use crate::clipboard::copy_text_to_clipboard;
use dracon_terminal_engine::input::mapping::{to_runtime_event, Event as InputEvent, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::ffi::OsStr;
use std::path::PathBuf;


use tokio::sync::mpsc;

pub fn update_commands(app: &mut App) {
    let mut commands = vec![
        CommandItem {
            key: "q".to_string(),
            desc: "Quit".to_string(),
            action: CommandAction::Quit,
        },
        CommandItem {
            key: "z".to_string(),
            desc: "Toggle Zoom".to_string(),
            action: CommandAction::ToggleZoom,
        },
        CommandItem {
            key: "f".to_string(),
            desc: "File Manager".to_string(),
            action: CommandAction::SwitchView(CurrentView::Files),
        },
        CommandItem {
            key: "e".to_string(),
            desc: "Editor".to_string(),
            action: CommandAction::SwitchView(CurrentView::Editor),
        },
        CommandItem {
            key: "g".to_string(),
            desc: "Git".to_string(),
            action: CommandAction::SwitchView(CurrentView::Git),
        },
        CommandItem {
            key: "m".to_string(),
            desc: "Monitor".to_string(),
            action: CommandAction::SwitchView(CurrentView::Processes),
        },
        CommandItem {
            key: "a".to_string(),
            desc: "Add Remote".to_string(),
            action: CommandAction::AddRemote,
        },
    ];

    for (i, bookmark) in app.remote.remote_bookmarks.iter().enumerate() {
        commands.push(CommandItem {
            key: format!("r{}", i),
            desc: format!("Connect to {}", bookmark.name),
            action: CommandAction::ConnectToRemote(i),
        });
    }

    app.nav.filtered_commands = commands
        .into_iter()
        .filter(|cmd| {
            cmd.desc
                .to_lowercase()
                .contains(&app.core.input.value.to_lowercase())
        })
        .collect();
    app.nav.command_index = app.nav.command_index
        .min(app.nav.filtered_commands.len().saturating_sub(1));
}

pub fn execute_command(action: CommandAction, app: &mut App, event_tx: mpsc::Sender<AppEvent>) {
    match action {
        CommandAction::Quit => {
            crate::config::save_state_quiet(app);
            app.core.running = false;
        }
        CommandAction::ToggleZoom => app.toggle_split(),
        CommandAction::SwitchView(view) => app.core.current_view = view,
        CommandAction::AddRemote => {
            app.core.mode = AppMode::AddRemote(0);
            app.core.input.clear();
        }
        CommandAction::ConnectToRemote(idx) => {
            let _ = crate::app::try_send_event(&event_tx, AppEvent::ConnectToRemote(app.focused_pane_index, idx));
        }
        CommandAction::CommandPalette => {
            app.core.mode = AppMode::CommandPalette;
        }
    }
}

pub fn get_context_menu_actions(target: &ContextMenuTarget, app: &App) -> Vec<ContextMenuAction> {
    match target {
        ContextMenuTarget::File(idx) => {
            let mut actions = vec![
                ContextMenuAction::Open,
                ContextMenuAction::OpenWith,
                ContextMenuAction::Separator,
                ContextMenuAction::Cut,
                ContextMenuAction::Copy,
                ContextMenuAction::CopyPath,
                ContextMenuAction::CopyName,
                ContextMenuAction::Separator,
            ];

            if let Some(fs) = app.current_file_state() {
                if let Some(path) = fs.list.files.get(*idx) {
                    let ext = path
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    if matches!(ext.as_str(), "zip" | "tar" | "gz" | "7z" | "rar") {
                        actions.push(ContextMenuAction::ExtractHere);
                    } else {
                        actions.push(ContextMenuAction::Compress);
                    }

                    // Toggle Add/Remove Favorites
                    if app.nav.starred.contains(path) {
                        actions.push(ContextMenuAction::RemoveFromFavorites);
                    } else {
                        actions.push(ContextMenuAction::AddToFavorites);
                    }
                }
            }

            actions.extend(vec![
                ContextMenuAction::Rename,
                ContextMenuAction::Delete,
                ContextMenuAction::Separator,
                ContextMenuAction::SetColor(None),
                ContextMenuAction::Separator,
                ContextMenuAction::Properties,
            ]);
            actions
        }
        ContextMenuTarget::Folder(idx) => {
            let mut actions = vec![
                ContextMenuAction::Open,
                ContextMenuAction::OpenNewTab,
                ContextMenuAction::TerminalTab,
                ContextMenuAction::TerminalWindow,
                ContextMenuAction::Separator,
                ContextMenuAction::Cut,
                ContextMenuAction::Copy,
                ContextMenuAction::CopyPath,
                ContextMenuAction::CopyName,
                ContextMenuAction::Separator,
                ContextMenuAction::Rename,
                ContextMenuAction::Delete,
                ContextMenuAction::Separator,
            ];

            if let Some(fs) = app.current_file_state() {
                if let Some(path) = fs.list.files.get(*idx) {
                    // Toggle Add/Remove Favorites
                    if app.nav.starred.contains(path) {
                        actions.push(ContextMenuAction::RemoveFromFavorites);
                    } else {
                        actions.push(ContextMenuAction::AddToFavorites);
                    }
                }
            }

            actions.extend(vec![
                ContextMenuAction::Compress,
                ContextMenuAction::SetColor(None),
                ContextMenuAction::Separator,
                ContextMenuAction::Properties,
            ]);
            actions
        }
        ContextMenuTarget::SidebarFavorite(_path) => {
            let actions = vec![
                ContextMenuAction::Open,
                ContextMenuAction::OpenNewTab,
                ContextMenuAction::Separator,
                ContextMenuAction::NewFile,
                ContextMenuAction::NewFolder,
                ContextMenuAction::Separator,
                ContextMenuAction::TerminalTab,
                ContextMenuAction::TerminalWindow,
                ContextMenuAction::Separator,
                ContextMenuAction::RemoveFromFavorites,
                ContextMenuAction::Separator,
                ContextMenuAction::Properties,
            ];
            actions
        }
        ContextMenuTarget::EmptySpace => {
            let mut actions = vec![ContextMenuAction::NewFile, ContextMenuAction::NewFolder];
            if app.selection.clipboard.is_some() {
                actions.push(ContextMenuAction::Paste);
            }
            actions.extend(vec![
                ContextMenuAction::Separator,
                ContextMenuAction::ToggleHidden,
                ContextMenuAction::Separator,
                ContextMenuAction::TerminalTab,
                ContextMenuAction::TerminalWindow,
                ContextMenuAction::SystemMonitor,
            ]);
            actions
        }

        ContextMenuTarget::SidebarRemote(_) => vec![
            ContextMenuAction::ConnectRemote,
            ContextMenuAction::DeleteRemote,
            ContextMenuAction::Separator,
            ContextMenuAction::Properties,
        ],
        ContextMenuTarget::SidebarStorage(_) => vec![
            ContextMenuAction::Mount,
            ContextMenuAction::Unmount,
            ContextMenuAction::Separator,
            ContextMenuAction::Properties,
        ],
        ContextMenuTarget::ProjectTree(path) => {
            let mut actions = vec![
                ContextMenuAction::NewFile,
                ContextMenuAction::NewFolder,
                ContextMenuAction::Separator,
            ];
            if path.is_file() {
                actions.extend(vec![
                    ContextMenuAction::Rename,
                    ContextMenuAction::Delete,
                    ContextMenuAction::Separator,
                ]);
            } else {
                actions.extend(vec![
                    ContextMenuAction::TerminalTab,
                    ContextMenuAction::Separator,
                ]);
            }
            actions.push(ContextMenuAction::Properties);
            actions
        }
        ContextMenuTarget::Process(_) => vec![
            ContextMenuAction::Delete, // Kill
            ContextMenuAction::Properties,
        ],
        ContextMenuTarget::Editor => vec![],
    }
}

fn get_active_editor_mut(app: &mut App) -> Option<&mut dracon_terminal_engine::widgets::TextEditor> {
    if app.core.current_view == CurrentView::Editor {
        if let Some(pane) = app.panes.get_mut(app.focused_pane_index) {
            if let Some(fs) = pane.current_state_mut() {
                if let Some(preview) = &mut fs.view.preview {
                    if let Some(editor) = &mut preview.editor {
                        return Some(editor);
                    }
                }
            }
        }
    }
    if let Some(preview) = &mut app.editor_global.editor_state {
        if let Some(editor) = &mut preview.editor {
            return Some(editor);
        }
    }
    None
}

fn get_active_editor_path(app: &App) -> Option<PathBuf> {
    if app.core.current_view == CurrentView::Editor {
        if let Some(pane) = app.panes.get(app.focused_pane_index) {
            if let Some(fs) = pane.current_state() {
                if let Some(preview) = &fs.view.preview {
                    return Some(preview.path.clone());
                }
            }
        }
    }
    if let Some(preview) = &app.editor_global.editor_state {
        return Some(preview.path.clone());
    }
    None
}

pub fn handle_context_menu_action(
    action: &ContextMenuAction,
    target: &ContextMenuTarget,
    app: &mut App,
    event_tx: mpsc::Sender<AppEvent>,
) {
    match action {
        ContextMenuAction::Open => {
            if let ContextMenuTarget::File(idx) | ContextMenuTarget::Folder(idx) = target {
                let path_opt = app.current_file_state()
                    .and_then(|fs| fs.list.files.get(*idx).cloned());
                if let Some(path) = path_opt {
                    if path.is_dir() {
                        let path_clone = path.clone();
                        if let Some(fs_mut) = app.current_file_state_mut() {
                            fs_mut.nav.current_path = path_clone;
                            let _ =
                                crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(app.focused_pane_index));
                        }
                    } else {
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::PreviewRequested(
                            app.focused_pane_index,
                            path.clone(),
                        ));
                    }
                }
            }
        }
        ContextMenuAction::AddToFavorites => {
            if let ContextMenuTarget::Folder(idx) | ContextMenuTarget::File(idx) = target {
                let path_opt = app.current_file_state()
                    .and_then(|fs| fs.list.files.get(*idx).cloned());
                if let Some(path) = path_opt {
                    if !app.nav.starred.contains(&path) {
                        app.nav.starred.push(path);
                        crate::config::save_state_quiet(app);
                        // Refresh to update sidebar
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(app.focused_pane_index));
                    }
                }
            }
        }
        ContextMenuAction::RemoveFromFavorites => {
            let mut removed = false;
            match target {
                ContextMenuTarget::SidebarFavorite(path) => {
                    let path_clone = path.clone();
                    app.nav.starred.retain(|p| p != &path_clone);
                    removed = true;
                }
                ContextMenuTarget::File(idx) | ContextMenuTarget::Folder(idx) => {
                    if let Some(fs) = app.current_file_state() {
                        if let Some(path) = fs.list.files.get(*idx) {
                            let path_clone = path.clone();
                            if app.nav.starred.contains(&path_clone) {
                                app.nav.starred.retain(|p| p != &path_clone);
                                removed = true;
                            }
                        }
                    }
                }
                _ => {}
            }
            if removed {
                crate::config::save_state_quiet(app);
                let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(app.focused_pane_index));
            }
        }
        ContextMenuAction::Rename => {
            if let ContextMenuTarget::File(idx) | ContextMenuTarget::Folder(idx) = target {
                let path_opt = app.current_file_state()
                    .and_then(|fs| fs.list.files.get(*idx).cloned());
                if let Some(path) = path_opt {
                    if let Some(name) = path.file_name() {
                        let name_str = name.to_string_lossy().to_string();
                        app.core.mode = AppMode::Rename;
                        app.core.input.set_value(name_str);
                    }
                }
            }
        }
        ContextMenuAction::Delete => {
            if let ContextMenuTarget::File(idx) | ContextMenuTarget::Folder(idx) = target {
                let path_opt = app.current_file_state()
                    .and_then(|fs| fs.list.files.get(*idx).cloned());
                if let Some(path) = path_opt {
                    if matches!(target, ContextMenuTarget::File(_)) {
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::TrashFile(path.clone()));
                    } else {
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::Delete(path.clone()));
                    }
                }
            }
        }
        ContextMenuAction::CopyPath | ContextMenuAction::CopyName => {
            match copy_target_text(action, target, app) {
                Ok(text) => match copy_text_to_clipboard(&text) {
                    Ok(()) => {
                        let label = if matches!(action, ContextMenuAction::CopyName) {
                            "name"
                        } else {
                            "path"
                        };
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::StatusMsg(format!(
                            "Copied {} to clipboard",
                            label
                        )));
                    }
                    Err(err) => {
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::StatusMsg(format!("Clipboard failed: {}", err)));
                    }
                },
                Err(err) => {
                    let _ = crate::app::try_send_event(&event_tx, AppEvent::StatusMsg(err));
                }
            }
        }
        ContextMenuAction::Refresh => {
            let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(app.focused_pane_index));
        }
        ContextMenuAction::ToggleHidden => {
            if let Some(fs) = app.current_file_state_mut() {
                fs.nav.show_hidden = !fs.nav.show_hidden;
                app.settings.default_show_hidden = fs.nav.show_hidden;
                crate::config::save_state_quiet(app);
                let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(app.focused_pane_index));
            }
        }
        ContextMenuAction::TerminalTab | ContextMenuAction::TerminalWindow => {
            let new_tab = matches!(action, ContextMenuAction::TerminalTab);
            let mut path_to_open = None;
            let mut remote = None;

            if let Some(fs) = app.current_file_state() {
                remote = fs.nav.remote_session.clone();
            }

            match target {
                ContextMenuTarget::Folder(idx) => {
                    if let Some(fs) = app.current_file_state() {
                        path_to_open = fs.list.files.get(*idx).cloned();
                    }
                }
                ContextMenuTarget::EmptySpace => {
                    if let Some(fs) = app.current_file_state() {
                        path_to_open = Some(fs.nav.current_path.clone());
                    }
                }
                ContextMenuTarget::ProjectTree(p) => {
                    path_to_open = Some(p.clone());
                }
                _ => {}
            }

            if let Some(path) = path_to_open {
                let _ = crate::app::try_send_event(&event_tx, AppEvent::SpawnTerminal {
                    path,
                    new_tab,
                    remote,
                    command: None,
                });
            }
        }
        ContextMenuAction::OpenNewTab => {
            if let ContextMenuTarget::Folder(idx) = target {
                if let Some(pane) = app.panes.get_mut(app.focused_pane_index) {
                    if let Some(fs) = pane.current_state() {
                        if let Some(path) = fs.list.files.get(*idx).cloned() {
                            let mut new_fs = fs.clone();
                            new_fs.nav.current_path = path;
                            new_fs.list.selection.clear();
                            let current_path_clone = new_fs.nav.current_path.clone();
                            crate::nav_helpers::push_history(&mut new_fs, current_path_clone);
                            pane.open_tab(new_fs);
                            let _ =
                                crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(app.focused_pane_index));
                        }
                    }
                }
            }
        }
        ContextMenuAction::NewFile | ContextMenuAction::NewFolder => {
            let mut target_dir = app.current_file_state().map(|fs| fs.nav.current_path.clone());
            match target {
                ContextMenuTarget::Folder(idx) => {
                    if let Some(fs) = app.current_file_state() {
                        if let Some(p) = fs.list.files.get(*idx) {
                            target_dir = Some(p.clone());
                        }
                    }
                }
                ContextMenuTarget::File(idx) => {
                    if let Some(fs) = app.current_file_state() {
                        if let Some(p) = fs.list.files.get(*idx) {
                            target_dir = p.parent().map(|pp| pp.to_path_buf());
                        }
                    }
                }
                ContextMenuTarget::ProjectTree(path) | ContextMenuTarget::SidebarFavorite(path) => {
                    if path.is_dir() {
                        target_dir = Some(path.clone());
                    } else {
                        target_dir = path.parent().map(|pp| pp.to_path_buf());
                    }
                }
                ContextMenuTarget::SidebarRemote(idx) => {
                    if let Some(bookmark) = app.remote.remote_bookmarks.get(*idx) {
                        target_dir = Some(bookmark.last_path.clone());
                    }
                }
                ContextMenuTarget::EmptySpace => {}
                _ => {}
            }
            if let (Some(fs), Some(dir)) = (app.current_file_state_mut(), target_dir) {
                fs.nav.current_path = dir;
            }
            app.core.mode = if matches!(action, ContextMenuAction::NewFolder) {
                AppMode::NewFolder
            } else {
                AppMode::NewFile
            };
            app.core.input.clear();
            app.selection.rename_selected = false;
        }
        ContextMenuAction::Cut => {
            if let ContextMenuTarget::File(idx) | ContextMenuTarget::Folder(idx) = target {
                let path_opt = app.current_file_state()
                    .and_then(|fs| fs.list.files.get(*idx).cloned());
                if let Some(path) = path_opt {
                    app.selection.clipboard = Some((path, crate::app::ClipboardOp::Cut));
                }
            }
        }
        ContextMenuAction::Copy => {
            if let ContextMenuTarget::File(idx) | ContextMenuTarget::Folder(idx) = target {
                let path_opt = app.current_file_state()
                    .and_then(|fs| fs.list.files.get(*idx).cloned());
                if let Some(path) = path_opt {
                    app.selection.clipboard = Some((path, crate::app::ClipboardOp::Copy));
                }
            }
        }
        ContextMenuAction::Paste => {
            if let Some((src, op)) = app.selection.clipboard.clone() {
                let target_dir = match target {
                    ContextMenuTarget::Folder(idx) => {
                        app.current_file_state()
                            .and_then(|fs| fs.list.files.get(*idx).cloned())
                    }
                    ContextMenuTarget::SidebarFavorite(path) => Some(path.clone()),
                    ContextMenuTarget::EmptySpace => {
                        app.current_file_state().map(|fs| fs.nav.current_path.clone())
                    }
                    _ => app.current_file_state().map(|fs| fs.nav.current_path.clone()),
                };
                if let Some(dest_dir) = target_dir {
                    let dest = dest_dir.join(
                        src.file_name()
                            .unwrap_or_else(|| OsStr::new("root")),
                    );
                    match op {
                        crate::app::ClipboardOp::Copy => {
                            let _ = crate::app::try_send_event(&event_tx, AppEvent::Copy(src, dest));
                        }
                        crate::app::ClipboardOp::Cut => {
                            let result = crate::app::try_send_event(&event_tx, AppEvent::Rename(src, dest));
                            if result {
                                app.selection.clipboard = None;
                            }
                        }
                    }
                }
            }
        }
        ContextMenuAction::Compress => {
            if let ContextMenuTarget::File(idx) | ContextMenuTarget::Folder(idx) = target {
                let path_opt = app.current_file_state()
                    .and_then(|fs| fs.list.files.get(*idx).cloned());
                if let Some(path) = path_opt {
                    let name = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("archive");
                    let dest = path.parent()
                        .map(|p| p.join(format!("{}.zip", name)));
                    if let Some(dest_path) = dest {
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::StatusMsg(format!(
                            "Compressing {}...",
                            path.display()
                        )));
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::Copy(path.clone(), dest_path));
                    }
                }
            }
        }
        ContextMenuAction::ExtractHere => {
            if let ContextMenuTarget::File(idx) = target {
                let path_opt = app.current_file_state()
                    .and_then(|fs| fs.list.files.get(*idx).cloned());
                if let Some(path) = path_opt {
                    if let Some(parent) = path.parent() {
                        let dest_dir = parent.join(
                            path.file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("extracted")
                        );
                        let ext = path.extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("")
                            .to_lowercase();
                        let cmd = match ext.as_str() {
                            "zip" => Some(("unzip".to_string(), vec!["-o".to_string(), path.to_string_lossy().to_string(), "-d".to_string(), dest_dir.to_string_lossy().to_string()])),
                            "tar" | "gz" | "tgz" | "tar.gz" | "tar.bz2" | "tar.xz" | "txz" => {
                                Some(("tar".to_string(), vec!["xf".to_string(), path.to_string_lossy().to_string(), "-C".to_string(), dest_dir.to_string_lossy().to_string()]))
                            }
                            "7z" => Some(("7z".to_string(), vec!["x".to_string(), path.to_string_lossy().to_string(), "-o".to_string(), dest_dir.to_string_lossy().to_string()])),
                            "rar" => Some(("unrar".to_string(), vec!["x".to_string(), "-o+".to_string(), path.to_string_lossy().to_string(), dest_dir.to_string_lossy().to_string()])),
                            _ => None,
                        };
                        if let Some((cmd_name, args)) = cmd {
                            let _ = crate::app::try_send_event(&event_tx, AppEvent::StatusMsg(format!(
                                "Extracting {} to {}...",
                                path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default(),
                                dest_dir.file_name().map(|n| n.to_string_lossy()).unwrap_or_default()
                            )));
                            let _ = crate::app::try_send_event(&event_tx, AppEvent::SpawnDetached {
                                cmd: cmd_name,
                                args
                            });
                        } else {
                            let _ = crate::app::try_send_event(&event_tx, AppEvent::StatusMsg(format!(
                                "Unsupported archive format: .{}",
                                ext
                            )));
                        }
                    }
                }
            }
        }
        ContextMenuAction::OpenWith => {
            if let ContextMenuTarget::File(idx) = target {
                let path_opt = app.current_file_state()
                    .and_then(|fs| fs.list.files.get(*idx).cloned());
                if let Some(path) = path_opt {
                    app.core.mode = AppMode::OpenWith(path.clone());
                    app.core.input.clear();
                }
            }
        }
        ContextMenuAction::Duplicate => {
            if let ContextMenuTarget::File(idx) = target {
                let path_opt = app.current_file_state()
                    .and_then(|fs| fs.list.files.get(*idx).cloned());
                if let Some(path) = path_opt {
                    if let Some(parent) = path.parent() {
                        let stem = path.file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("file");
                        let ext = path.extension()
                            .and_then(|e| e.to_str())
                            .map(|s| format!(".{}", s))
                            .unwrap_or_default();
                        let new_name = format!("{}_copy{}", stem, ext);
                        let dest = parent.join(new_name);
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::Copy(path, dest));
                    }
                }
            }
        }
        ContextMenuAction::SystemMonitor => {
            let _ = crate::app::try_send_event(&event_tx, AppEvent::SystemMonitor);
        }
        ContextMenuAction::Run | ContextMenuAction::RunTerminal => {
            match target {
                ContextMenuTarget::File(idx) => {
                    let path_opt = app.current_file_state()
                        .and_then(|fs| fs.list.files.get(*idx).cloned());
                    if let Some(path) = path_opt {
                        if path.is_dir() {
                            return;
                        }
                        let remote = app.current_file_state()
                            .and_then(|fs| fs.nav.remote_session.clone());
                        if let Some((work_dir, program, args)) =
                            crate::modules::files::get_run_command(&path)
                        {
                            let _ = crate::app::try_send_event(&event_tx, AppEvent::SpawnTerminal {
                                path: work_dir,
                                new_tab: true,
                                remote,
                                command: Some(format!("{} {}", program, args.join(" "))),
                            });
                            let _ = crate::app::try_send_event(&event_tx, AppEvent::StatusMsg(format!(
                                "Running: {} {}",
                                program,
                                args.join(" ")
                            )));
                        } else {
                            let _ = crate::app::try_send_event(&event_tx, AppEvent::StatusMsg(format!(
                                "No run command for: {}",
                                path.extension()
                                    .and_then(|e| e.to_str())
                                    .map(|e| format!(".{e}"))
                                    .unwrap_or_else(|| "unknown".to_string())
                            )));
                        }
                    }
                }
                ContextMenuTarget::Editor => {
                    if let Some(path) = get_active_editor_path(app) {
                        if let Some((work_dir, program, args)) =
                            crate::modules::files::get_run_command(&path)
                        {
                            let remote = app.current_file_state().and_then(|fs| fs.nav.remote_session.clone());
                            let _ = crate::app::try_send_event(&event_tx, AppEvent::SpawnTerminal {
                                path: work_dir,
                                new_tab: true,
                                remote,
                                command: Some(format!("{} {}", program, args.join(" "))),
                            });
                            let _ = crate::app::try_send_event(&event_tx, AppEvent::StatusMsg(format!(
                                "Running: {} {}",
                                program,
                                args.join(" ")
                            )));
                        }
                    }
                }
                _ => {}
            }
        }
        ContextMenuAction::EditorSelectAll => {
            if let Some(editor) = get_active_editor_mut(app) {
                editor.select_all();
            }
        }
        ContextMenuAction::EditorCopy => {
            let text = {
                if let Some(editor) = get_active_editor_mut(app) {
                    editor.get_selected_text()
                } else {
                    None
                }
            };
            if let Some(text) = text {
                app.editor_global.editor_clipboard = Some(text.clone());
                let _ = copy_text_to_clipboard(&text);
            }
        }
        ContextMenuAction::EditorCut => {
            let text = {
                if let Some(editor) = get_active_editor_mut(app) {
                    editor.get_selected_text()
                } else {
                    None
                }
            };
            if let Some(text) = text {
                app.editor_global.editor_clipboard = Some(text.clone());
                let _ = copy_text_to_clipboard(&text);
                if let Some(editor) = get_active_editor_mut(app) {
                    editor.delete_selection();
                }
            }
        }
        ContextMenuAction::EditorPaste => {
            let text = app.editor_global.editor_clipboard.clone()
                .or_else(dracon_terminal_engine::utils::get_clipboard_text);
            if let Some(text) = text {
                if let Some(editor) = get_active_editor_mut(app) {
                    editor.insert_string(&text);
                    editor.modified = true;
                }
            }
        }
        ContextMenuAction::EditorUndo | ContextMenuAction::Undo => {
            if let Some(editor) = get_active_editor_mut(app) {
                let editor_area = ratatui::layout::Rect::new(0, 0, 9999, 9999);
                let ctrl = KeyModifiers::CONTROL;
                let key_event = KeyEvent { code: KeyCode::Char('z'), modifiers: ctrl, kind: KeyEventKind::Press };
                let event = InputEvent::Key(key_event);
                let _ = editor.handle_event(&to_runtime_event(&event), editor_area);
            }
        }
        ContextMenuAction::EditorRedo | ContextMenuAction::Redo => {
            if let Some(editor) = get_active_editor_mut(app) {
                let editor_area = ratatui::layout::Rect::new(0, 0, 9999, 9999);
                let ctrl = KeyModifiers::CONTROL;
                let key_event = KeyEvent { code: KeyCode::Char('y'), modifiers: ctrl, kind: KeyEventKind::Press };
                let event = InputEvent::Key(key_event);
                let _ = editor.handle_event(&to_runtime_event(&event), editor_area);
            }
        }
        ContextMenuAction::Save => {
            let path = get_active_editor_path(app);
            let content = get_active_editor_mut(app).map(|editor| editor.get_content());
            if let (Some(path), Some(content)) = (path, content) {
                let _ = crate::app::try_send_event(&event_tx, AppEvent::SaveFile(path, content));
                if let Some(editor) = get_active_editor_mut(app) {
                    editor.modified = false;
                }
            }
        }
        _ => {}
    }
}

fn copy_target_text(
    action: &ContextMenuAction,
    target: &ContextMenuTarget,
    app: &App,
) -> Result<String, String> {
    let path = match target {
        ContextMenuTarget::File(idx) | ContextMenuTarget::Folder(idx) => app.current_file_state()
            .and_then(|fs| fs.list.files.get(*idx))
            .cloned()
            .ok_or_else(|| "No file selected".to_string())?,
        ContextMenuTarget::SidebarFavorite(path) | ContextMenuTarget::ProjectTree(path) => {
            path.clone()
        }
        _ => return Err("Nothing here supports path copy".to_string()),
    };

    if matches!(action, ContextMenuAction::CopyName) {
        Ok(path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string()))
    } else {
        Ok(path.to_string_lossy().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_app() -> App {
        App::new()
    }

    #[test]
    fn update_commands_includes_quit() {
        let mut app = test_app();
        update_commands(&mut app);
        assert!(!app.nav.filtered_commands.is_empty());
        assert!(app.nav.filtered_commands.iter().any(|c| c.action == CommandAction::Quit));
    }

    #[test]
    fn update_commands_includes_view_switches() {
        let mut app = test_app();
        update_commands(&mut app);
        assert!(app.nav.filtered_commands.iter().any(|c| matches!(c.action, CommandAction::SwitchView(_))));
    }

    #[test]
    fn update_commands_filters_by_input() {
        let mut app = test_app();
        app.core.input.value = "quit".to_string();
        update_commands(&mut app);
        assert_eq!(app.nav.filtered_commands.len(), 1);
    }
}

