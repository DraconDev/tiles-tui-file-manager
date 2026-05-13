use dracon_terminal_engine::contracts::{KeyCode, KeyModifiers};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Represents a parsed key combination for event matching.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyCombo {
    pub code: String,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

impl KeyCombo {
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('+').map(|p| p.trim()).collect();
        let mut ctrl = false;
        let mut alt = false;
        let mut shift = false;
        let mut code = String::new();

        for part in &parts {
            match part.to_ascii_lowercase().as_str() {
                "ctrl" | "control" | "c" => ctrl = true,
                "alt" | "a" => alt = true,
                "shift" | "s" => shift = true,
                c => code = c.to_string(),
            }
        }

        if code.is_empty() {
            return None;
        }

        Some(Self {
            code,
            ctrl,
            alt,
            shift,
        })
    }

    pub fn matches(&self, code: &KeyCode, modifiers: &KeyModifiers) -> bool {
        let ctrl = modifiers.contains(KeyModifiers::CONTROL);
        let alt = modifiers.contains(KeyModifiers::ALT);
        let shift = modifiers.contains(KeyModifiers::SHIFT);

        if self.ctrl != ctrl || self.alt != alt || self.shift != shift {
            return false;
        }

        match code {
            KeyCode::Char(c) => {
                self.code.len() == 1
                    && self.code.chars().next().unwrap().to_ascii_lowercase() == c.to_ascii_lowercase()
            }
            KeyCode::Up => self.code.eq_ignore_ascii_case("up"),
            KeyCode::Down => self.code.eq_ignore_ascii_case("down"),
            KeyCode::Left => self.code.eq_ignore_ascii_case("left"),
            KeyCode::Right => self.code.eq_ignore_ascii_case("right"),
            KeyCode::Enter => self.code.eq_ignore_ascii_case("enter") || self.code.eq_ignore_ascii_case("return"),
            KeyCode::Esc => self.code.eq_ignore_ascii_case("esc") || self.code.eq_ignore_ascii_case("escape"),
            KeyCode::Backspace => self.code.eq_ignore_ascii_case("backspace"),
            KeyCode::Tab => self.code.eq_ignore_ascii_case("tab"),
            KeyCode::Delete => self.code.eq_ignore_ascii_case("delete") || self.code.eq_ignore_ascii_case("del"),
            KeyCode::Home => self.code.eq_ignore_ascii_case("home"),
            KeyCode::End => self.code.eq_ignore_ascii_case("end"),
            KeyCode::PageUp => self.code.eq_ignore_ascii_case("pageup") || self.code.eq_ignore_ascii_case("page_up"),
            KeyCode::PageDown => self.code.eq_ignore_ascii_case("pagedown") || self.code.eq_ignore_ascii_case("page_down"),
            KeyCode::Insert => self.code.eq_ignore_ascii_case("insert"),
            KeyCode::F(n) => {
                self.code.eq_ignore_ascii_case(&format!("f{}", n))
            }
            _ => false,
        }
    }
}

/// Named actions that can be bound to keys.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyAction {
    Quit,
    ToggleHidden,
    Properties,
    Settings,
    NewTab,
    CloseTab,
    Copy,
    Cut,
    Paste,
    Search,
    CommandPalette,
    Undo,
    Redo,
    SelectAll,
    Delete,
    Rename,
    NewFolder,
    Up,
    Down,
    Left,
    Right,
    Enter,
    Back,
    TogglePreview,
    ToggleZoom,
    NextPane,
    PrevPane,
    ToggleSidebar,
    SidebarUp,
    SidebarDown,
    SidebarEnter,
    ToggleMultiSelect,
    Star,
    SortToggle,
    QuickFilter,
}

impl KeyAction {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "quit" => Some(Self::Quit),
            "toggle_hidden" => Some(Self::ToggleHidden),
            "properties" => Some(Self::Properties),
            "settings" => Some(Self::Settings),
            "new_tab" => Some(Self::NewTab),
            "close_tab" => Some(Self::CloseTab),
            "copy" => Some(Self::Copy),
            "cut" => Some(Self::Cut),
            "paste" => Some(Self::Paste),
            "search" => Some(Self::Search),
            "command_palette" => Some(Self::CommandPalette),
            "undo" => Some(Self::Undo),
            "redo" => Some(Self::Redo),
            "select_all" => Some(Self::SelectAll),
            "delete" => Some(Self::Delete),
            "rename" => Some(Self::Rename),
            "new_folder" => Some(Self::NewFolder),
            "up" => Some(Self::Up),
            "down" => Some(Self::Down),
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            "enter" => Some(Self::Enter),
            "back" => Some(Self::Back),
            "toggle_preview" => Some(Self::TogglePreview),
            "toggle_zoom" => Some(Self::ToggleZoom),
            "next_pane" => Some(Self::NextPane),
            "prev_pane" => Some(Self::PrevPane),
            "toggle_sidebar" => Some(Self::ToggleSidebar),
            "sidebar_up" => Some(Self::SidebarUp),
            "sidebar_down" => Some(Self::SidebarDown),
            "sidebar_enter" => Some(Self::SidebarEnter),
            "toggle_multi_select" => Some(Self::ToggleMultiSelect),
            "star" => Some(Self::Star),
            "sort_toggle" => Some(Self::SortToggle),
            "quick_filter" => Some(Self::QuickFilter),
            _ => None,
        }
    }
}

/// User-configurable keybindings loaded from `~/.config/tiles/keybindings.toml`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Keybindings {
    #[serde(default)]
    pub bindings: HashMap<String, String>,
}

impl Default for Keybindings {
    fn default() -> Self {
        let mut bindings = HashMap::new();
        bindings.insert("quit".to_string(), "q".to_string());
        bindings.insert("toggle_hidden".to_string(), "h".to_string());
        bindings.insert("properties".to_string(), "i".to_string());
        bindings.insert("settings".to_string(), ",".to_string());
        bindings.insert("new_tab".to_string(), "t".to_string());
        bindings.insert("close_tab".to_string(), "w".to_string());
        bindings.insert("copy".to_string(), "y".to_string());
        bindings.insert("cut".to_string(), "x".to_string());
        bindings.insert("paste".to_string(), "p".to_string());
        bindings.insert("search".to_string(), "/".to_string());
        bindings.insert("command_palette".to_string(), "Ctrl+Shift+P".to_string());
        bindings.insert("undo".to_string(), "u".to_string());
        bindings.insert("redo".to_string(), "Ctrl+r".to_string());
        bindings.insert("select_all".to_string(), "a".to_string());
        bindings.insert("delete".to_string(), "d".to_string());
        bindings.insert("rename".to_string(), "r".to_string());
        bindings.insert("new_folder".to_string(), "n".to_string());
        bindings.insert("up".to_string(), "up".to_string());
        bindings.insert("down".to_string(), "down".to_string());
        bindings.insert("left".to_string(), "left".to_string());
        bindings.insert("right".to_string(), "right".to_string());
        bindings.insert("enter".to_string(), "enter".to_string());
        bindings.insert("back".to_string(), "backspace".to_string());
        bindings.insert("toggle_preview".to_string(), ".".to_string());
        bindings.insert("toggle_zoom".to_string(), "z".to_string());
        bindings.insert("next_pane".to_string(), "tab".to_string());
        bindings.insert("prev_pane".to_string(), "Shift+Tab".to_string());
        bindings.insert("toggle_sidebar".to_string(), "b".to_string());
        bindings.insert("sidebar_up".to_string(), "Ctrl+Up".to_string());
        bindings.insert("sidebar_down".to_string(), "Ctrl+Down".to_string());
        bindings.insert("sidebar_enter".to_string(), "Ctrl+Enter".to_string());
        bindings.insert("toggle_multi_select".to_string(), "space".to_string());
        bindings.insert("star".to_string(), "*".to_string());
        bindings.insert("sort_toggle".to_string(), "s".to_string());
        bindings.insert("quick_filter".to_string(), "Ctrl+f".to_string());
        Self { bindings }
    }
}

impl Keybindings {
    pub fn load() -> Self {
        let path = dirs::config_dir()
            .map(|d| d.join("tiles/keybindings.toml"))
            .unwrap_or_else(|| Path::new("/tmp/tiles_keybindings.toml").to_path_buf());

        if !path.exists() {
            return Self::default();
        }

        match std::fs::read_to_string(&path) {
            Ok(contents) => match toml::from_str(&contents) {
                Ok(kb) => kb,
                Err(e) => {
                    eprintln!("[tiles] Warning: failed to parse keybindings.toml: {}", e);
                    Self::default()
                }
            },
            Err(e) => {
                eprintln!("[tiles] Warning: failed to read keybindings.toml: {}", e);
                Self::default()
            }
        }
    }

    /// Look up which action (if any) is bound to the given key event.
    pub fn lookup(&self, code: &KeyCode, modifiers: &KeyModifiers) -> Option<KeyAction> {
        for (action_str, combo_str) in &self.bindings {
            if let Some(combo) = KeyCombo::parse(combo_str) {
                if combo.matches(code, modifiers) {
                    return KeyAction::from_str(action_str);
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_char() {
        let c = KeyCombo::parse("q").unwrap();
        assert_eq!(c.code, "q");
        assert!(!c.ctrl);
    }

    #[test]
    fn parse_ctrl_combo() {
        let c = KeyCombo::parse("Ctrl+h").unwrap();
        assert_eq!(c.code, "h");
        assert!(c.ctrl);
        assert!(!c.alt);
    }

    #[test]
    fn parse_alt_shift() {
        let c = KeyCombo::parse("Alt+Shift+Enter").unwrap();
        assert_eq!(c.code, "enter");
        assert!(c.alt);
        assert!(c.shift);
    }

    #[test]
    fn parse_function_key() {
        let c = KeyCombo::parse("F5").unwrap();
        assert_eq!(c.code, "f5");
    }

    #[test]
    fn matches_char_case_insensitive() {
        let c = KeyCombo::parse("Ctrl+Q").unwrap();
        assert!(c.matches(&KeyCode::Char('q'), &KeyModifiers::CONTROL));
        assert!(c.matches(&KeyCode::Char('Q'), &KeyModifiers::CONTROL));
    }
}
