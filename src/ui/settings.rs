#![allow(unused_imports)]

//! Settings panel — settings modal, shortcuts, column, tab, general settings.
//! Extracted from ui/mod.rs (Phase 3).

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, Borders, Cell, Clear, Gauge, List, ListItem, ListState, Paragraph, Row, Table, TableState, Tabs, Widget,
    },
    Frame,
};

use crate::app::App;
use crate::app::{SettingsSection, SettingsTarget};
use crate::icons::Icon;
use crate::ui::theme as theme;
use crate::state::FileColumn;
use dracon_terminal_engine::layout::centered_rect;
use dracon_terminal_engine::utils::format_size;
use dracon_terminal_engine::widgets::HotkeyHint;

pub fn draw_settings_modal(f: &mut Frame, app: &App) {
    let area = f.area();

    f.render_widget(Clear, area);

    let block = Block::default()
        .title_top(Line::from(vec![Span::styled(
            " SETTINGS ",
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
        .border_style(Style::default().fg(theme::accent_primary()))
        .style(Style::default().bg(theme::bg()));

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

    let sel = match app.settings.settings_section {
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
                        .bg(theme::accent_primary())
                        .fg(theme::selection_fg())
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
                .border_style(Style::default().fg(theme::muted())),
        ),
        chunks[0],
    );
    match app.settings.settings_section {
        SettingsSection::Columns => draw_column_settings(f, chunks[1], app),
        SettingsSection::Tabs => draw_tab_settings(f, chunks[1], app),
        SettingsSection::General => draw_general_settings(f, chunks[1], app),
        SettingsSection::Style => draw_style_settings(f, chunks[1], app),
        SettingsSection::Remotes => crate::ui::debug::draw_remote_settings(f, chunks[1], app),
        SettingsSection::Shortcuts => draw_shortcuts_settings(f, chunks[1], app),
    }
}

pub fn draw_shortcuts_settings(f: &mut Frame, area: Rect, _app: &App) {
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
                ("F2", "Replace"),
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
                ("Ctrl + . / Ctrl + k", "New Terminal Window"),
            ],
        ),
    ];

    let mut rows = Vec::new();
    for (category, items) in shortcuts {
        rows.push(Row::new(vec![
            Cell::from(Span::styled(
                category,
                Style::default()
                    .fg(theme::accent_primary())
                    .add_modifier(Modifier::BOLD),
            )),
            Cell::from(""),
        ]));
        for (key, desc) in items {
            rows.push(Row::new(vec![
                Cell::from(Span::styled(key, Style::default().fg(theme::warning()))),
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

pub fn draw_column_settings(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);
    let titles = vec![" [Single] ", " [Split] "];
    let sel = match app.settings.settings_target {
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
                    .fg(theme::warning())
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
    let target = match app.settings.settings_target {
        SettingsTarget::SingleMode => &app.layout.single_columns,
        SettingsTarget::SplitMode => &app.layout.split_columns,
    };
    let items: Vec<ListItem> = options
        .iter()
        .enumerate()
        .map(|(i, (col, label))| {
            let prefix = if target.contains(col) { "[x] " } else { "[ ] " };
            let mut style = Style::default().fg(theme::fg());
            if i == app.settings.settings_index && app.settings.settings_section == SettingsSection::Columns {
                style = Style::default()
                    .bg(theme::accent_primary())
                    .fg(theme::selection_fg())
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

pub fn draw_tab_settings(f: &mut Frame, area: Rect, app: &App) {
    let mut rows = Vec::new();
    let mut tab_counter = 0;

    for (p_idx, pane) in app.panes.iter().enumerate() {
        rows.push(Row::new(vec![
            Cell::from(Span::styled(
                format!("PANE {}", p_idx + 1),
                Style::default()
                    .fg(theme::accent_secondary())
                    .add_modifier(Modifier::BOLD),
            )),
            Cell::from(""),
            Cell::from(""),
        ]));

        for (t_idx, tab) in pane.tabs.iter().enumerate() {
            let is_selected =
                tab_counter == app.settings.settings_index && app.settings.settings_section == SettingsSection::Tabs;
            let mut style = Style::default().fg(theme::fg());
            if is_selected {
                style = style
                    .bg(theme::accent_primary())
                    .fg(theme::selection_fg())
                    .add_modifier(Modifier::BOLD);
            }

            let is_active = t_idx == pane.active_tab_index;
            let status = if is_active {
                " [ACTIVE] "
            } else {
                "          "
            };
            let status_style = if is_active {
                Style::default().fg(theme::success())
            } else {
                Style::default()
            };

            rows.push(Row::new(vec![
                Cell::from(format!("  Tab {}", t_idx + 1)).style(style),
                Cell::from(tab.nav.current_path.to_string_lossy().to_string()).style(style),
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
                .fg(theme::accent_secondary())
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::default()
            .title(" OPEN TABS MANAGEMENT ")
            .borders(Borders::TOP)
            .border_style(Style::default().fg(theme::border_subtle())),
    )
    .column_spacing(2);

    f.render_widget(table, area);
}

pub fn draw_general_settings(f: &mut Frame, area: Rect, app: &App) {
    struct GeneralOption {
        label: &'static str,
        status: String,
        key: &'static str,
        bool_state: Option<bool>,
        read_only: bool,
    }
    let options = [
        GeneralOption {
            label: "Version",
            status: format!("{}  (press ? for help)", env!("CARGO_PKG_VERSION")),
            key: "",
            bool_state: None,
            read_only: true,
        },
        GeneralOption {
            label: "Show Hidden Files",
            status: if app.settings.default_show_hidden {
                "ENABLED ".to_string()
            } else {
                "DISABLED".to_string()
            },
            key: "h",
            bool_state: Some(app.settings.default_show_hidden),
            read_only: false,
        },
        GeneralOption {
            label: "Confirm Delete",
            status: if app.settings.confirm_delete {
                "ENABLED ".to_string()
            } else {
                "DISABLED".to_string()
            },
            key: "d",
            bool_state: Some(app.settings.confirm_delete),
            read_only: false,
        },
        GeneralOption {
            label: "Smart Date Formatting",
            status: if app.settings.smart_date {
                "ENABLED ".to_string()
            } else {
                "DISABLED".to_string()
            },
            key: "t",
            bool_state: Some(app.settings.smart_date),
            read_only: false,
        },
        GeneralOption {
            label: "Semantic Coloring",
            status: if app.settings.semantic_coloring {
                "ENABLED ".to_string()
            } else {
                "DISABLED".to_string()
            },
            key: "s",
            bool_state: Some(app.settings.semantic_coloring),
            read_only: false,
        },
        GeneralOption {
            label: "Auto Save",
            status: if app.settings.auto_save {
                "ENABLED ".to_string()
            } else {
                "DISABLED".to_string()
            },
            key: "a",
            bool_state: Some(app.settings.auto_save),
            read_only: false,
        },
        GeneralOption {
            label: "Preview Max Size",
            status: format!("{} MB", app.preview_max_mb),
            key: "p",
            bool_state: None,
            read_only: false,
        },
        GeneralOption {
            label: "Icon Mode",
            status: format!("{:?}", app.core.icon_mode),
            key: "i",
            bool_state: None,
            read_only: false,
        },
        GeneralOption {
            label: "─── Sidebar Sections ───",
            status: "".to_string(),
            key: "",
            bool_state: None,
            read_only: false,
        },
        GeneralOption {
            label: "Sidebar Folders",
            status: if app.sidebar.sidebar_folders {
                "ENABLED ".to_string()
            } else {
                "DISABLED".to_string()
            },
            key: "f",
            bool_state: Some(app.sidebar.sidebar_folders),
            read_only: false,
        },
        GeneralOption {
            label: "Sidebar Favorites",
            status: if app.sidebar.sidebar_favorites {
                "ENABLED ".to_string()
            } else {
                "DISABLED".to_string()
            },
            key: "v",
            bool_state: Some(app.sidebar.sidebar_favorites),
            read_only: false,
        },
        GeneralOption {
            label: "Sidebar Recent",
            status: if app.sidebar.sidebar_recent {
                "ENABLED ".to_string()
            } else {
                "DISABLED".to_string()
            },
            key: "c",
            bool_state: Some(app.sidebar.sidebar_recent),
            read_only: false,
        },
        GeneralOption {
            label: "Sidebar Storage",
            status: if app.sidebar.sidebar_storage {
                "ENABLED ".to_string()
            } else {
                "DISABLED".to_string()
            },
            key: "g",
            bool_state: Some(app.sidebar.sidebar_storage),
            read_only: false,
        },
        GeneralOption {
            label: "Sidebar Remotes",
            status: if app.sidebar.sidebar_remotes {
                "ENABLED ".to_string()
            } else {
                "DISABLED".to_string()
            },
            key: "m",
            bool_state: Some(app.sidebar.sidebar_remotes),
            read_only: false,
        },
        GeneralOption {
            label: "Reset All Settings",
            status: "CONFIRM".to_string(),
            key: "!",
            bool_state: None,
            read_only: false,
        },
    ];

    let rows: Vec<_> = options
        .iter()
        .enumerate()
        .map(|(i, opt)| {
            let is_selected =
                i == app.settings.settings_index && app.settings.settings_section == SettingsSection::General;
            let mut style = Style::default().fg(theme::fg());
            let mut status_style = match opt.bool_state {
                Some(true) => Style::default().fg(theme::success()),
                Some(false) => Style::default().fg(theme::danger()),
                None => Style::default().fg(theme::info()),
            };

            let key_display = if opt.read_only || opt.key.is_empty() {
                "-".to_string()
            } else {
                opt.key.to_string()
            };
            if is_selected {
                style = style
                    .bg(theme::accent_primary())
                    .fg(theme::selection_fg())
                    .add_modifier(Modifier::BOLD);
                status_style = status_style
                    .bg(theme::accent_primary())
                    .fg(theme::selection_fg())
                    .add_modifier(Modifier::BOLD);
            }

            Row::new(vec![
                Cell::from(format!("  {}", opt.label)).style(style),
                Cell::from(format!(" [ {} ] ", opt.status)).style(status_style),
                Cell::from(format!("({})", key_display)).style(if is_selected {
                    style
                } else {
                    Style::default().fg(theme::muted())
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
            .border_style(Style::default().fg(theme::border_subtle())),
    )
    .column_spacing(2);

    f.render_widget(table, area);
}

pub fn draw_style_settings(f: &mut Frame, area: Rect, app: &App) {
    let style = theme::style_settings();
    const STYLE_PRESET_ROWS: usize = 11;
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
    let reset_selected = app.settings.settings_index == 0 && app.settings.settings_section == SettingsSection::Style;
    let reset_style = if reset_selected {
        Style::default()
            .bg(theme::accent_primary())
            .fg(theme::selection_fg())
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(theme::warning())
            .add_modifier(Modifier::BOLD)
    };
    rows.push(Row::new(vec![
        Cell::from("  Reset To Default Theme").style(reset_style),
        Cell::from("↺").style(reset_style),
        Cell::from("restore baseline").style(reset_style),
    ]));

    let preset_rows = [
        ("Warm", "amber + mint", theme::warning()),
        ("Cool", "violet + ice", theme::info()),
        ("Forest", "moss + pine", theme::success()),
        ("Sunset", "coral + plum", crate::ui::theme::ThemeStyle::preset_sunset().accent_primary.to_color()),
        ("Mono", "steel grayscale", crate::ui::theme::ThemeStyle::preset_mono().accent_primary.to_color()),
        ("Legacy Red", "classic red accent", theme::danger()),
        ("Nord", "frost blue + aurora", theme::accent_primary()),
        ("Dracula", "purple + neon green", theme::accent_primary()),
        ("Solarized Dark", "yellow + cyan", theme::warning()),
        ("One Dark", "purple + teal", theme::accent_primary()),
        ("Tokyo Night", "blue + purple", theme::info()),
    ];
    for (i, (name, desc, color)) in preset_rows.iter().enumerate() {
        let row_idx = i + 1;
        let is_selected =
            row_idx == app.settings.settings_index && app.settings.settings_section == SettingsSection::Style;
        let row_style = if is_selected {
            Style::default()
                .bg(theme::accent_primary())
                .fg(theme::selection_fg())
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
                    row_idx == app.settings.settings_index && app.settings.settings_section == SettingsSection::Style;
                let mut left_style = Style::default().fg(theme::fg());
                let mut value_style = Style::default().fg(Color::Rgb(rgb.r, rgb.g, rgb.b));
                if is_selected {
                    left_style = left_style
                        .bg(theme::accent_primary())
                        .fg(theme::selection_fg())
                        .add_modifier(Modifier::BOLD);
                    value_style = value_style
                        .bg(theme::accent_primary())
                        .fg(theme::selection_fg())
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
            .border_style(Style::default().fg(theme::border_subtle())),
    )
    .column_spacing(1);

    f.render_widget(table, area);
}
