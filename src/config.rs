use crate::app::{App, CurrentView, Pane, RemoteBookmark};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::LazyLock;
use parking_lot::Mutex;
use std::time::{Duration, Instant};

static LAST_SAVE: LazyLock<Mutex<Option<(Instant, String)>>> = LazyLock::new(|| Mutex::new(None));

// === Tiles Configuration Constants ===
// User-adjustable settings for behavior tuning

/// Maximum number of tabs per pane
pub const MAX_TABS: usize = 8;
/// Maximum depth for tree expansion (sidebar and file pane)
pub const MAX_TREE_DEPTH: u16 = 10;
/// Maximum number of recent folders to remember
pub const MAX_RECENT_FOLDERS: usize = 10;
/// Maximum navigation history per tab
pub const MAX_HISTORY: usize = 50;
/// Debounce interval for file watch events (milliseconds)
pub const FILE_WATCH_DEBOUNCE_MS: u64 = 200;
/// Debounce interval for auto-save (milliseconds)
pub const SAVE_DEBOUNCE_MS: u64 = 350;
/// Maximum preview file size (megabytes)
pub const PREVIEW_MAX_MB: u16 = 20;
pub const MPSC_CHANNEL_CAPACITY: usize = 1000;
pub const GIT_CACHE_TTL_SECONDS: u64 = 30;
pub const FUZZY_SEARCH: bool = false;

pub fn fuzzy_contains(text: &str, pattern: &str) -> bool {
    if pattern.is_empty() {
        return true;
    }
    let text_lower = text.to_lowercase();
    let pattern_lower = pattern.to_lowercase();
    let mut pattern_chars = pattern_lower.chars().peekable();
    for c in text_lower.chars() {
        if Some(&c) == pattern_chars.peek() {
            pattern_chars.next();
            if pattern_chars.peek().is_none() {
                return true;
            }
        }
    }
    false
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ExternalTool {
    pub name: String,
    pub command: String,
}

#[derive(Serialize, Deserialize)]
pub struct PersistentState {
    pub panes: Vec<Pane>,
    pub focused_pane_index: usize,
    pub starred: Vec<PathBuf>,
    pub remote_bookmarks: Vec<RemoteBookmark>,
    pub current_view: CurrentView,
    pub window_size: Option<(u16, u16)>,
    pub path_colors: HashMap<PathBuf, u8>,
    #[serde(default)]
    pub external_tools: HashMap<String, Vec<ExternalTool>>, // ext -> tools
    #[serde(default)]
    pub icon_mode: Option<crate::icons::IconMode>,
    #[serde(default)]
    pub is_split_mode: bool,
    #[serde(default = "default_true")]
    pub semantic_coloring: bool,
    #[serde(default = "default_true")]
    pub show_sidebar: bool,
    #[serde(default = "default_true")]
    pub sidebar_folders: bool,
    #[serde(default = "default_true")]
    pub sidebar_favorites: bool,
    #[serde(default = "default_true")]
    pub sidebar_recent: bool,
    #[serde(default = "default_true")]
    pub sidebar_storage: bool,
    #[serde(default = "default_true")]
    pub sidebar_remotes: bool,
    #[serde(default)]
    pub show_side_panel: bool,
    #[serde(default = "default_true")]
    pub default_show_hidden: bool,
    #[serde(default = "default_true")]
    pub auto_save: bool,
    #[serde(default = "default_preview_max_mb")]
    pub preview_max_mb: u16,
    #[serde(default)]
    pub theme_style: Option<crate::ui::theme::ThemeStyle>,
    #[serde(default)]
    pub expanded_folders: Vec<PathBuf>,
    #[serde(default)]
    pub sidebar_width_percent: u16,
    #[serde(default)]
    pub recent_folders: Vec<PathBuf>,
}

fn default_true() -> bool {
    true
}

fn default_preview_max_mb() -> u16 {
    20
}

pub fn save_state(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    let state = PersistentState {
        panes: {
            // We need to clone the panes but some fields are skipped by serde anyway
            // but we need to make sure we don't save ephemeral data if we can avoid it.
            // Actually Pane and FileState already have #[serde(skip)] on ephemeral fields.
            let mut panes = Vec::new();
            for p in &app.panes {
                let mut tabs = Vec::new();
                for t in &p.tabs {
                    let mut tab_clone = t.clone();
                    tab_clone.search_filter.clear();
                    tab_clone.files.clear();
                    tab_clone.local_count = 0;
                    tabs.push(tab_clone);
                }
                panes.push(Pane {
                    tabs,
                    active_tab_index: p.active_tab_index,
                });
            }
            panes
        },
        focused_pane_index: app.focused_pane_index,
        starred: app.starred.clone(),
        remote_bookmarks: app.remote_bookmarks.clone(),
        current_view: app.current_view.clone(),
        window_size: if app.terminal_size.0 > 0 && app.terminal_size.1 > 0 {
            Some(app.terminal_size)
        } else {
            None
        },
        path_colors: app.path_colors.clone(),
        external_tools: app.external_tools.clone(),
        icon_mode: Some(app.icon_mode),
        is_split_mode: app.is_split_mode,
        semantic_coloring: app.semantic_coloring,
        show_sidebar: app.show_sidebar,
        sidebar_folders: app.sidebar_folders,
        sidebar_favorites: app.sidebar_favorites,
        sidebar_recent: app.sidebar_recent,
        sidebar_storage: app.sidebar_storage,
        sidebar_remotes: app.sidebar_remotes,
        show_side_panel: app.show_side_panel,
        default_show_hidden: app.default_show_hidden,
        auto_save: app.auto_save,
        preview_max_mb: app.preview_max_mb,
        theme_style: Some(crate::ui::theme::style_settings()),
        expanded_folders: app.expanded_folders.iter().cloned().collect(),
        sidebar_width_percent: app.sidebar_width_percent,
        recent_folders: app.recent_folders.clone(),
    };

    let config_dir = dirs::config_dir()
        .ok_or("Could not find config dir")?
        .join("tiles");
    fs::create_dir_all(&config_dir)?;
    let state_path = config_dir.join("state.json");
    let json = serde_json::to_string_pretty(&state)?;

    {
        let mut last = LAST_SAVE.lock();
        let now = Instant::now();
        if let Some((last_at, last_json)) = last.as_ref() {
            // Avoid repeated writes of identical content.
            if *last_json == json {
                return Ok(());
            }
            // Debounce bursts while app is active/autosaving.
            if app.running
                && app.auto_save
                && now.duration_since(*last_at) < Duration::from_millis(SAVE_DEBOUNCE_MS)
            {
                return Ok(());
            }
        }
        *last = Some((now, json.clone()));
    }

    fs::write(state_path, json)?;
    Ok(())
}

pub fn save_state_quiet(app: &App) {
    if let Err(e) = save_state(app) {
        crate::app::log_debug(&format!("save_state failed: {}", e));
    }
}

pub fn load_state() -> Option<PersistentState> {
    let config_dir = dirs::config_dir()?.join("tiles");
    let state_path = config_dir.join("state.json");
    if !state_path.exists() {
        return None;
    }
    let json = fs::read_to_string(state_path).ok()?;
    serde_json::from_str(&json).ok()
}

#[derive(Debug)]
struct SshHostEntry {
    name: String,
    host: Option<String>,
    user: Option<String>,
    port: Option<u16>,
    key_path: Option<PathBuf>,
}

fn parse_ssh_config() -> Vec<RemoteBookmark> {
    let ssh_config_path = match dirs::home_dir() {
        Some(home) => home.join(".ssh").join("config"),
        None => return Vec::new(),
    };
    if !ssh_config_path.exists() {
        return Vec::new();
    }
    let content = match fs::read_to_string(&ssh_config_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut results = Vec::new();
    let mut current_entry: Option<SshHostEntry> = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(stripped) = line.strip_prefix("Host ") {
            if let Some(entry) = current_entry.take() {
                if entry.host.is_some() && entry.user.is_some() {
                    let host = entry.host.unwrap();
                    if !host.contains("github.com") && !host.contains("codeberg.org") {
                        results.push(RemoteBookmark {
                            name: entry.name,
                            host,
                            user: entry.user.unwrap(),
                            port: entry.port.unwrap_or(22),
                            last_path: PathBuf::from("/"),
                            key_path: entry.key_path,
                        });
                    }
                }
            }
            let name = stripped.trim().to_string();
            current_entry = Some(SshHostEntry {
                name,
                host: None,
                user: None,
                port: None,
                key_path: None,
            });
        } else if let Some(current) = current_entry.as_mut() {
            if let Some(value) = line.strip_prefix("HostName ") {
                current.host = Some(value.trim().to_string());
            } else if let Some(value) = line.strip_prefix("User ") {
                current.user = Some(value.trim().to_string());
            } else if let Some(value) = line.strip_prefix("Port ") {
                current.port = Some(value.trim().parse().unwrap_or(22));
            } else if let Some(value) = line.strip_prefix("IdentityFile ") {
                let path = value.trim().replace('~', &dirs::home_dir().unwrap_or_default().to_string_lossy());
                current.key_path = Some(PathBuf::from(path));
            }
        }
    }

    if let Some(entry) = current_entry {
        if entry.host.is_some() && entry.user.is_some() {
            let host = entry.host.unwrap();
            if !host.contains("github.com") && !host.contains("codeberg.org") {
                results.push(RemoteBookmark {
                    name: entry.name,
                    host,
                    user: entry.user.unwrap(),
                    port: entry.port.unwrap_or(22),
                    last_path: PathBuf::from("/"),
                    key_path: entry.key_path,
                });
            }
        }
    }

    results
}

pub fn merge_ssh_config_bookmarks(bookmarks: &mut Vec<RemoteBookmark>) {
    let ssh_bookmarks = parse_ssh_config();
    for sb in ssh_bookmarks {
        if !bookmarks.iter().any(|b| b.host == sb.host && b.user == sb.user) {
            bookmarks.push(sb);
        }
    }
}
