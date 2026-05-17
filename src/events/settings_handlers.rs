#![allow(unused_imports)]

//! Settings modal key handlers — style color, reset, preview MB cycling.
//! Extracted from events/modals.rs (Phase 4).

use dracon_terminal_engine::contracts::{InputEvent as Event, KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, AppEvent, AppMode, CurrentView, SettingsSection};
use crate::state::IconMode;
use crate::ui::theme as theme;
use tokio::sync::mpsc;

const STYLE_PRESET_COUNT: usize = 11;
const STYLE_COLOR_FIELD_COUNT: usize = 6;
const STYLE_COLOR_START_INDEX: usize = 1 + STYLE_PRESET_COUNT;
pub const STYLE_MAX_INDEX: usize = STYLE_COLOR_START_INDEX + STYLE_COLOR_FIELD_COUNT - 1;

pub fn cycle_preview_max_mb(current: u16) -> u16 {
    match current {
        5 => 10,
        10 => 20,
        20 => 50,
        50 => 100,
        _ => 5,
    }
}

pub fn open_style_color_input(app: &mut App) {
    if app.settings.settings_index < STYLE_COLOR_START_INDEX {
        return;
    }
    let style = crate::ui::theme::style_settings();
    let color = style_field_color(app.settings.settings_index, &style);
    app.core.input.value = format!("#{:02X}{:02X}{:02X}", color.r, color.g, color.b);
    app.core.input.cursor_position = app.core.input.value.len();
    app.core.mode = AppMode::StyleColorInput;
}

pub fn style_preset_for_index(index: usize) -> Option<crate::ui::theme::ThemeStyle> {
    match index {
        1 => Some(crate::ui::theme::ThemeStyle::preset_warm()),
        2 => Some(crate::ui::theme::ThemeStyle::preset_cool()),
        3 => Some(crate::ui::theme::ThemeStyle::preset_forest()),
        4 => Some(crate::ui::theme::ThemeStyle::preset_sunset()),
        5 => Some(crate::ui::theme::ThemeStyle::preset_mono()),
        6 => Some(crate::ui::theme::ThemeStyle::default_purple()),
        7 => Some(crate::ui::theme::ThemeStyle::preset_nord()),
        8 => Some(crate::ui::theme::ThemeStyle::preset_dracula()),
        9 => Some(crate::ui::theme::ThemeStyle::preset_solarized_dark()),
        10 => Some(crate::ui::theme::ThemeStyle::preset_one_dark()),
        11 => Some(crate::ui::theme::ThemeStyle::preset_tokyo_night()),
        _ => None,
    }
}

pub fn style_field_name(index: usize) -> &'static str {
    let idx = index.saturating_sub(STYLE_COLOR_START_INDEX);
    match idx {
        0 => "accent_primary",
        1 => "accent_secondary",
        2 => "selection_bg",
        3 => "border_active",
        4 => "border_inactive",
        5 => "header_fg",
        _ => "accent_primary",
    }
}

pub fn style_field_color(
    index: usize,
    style: &crate::ui::theme::ThemeStyle,
) -> crate::ui::theme::RgbColor {
    let idx = index.saturating_sub(STYLE_COLOR_START_INDEX);
    match idx {
        0 => style.accent_primary,
        1 => style.accent_secondary,
        2 => style.selection_bg,
        3 => style.border_active,
        4 => style.border_inactive,
        5 => style.header_fg,
        _ => style.accent_primary,
    }
}

pub fn set_style_field_color(
    index: usize,
    style: &mut crate::ui::theme::ThemeStyle,
    color: crate::ui::theme::RgbColor,
) {
    let idx = index.saturating_sub(STYLE_COLOR_START_INDEX);
    match idx {
        0 => style.accent_primary = color,
        1 => style.accent_secondary = color,
        2 => style.selection_bg = color,
        3 => style.border_active = color,
        4 => style.border_inactive = color,
        5 => style.header_fg = color,
        _ => {}
    }
}

pub fn parse_style_color_input(input: &str) -> Option<crate::ui::theme::RgbColor> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    let hex = trimmed.trim_start_matches('#');
    if hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        return Some(crate::ui::theme::RgbColor::new(r, g, b));
    }

    let parts: Vec<&str> = trimmed.split(',').map(|p| p.trim()).collect();
    if parts.len() == 3 {
        let r = parts[0].parse::<u8>().ok()?;
        let g = parts[1].parse::<u8>().ok()?;
        let b = parts[2].parse::<u8>().ok()?;
        return Some(crate::ui::theme::RgbColor::new(r, g, b));
    }

    None
}

pub fn handle_style_color_input_keys(key: &dracon_terminal_engine::contracts::KeyEvent, app: &mut App) -> bool {
    match key.code {
        KeyCode::Esc => {
            app.core.mode = AppMode::Settings;
            app.core.input.clear();
            true
        }
        KeyCode::Enter => {
            if let Some(color) = parse_style_color_input(&app.core.input.value) {
                let mut style = crate::ui::theme::style_settings();
                set_style_field_color(app.settings.settings_index, &mut style, color);
                crate::ui::theme::set_style_settings(style);
                crate::config::save_state_quiet(app);
                app.core.mode = AppMode::Settings;
                app.core.input.clear();
            } else {
                app.output.last_action_msg = Some((
                    format!(
                        "Invalid color for {}. Use #RRGGBB or R,G,B",
                        style_field_name(app.settings.settings_index)
                    ),
                    std::time::Instant::now(),
                ));
            }
            true
        }
        _ => app.core.input
            .handle_event(&dracon_terminal_engine::input::mapping::to_runtime_event(&Event::Key(*key))),
    }
}

pub fn reset_all_settings_to_defaults(app: &mut App) {
    app.settings.confirm_delete = true;
    app.settings.smart_date = true;
    app.settings.semantic_coloring = true;
    app.settings.auto_save = true;
    app.settings.default_show_hidden = false;
    app.preview_max_mb = 20;
    app.core.icon_mode = crate::state::IconMode::Nerd;
    app.sidebar.show_sidebar = true;
    app.sidebar.show_side_panel = true;
    app.show_main_stage = true;
    app.sidebar.sidebar_width_percent = 15;
    app.layout.single_columns = vec![
        crate::app::FileColumn::Name,
        crate::app::FileColumn::Size,
        crate::app::FileColumn::Modified,
        crate::app::FileColumn::Permissions,
    ];
    app.layout.split_columns = vec![crate::app::FileColumn::Name, crate::app::FileColumn::Size];
    app.nav.view_prefs.files.show_sidebar = true;
    app.nav.view_prefs.files.is_split_mode = false;
    app.nav.view_prefs.editor.show_sidebar = true;
    app.nav.view_prefs.editor.is_split_mode = false;
    app.settings.settings_section = SettingsSection::General;
    app.settings.settings_index = 0;
    crate::ui::theme::set_style_settings(crate::ui::theme::ThemeStyle::default_purple());

    for pane in &mut app.panes {
        for tab in &mut pane.tabs {
            tab.nav.show_hidden = app.settings.default_show_hidden;
        }
    }
}

pub fn handle_reset_settings_confirm_keys(key: &dracon_terminal_engine::contracts::KeyEvent, app: &mut App) -> bool {
    match key.code {
        KeyCode::Esc => {
            app.core.mode = AppMode::Settings;
            app.core.input.clear();
            true
        }
        KeyCode::Enter => {
            if app.core.input.value.trim().eq_ignore_ascii_case("RESET") {
                reset_all_settings_to_defaults(app);
                crate::config::save_state_quiet(app);
                app.output.last_action_msg = Some((
                    "Settings reset to defaults".to_string(),
                    std::time::Instant::now(),
                ));
                app.core.mode = AppMode::Settings;
            } else {
                app.output.last_action_msg = Some((
                    "Type RESET to confirm".to_string(),
                    std::time::Instant::now(),
                ));
            }
            app.core.input.clear();
            true
        }
        _ => app.core.input
            .handle_event(&dracon_terminal_engine::input::mapping::to_runtime_event(&Event::Key(*key))),
    }
}
