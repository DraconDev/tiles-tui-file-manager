//! Context menu rendering.
//! Extracted from ui/mod.rs (Phase 3).

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem},
    Frame,
};

use crate::app::App;
use crate::icons::Icon;
use crate::state::AppMode;
use crate::ui::theme::{accent_primary, accent_secondary, THEME};

pub fn draw_context_menu(
    f: &mut Frame,
    x: u16,
    y: u16,
    target: &crate::app::ContextMenuTarget,
    app: &App,
) {
    use crate::app::ContextMenuAction;
    let mut items = Vec::new();

    let actions = if let AppMode::ContextMenu { actions, .. } = &app.core.mode {
        actions.clone()
    } else {
        vec![]
    };

    let selected_idx = if let AppMode::ContextMenu { selected_index, .. } = &app.core.mode {
        *selected_index
    } else {
        None
    };

    for (i, action) in actions.iter().enumerate() {
        let label = match action {
            ContextMenuAction::Open => format!(" {} Open", Icon::Folder.get(app.core.icon_mode)),
            ContextMenuAction::OpenNewTab => {
                format!(" {} Open in New Tab", Icon::Split.get(app.core.icon_mode))
            }
            ContextMenuAction::OpenWith => {
                format!(" {} Open With...", Icon::Split.get(app.core.icon_mode))
            }
            ContextMenuAction::Edit => format!(" {} Edit", Icon::Document.get(app.core.icon_mode)),
            ContextMenuAction::Run => format!(" {} Run", Icon::Video.get(app.core.icon_mode)),
            ContextMenuAction::RunTerminal => {
                format!(" {} Run in Terminal", Icon::Script.get(app.core.icon_mode))
            }
            ContextMenuAction::ExtractHere => {
                format!(" {} Extract Here", Icon::Archive.get(app.core.icon_mode))
            }
            ContextMenuAction::NewFolder => {
                format!(" {} New Folder", Icon::Folder.get(app.core.icon_mode))
            }
            ContextMenuAction::NewFile => format!(" {} New File", Icon::File.get(app.core.icon_mode)),
            ContextMenuAction::Cut => format!(" {} Cut", Icon::Cut.get(app.core.icon_mode)),
            ContextMenuAction::Copy => format!(" {} Copy", Icon::Copy.get(app.core.icon_mode)),
            ContextMenuAction::CopyPath => format!(" {} Copy Path", Icon::Copy.get(app.core.icon_mode)),
            ContextMenuAction::CopyName => format!(" {} Copy Name", Icon::Copy.get(app.core.icon_mode)),
            ContextMenuAction::Paste => format!(" {} Paste", Icon::Paste.get(app.core.icon_mode)),
            ContextMenuAction::Rename => format!(" {} Rename", Icon::Rename.get(app.core.icon_mode)),
            ContextMenuAction::Duplicate => {
                format!(" {} Duplicate", Icon::Duplicate.get(app.core.icon_mode))
            }
            ContextMenuAction::Compress => {
                format!(" {} Compress", Icon::Archive.get(app.core.icon_mode))
            }
            ContextMenuAction::Delete => format!(" {} Delete", Icon::Delete.get(app.core.icon_mode)),
            ContextMenuAction::AddToFavorites => {
                format!(" {} Add to Favorites", Icon::Star.get(app.core.icon_mode))
            }
            ContextMenuAction::RemoveFromFavorites => {
                format!(" {} Remove from Favorites", Icon::Star.get(app.core.icon_mode))
            }
            ContextMenuAction::Properties => {
                format!(" {} Properties", Icon::Document.get(app.core.icon_mode))
            }
            ContextMenuAction::TerminalWindow => {
                format!(" {} New Terminal Window", Icon::Script.get(app.core.icon_mode))
            }
            ContextMenuAction::TerminalTab => {
                format!(" {} New Terminal Tab", Icon::Script.get(app.core.icon_mode))
            }
            ContextMenuAction::Refresh => format!(" {} Refresh", Icon::Refresh.get(app.core.icon_mode)),
            ContextMenuAction::SelectAll => {
                format!(" {} Select All", Icon::SelectAll.get(app.core.icon_mode))
            }
            ContextMenuAction::ToggleHidden => {
                format!(" {} Toggle Hidden", Icon::ToggleHidden.get(app.core.icon_mode))
            }
            ContextMenuAction::ConnectRemote => {
                format!(" {} Connect", Icon::Remote.get(app.core.icon_mode))
            }
            ContextMenuAction::DeleteRemote => {
                format!(" {} Delete Bookmark", Icon::Delete.get(app.core.icon_mode))
            }
            ContextMenuAction::Mount => format!(" {} Mount", Icon::Storage.get(app.core.icon_mode)),
            ContextMenuAction::Unmount => format!(" {} Unmount", Icon::Storage.get(app.core.icon_mode)),
            ContextMenuAction::SetWallpaper => {
                format!(" {} Set as Wallpaper", Icon::Image.get(app.core.icon_mode))
            }
            ContextMenuAction::GitInit => format!(" {} Git Init", Icon::Git.get(app.core.icon_mode)),
            ContextMenuAction::GitStatus => format!(" {} Git Status", Icon::Git.get(app.core.icon_mode)),
            ContextMenuAction::SystemMonitor => {
                format!(" {} System Monitor", Icon::Monitor.get(app.core.icon_mode))
            }
            ContextMenuAction::Drag => {
                format!(" {} Drag...", Icon::Remote.get(app.core.icon_mode))
            }
            ContextMenuAction::SetColor(_) => {
                format!(" {} Highlight...", Icon::Image.get(app.core.icon_mode))
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
            ContextMenuAction::Save => format!(" {} Save", Icon::Document.get(app.core.icon_mode)),
            ContextMenuAction::EditorCut => format!(" {} Cut", Icon::Cut.get(app.core.icon_mode)),
            ContextMenuAction::EditorCopy => format!(" {} Copy", Icon::Copy.get(app.core.icon_mode)),
            ContextMenuAction::EditorPaste => format!(" {} Paste", Icon::Paste.get(app.core.icon_mode)),
            ContextMenuAction::EditorUndo => format!(" {} Undo", Icon::Refresh.get(app.core.icon_mode)),
            ContextMenuAction::EditorRedo => format!(" {} Redo", Icon::Refresh.get(app.core.icon_mode)),
            ContextMenuAction::EditorSelectAll => {
                format!(" {} Select All", Icon::SelectAll.get(app.core.icon_mode))
            }
            ContextMenuAction::Undo => format!(" {} Undo", Icon::Refresh.get(app.core.icon_mode)),
            ContextMenuAction::Redo => format!(" {} Redo", Icon::Refresh.get(app.core.icon_mode)),
            ContextMenuAction::Separator => " ────────────────".to_string(),
        };

        let style = if Some(i) == selected_idx {
            Style::default()
                .bg(accent_primary())
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(THEME.fg)
        };

        let mut item = ListItem::new(label).style(style);
        if (*action == ContextMenuAction::Paste) && app.selection.clipboard.is_none() {
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

    f.render_widget(Clear, area);
    let menu_block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(accent_secondary()));

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
