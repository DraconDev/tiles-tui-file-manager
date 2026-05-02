use chrono::{DateTime, Local};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::SystemTime;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum IconMode {
    Nerd,
    Unicode,
    ASCII,
}

pub fn guess_icon_mode() -> IconMode {
    let term = std::env::var("TERM").unwrap_or_default().to_lowercase();
    let term_program = std::env::var("TERM_PROGRAM")
        .unwrap_or_default()
        .to_lowercase();

    if term.contains("kitty")
        || term.contains("alacritty")
        || term.contains("wezterm")
        || term.contains("konsole")
        || term.contains("foot")
        || term.contains("tmux")
        || term.contains("nerd")
        || term_program.contains("vscode")
        || term_program.contains("iterm")
        || term_program.contains("warp")
        || std::env::var("TERMINAL_EMULATOR")
            .map(|s| s.to_lowercase().contains("jetbrains"))
            .unwrap_or(false)
        || std::env::var("KONSOLE_VERSION").is_ok()
    {
        return IconMode::Nerd;
    }

    if std::env::var("COLORTERM").is_ok() {
        return IconMode::Unicode;
    }

    IconMode::ASCII
}

/// Returns true if the current terminal supports the Kitty Graphics Protocol.
///
/// Only certain terminals (Kitty, WezTerm, Ghostty) support this protocol.
/// Others like Konsole, gnome-terminal, and Alacritty do NOT.
pub fn supports_kitty_graphics() -> bool {
    let term = std::env::var("TERM").unwrap_or_default().to_lowercase();
    let term_program = std::env::var("TERM_PROGRAM")
        .unwrap_or_default()
        .to_lowercase();

    // Kitty sets TERM=xterm-kitty
    if term.contains("kitty") {
        return true;
    }

    // WezTerm sets TERM_PROGRAM=WezTerm
    if term_program.contains("wezterm") {
        return true;
    }

    // Ghostty sets TERM_PROGRAM=ghostty (when it matures)
    if term_program.contains("ghostty") {
        return true;
    }

    // Check for the KITTY_WINDOW_ID env var (definitive proof)
    if std::env::var("KITTY_WINDOW_ID").is_ok() {
        return true;
    }

    false
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum FileColumn {
    Name,
    Size,
    Modified,
    Created,
    Permissions,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SelectionState {
    pub selected: Option<usize>,
    pub anchor: Option<usize>,
    pub multi: HashSet<usize>,
}

impl SelectionState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.multi.clear();
        self.selected = None;
        self.anchor = None;
    }

    pub fn clear_multi(&mut self) {
        self.multi.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.multi.is_empty()
    }

    pub fn multi_selected_indices(&self) -> &HashSet<usize> {
        &self.multi
    }

    pub fn add(&mut self, idx: usize) {
        self.multi.insert(idx);
    }

    pub fn select_all(&mut self, len: usize) {
        self.multi = (0..len).collect();
    }

    pub fn handle_click(&mut self, idx: usize, is_shift: bool, is_ctrl: bool, is_sticky: bool) {
        if is_ctrl || is_sticky {
            // Ensure the primary selection is part of multi before we start toggling others
            if let Some(s) = self.selected {
                self.multi.insert(s);
            }

            if self.multi.contains(&idx) {
                self.multi.remove(&idx);
            } else {
                self.multi.insert(idx);
            }
            self.selected = Some(idx);
            self.anchor = Some(idx);
        } else if is_shift {
            let anchor = self.anchor.unwrap_or(self.selected.unwrap_or(0));
            self.anchor = Some(anchor);
            self.multi.clear();
            for i in std::cmp::min(anchor, idx)..=std::cmp::max(anchor, idx) {
                self.multi.insert(i);
            }
            self.selected = Some(idx);
        } else {
            self.multi.clear();
            self.multi.insert(idx);
            self.selected = Some(idx);
            self.anchor = Some(idx);
        }
    }

    pub fn handle_move(&mut self, next: usize, is_shift: bool) {
        let prev = self.selected;
        self.selected = Some(next);
        if is_shift {
            let anchor = self.anchor.unwrap_or(prev.unwrap_or(0));
            self.anchor = Some(anchor);
            self.multi.clear();
            for i in std::cmp::min(anchor, next)..=std::cmp::max(anchor, next) {
                self.multi.insert(i);
            }
        } else {
            self.multi.clear();
            self.anchor = Some(next);
        }
    }

    pub fn toggle(&mut self, idx: usize) {
        if self.multi.contains(&idx) {
            self.multi.remove(&idx);
        } else {
            self.multi.insert(idx);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum FileCategory {
    Archive,
    Image,
    Script,
    Text,
    Document,
    Audio,
    Video,
    Other,
}

impl FileCategory {
    pub fn cyber_color(&self) -> Color {
        match self {
            FileCategory::Archive => Color::Rgb(255, 50, 80), // Neon Red
            FileCategory::Image => Color::Rgb(255, 0, 255),   // Magenta
            FileCategory::Script => Color::Rgb(0, 255, 100),  // Matrix Green
            FileCategory::Text => Color::Rgb(255, 215, 0),    // Gold
            FileCategory::Document => Color::Rgb(100, 200, 255), // Light Blue
            FileCategory::Audio => Color::Rgb(0, 150, 255),   // Electric Blue
            FileCategory::Video => Color::Rgb(180, 50, 255),  // Neon Purple
            FileCategory::Other => Color::Rgb(255, 255, 255), // Pure White
        }
    }
}

pub fn get_file_category(path: &std::path::Path) -> FileCategory {
    let filename = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Special cases for filenames without extensions or specific dotfiles
    match filename.as_str() {
        "license" | "dockerfile" | "makefile" | "makefile.am" | "makefile.in" | "flake.nix"
        | "flake.lock" => return FileCategory::Text,
        ".gitignore" | ".gitattributes" | ".gitconfig" | ".env" | ".dockerignore"
        | ".geminiignore" | ".directory" => return FileCategory::Text,
        _ => {}
    }

    match ext.as_str() {
        "zip" | "tar" | "gz" | "7z" | "rar" | "xz" | "bz2" => FileCategory::Archive,
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp" => FileCategory::Image,
        "sh" | "py" | "pyw" | "rb" | "js" | "ts" | "pl" | "php" | "nix" => FileCategory::Script,
        "txt" | "md" | "rs" | "c" | "cpp" | "h" | "hpp" | "toml" | "yaml" | "yml" | "json"
        | "xml" | "html" | "css" | "conf" | "config" | "log" | "lock" | "env" | "gradle"
        | "properties" => FileCategory::Text,
        "pdf" | "doc" | "docx" | "odt" | "ods" | "odp" | "xlsx" | "xls" | "csv" => {
            FileCategory::Document
        }
        "mp3" | "wav" | "ogg" | "flac" | "m4a" | "aac" => FileCategory::Audio,
        "mp4" | "mkv" | "avi" | "mov" | "webm" | "flv" => FileCategory::Video,
        _ => FileCategory::Other,
    }
}

pub fn get_open_with_suggestions(ext: &str) -> Vec<String> {
    match ext {
        "txt" | "md" | "rs" | "toml" | "json" | "c" | "cpp" | "py" | "js" | "ts" | "log"
        | "conf" | "yaml" | "yml" | "lock" | "env" | "gradle" | "properties" => {
            vec![
                "code", "vim", "nvim", "nano", "kate", "subl", "gedit", "emacs", "mousepad",
                "leafpad", "xed",
            ]
        }
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp" | "ico" => {
            vec![
                "gwenview",
                "feh",
                "imv",
                "nomacs",
                "display",
                "eog",
                "ristretto",
                "gimp",
                "inkscape",
                "krita",
            ]
        }
        "pdf" | "epub" | "djvu" => vec![
            "okular", "evince", "zathura", "firefox", "chromium", "atril", "mupdf", "xreader",
        ],
        "mp4" | "mkv" | "avi" | "mov" | "webm" | "flv" | "m4v" => vec![
            "vlc",
            "mpv",
            "totem",
            "smplayer",
            "dragon",
            "celluloid",
            "kmplayer",
        ],
        "mp3" | "wav" | "ogg" | "flac" | "m4a" | "aac" => vec![
            "vlc",
            "clementine",
            "audacious",
            "rhythmbox",
            "strawberry",
            "lollypop",
            "sayonara",
        ],
        "zip" | "tar" | "gz" | "7z" | "rar" | "bz2" | "xz" => {
            vec!["ark", "file-roller", "engrampa", "xarchiver", "peazip"]
        }
        _ => vec![
            "xdg-open", "dolphin", "nautilus", "thunar", "pcmanfm", "code", "vim", "nvim",
        ],
    }
    .into_iter()
    .map(|s| s.to_string())
    .collect()
}

use unicode_width::UnicodeWidthChar;

/// Returns the visual width of a character, with high paranoia for any Non-ASCII characters.
pub fn get_visual_width(c: char) -> usize {
    if c.is_ascii() {
        return 1;
    }
    UnicodeWidthChar::width(c).unwrap_or(1)
}

pub fn squarify(s: &str) -> String {
    s.chars().filter(|c| !c.is_control()).collect()
}

/// Truncates a string to fit a visual width, adding an optional suffix if truncated.
pub fn truncate_to_width(s: &str, max_width: usize, suffix: &str) -> String {
    let mut total_width = 0;
    for c in s.chars() {
        total_width += get_visual_width(c);
    }

    if total_width <= max_width {
        return s.to_string();
    }

    let suffix_width = suffix.chars().map(get_visual_width).sum::<usize>();
    if max_width <= suffix_width {
        return ".".to_string();
    }

    let mut truncated = String::new();
    let mut cur_width = 0;
    for c in s.chars() {
        let cw = get_visual_width(c);
        if cur_width + cw + suffix_width <= max_width {
            truncated.push(c);
            cur_width += cw;
        } else {
            break;
        }
    }
    format!("{}{}", truncated, suffix)
}

pub fn command_exists(cmd: &str) -> bool {
    if let Ok(path) = std::env::var("PATH") {
        for p in path.split(':') {
            let p_str = format!("{}/{}", p, cmd);
            if std::path::Path::new(&p_str).exists() {
                return true;
            }
        }
    }
    false
}

pub fn spawn_detached(cmd: &str, args: Vec<String>) {
    let _ = std::process::Command::new(cmd)
        .args(args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .stdin(std::process::Stdio::null())
        .spawn();
}

pub fn format_size(size: u64) -> String {
    if size >= 1073741824 {
        format!("{:.1} GB", size as f64 / 1073741824.0)
    } else if size >= 1048576 {
        format!("{:.1} MB", size as f64 / 1048576.0)
    } else if size >= 1024 {
        format!("{:.1} KB", size as f64 / 1024.0)
    } else {
        format!("{} B", size)
    }
}

pub fn format_time(time: SystemTime) -> String {
    let datetime: DateTime<Local> = time.into();
    datetime.format("%Y-%m-%d %H:%M").to_string()
}

pub fn format_datetime_smart(time: SystemTime) -> String {
    use chrono::Datelike;
    let dt: DateTime<Local> = time.into();
    let now = Local::now();
    if dt.year() == now.year() && dt.month() == now.month() && dt.day() == now.day() {
        dt.format("%H:%M").to_string()
    } else {
        dt.format("%Y-%m-%d").to_string()
    }
}

pub fn format_permissions(mode: u32) -> String {
    let r = |b| if b & 4 != 0 { "r" } else { "-" };
    let w = |b| if b & 2 != 0 { "w" } else { "-" };
    let x = |b| if b & 1 != 0 { "x" } else { "-" };
    format!(
        "{}{}{}{}{}{}{}{}{}",
        r((mode >> 6) & 0o7),
        w((mode >> 6) & 0o7),
        x((mode >> 6) & 0o7),
        r((mode >> 3) & 0o7),
        w((mode >> 3) & 0o7),
        x((mode >> 3) & 0o7),
        r(mode & 0o7),
        w(mode & 0o7),
        x(mode & 0o7)
    )
}

use std::sync::OnceLock;
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static THEME_SET: OnceLock<ThemeSet> = OnceLock::new();

pub fn highlight_code<'a>(content: &'a str, extension: &str) -> Vec<Line<'a>> {
    let ps = SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines);
    let ts = THEME_SET.get_or_init(ThemeSet::load_defaults);

    let ext_lower = extension.to_lowercase();

    // 1. Try by extension
    let mut syntax = ps.find_syntax_by_extension(&ext_lower);

    // 2. Try by name (case-insensitive-ish)
    if syntax.is_none() {
        syntax = ps
            .find_syntax_by_name(extension)
            .or_else(|| ps.find_syntax_by_name(&ext_lower));
    }

    // If not found, try common mappings
    if syntax.is_none() {
        syntax = match ext_lower.as_str() {
            "makefile" | "make" => ps.find_syntax_by_extension("make"),
            "dockerfile" | "dockerignore" => ps
                .find_syntax_by_extension("dockerfile")
                .or_else(|| ps.find_syntax_by_extension("sh")),
            "cargo.toml" | "cargo" | "toml" | "lock" | "ini" | "inf" => ps
                .find_syntax_by_extension("toml")
                .or_else(|| ps.find_syntax_by_name("Ini")),
            "flake.lock" | "json" | "jsonc" | "ipynb" => ps.find_syntax_by_extension("json"),
            "xml" | "svg" | "plist" | "xaml" | "csproj" | "fsproj" | "vbproj" | "pom"
            | "pom.xml" | "xsd" | "xsl" => ps.find_syntax_by_extension("xml"),
            "nix" | "flake.nix" | "configuration.nix" | "home.nix" | "default.nix" => {
                ps.find_syntax_by_extension("nix")
                    .or_else(|| ps.find_syntax_by_name("Nix"))
                    .or_else(|| ps.find_syntax_by_extension("rb")) // Desperate fallback to Ruby
            }
            "yaml" | "yml" => ps.find_syntax_by_extension("yaml"),
            "gitignore" | "gitattributes" | "gitconfig" | "conf" | "config" | "env" | ".env"
            | "properties" | "prefs" => ps.find_syntax_by_extension("sh"),
            "ts" | "tsx" | "typescript" => ps.find_syntax_by_extension("ts"),
            "js" | "jsx" | "javascript" => ps.find_syntax_by_extension("js"),
            "go" | "golang" => ps.find_syntax_by_extension("go"),
            "sql" | "mysql" | "psql" => ps.find_syntax_by_extension("sql"),
            "md" | "markdown" | "rmd" => ps.find_syntax_by_extension("md"),
            "html" | "htm" | "xhtml" => ps.find_syntax_by_extension("html"),
            "css" | "scss" | "sass" | "less" => ps.find_syntax_by_extension("css"),
            "sh" | "bash" | "zsh" | "fish" | "command" | "bashrc" | "zshrc" | "profile" => {
                ps.find_syntax_by_extension("sh")
            }
            "py" | "python" | "pyw" | "cgi" => ps.find_syntax_by_extension("py"),
            "rs" | "rust" => ps.find_syntax_by_extension("rs"),
            "c" | "h" => ps.find_syntax_by_extension("c"),
            "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" => ps.find_syntax_by_extension("cpp"),
            "cs" | "csharp" => ps.find_syntax_by_extension("cs"),
            "java" | "jsp" => ps.find_syntax_by_extension("java"),
            "kt" | "kotlin" | "kts" => ps.find_syntax_by_extension("kotlin"),
            "swift" => ps.find_syntax_by_extension("swift"),
            "php" | "phtml" | "php4" | "php5" => ps.find_syntax_by_extension("php"),
            "rb" | "ruby" | "gemspec" | "rakefile" => ps.find_syntax_by_extension("rb"),
            "pl" | "perl" | "pm" | "t" => ps.find_syntax_by_extension("pl"),
            "lua" => ps.find_syntax_by_extension("lua"),
            "gradle" => ps
                .find_syntax_by_extension("groovy")
                .or_else(|| ps.find_syntax_by_extension("java")),
            "diff" | "patch" => ps.find_syntax_by_extension("diff"),
            _ => None,
        };
    }

    // 4. Try by first line (shebang)
    if syntax.is_none() {
        if let Some(first_line) = content.lines().next() {
            syntax = ps.find_syntax_by_first_line(first_line);
        }
    }

    let syntax = syntax.unwrap_or_else(|| ps.find_syntax_plain_text());

    let mut h = HighlightLines::new(syntax, &ts.themes["base16-mocha.dark"]);

    let mut lines = Vec::new();
    let is_markdown = syntax.name.contains("Markdown");

    for line in LinesWithEndings::from(content) {
        let ranges: Vec<(syntect::highlighting::Style, &str)> = h.highlight_line(line, ps).unwrap();
        let mut spans = Vec::new();

        for (style, text) in ranges {
            let r = style.foreground.r;
            let g = style.foreground.g;
            let b = style.foreground.b;

            let mut r_f = r as f32;
            let mut g_f = g as f32;
            let mut b_f = b as f32;

            if is_markdown {
                // SPECIAL "VIBRANT PRO" STYLE FOR MARKDOWN

                let max_c = r_f.max(g_f).max(b_f);

                let min_c = r_f.min(g_f).min(b_f);

                let diff = max_c - min_c;

                // Even more aggressive white for standard text

                if diff < 30.0 || max_c < 180.0 {
                    // Pure white for standard text and greyed out bits

                    r_f = 255.0;
                    g_f = 255.0;
                    b_f = 255.0;
                } else {
                    // Distinguish elements by syntect's default hues

                    if r_f > g_f && r_f > b_f {
                        // Reddish -> Headers or Strong

                        r_f = 255.0;
                        g_f = 0.0;
                        b_f = 255.0; // Magenta Headers
                    } else if g_f > r_f && g_f > b_f {
                        // Greenish -> Lists or Quotes

                        r_f = 150.0;
                        g_f = 255.0;
                        b_f = 0.0; // Lime Lists
                    } else if b_f > r_f && b_f > g_f {
                        // Bluish -> Links or Code

                        r_f = 0.0;
                        g_f = 255.0;
                        b_f = 255.0; // Cyan Links
                    }
                }
            } else {
                // "ULTRA-VIBRANT-SYNTAX" Heuristic:
                let r_f32 = r as f32;
                let g_f32 = g as f32;
                let b_f32 = b as f32;

                let max_c = r_f32.max(g_f32).max(b_f32);
                let min_c = r_f32.min(g_f32).min(b_f32);
                let diff = max_c - min_c;

                if diff < 20.0 {
                    // Muted tones (comments, punctuation)
                    if max_c < 140.0 {
                        // Muted Blue-Grey for comments
                        r_f = 100.0;
                        g_f = 120.0;
                        b_f = 140.0;
                    } else {
                        // Standard text -> Off-white
                        r_f = 230.0;
                        g_f = 235.0;
                        b_f = 240.0;
                    }
                } else {
                    // Saturated Mapping
                    if r_f32 > g_f32 && r_f32 > b_f32 {
                        if g_f32 > 120.0 {
                            // Bright Yellow/Orange (Types/Classes)
                            r_f = 255.0;
                            g_f = 215.0;
                            b_f = 0.0;
                        } else {
                            // Vibrant Pink/Red (Keywords/Storage)
                            r_f = 255.0;
                            g_f = 45.0;
                            b_f = 85.0;
                        }
                    } else if g_f32 > r_f32 && g_f32 > b_f32 {
                        // Matrix Green (Strings/Values)
                        r_f = 0.0;
                        g_f = 255.0;
                        b_f = 135.0;
                    } else if b_f32 > r_f32 && b_f32 > g_f32 {
                        if r_f32 > 130.0 {
                            // Neon Purple (Functions/Methods)
                            r_f = 180.0;
                            g_f = 100.0;
                            b_f = 255.0;
                        } else {
                            // Electric Blue (Variables/Constants)
                            r_f = 0.0;
                            g_f = 180.0;
                            b_f = 255.0;
                        }
                    }

                    // Boost saturation to max
                    let cur_max = r_f.max(g_f).max(b_f).max(1.0);
                    let boost = 255.0 / cur_max;
                    r_f *= boost;
                    g_f *= boost;
                    b_f *= boost;
                }
            }

            let fg = Color::Rgb(
                r_f.clamp(0.0, 255.0) as u8,
                g_f.clamp(0.0, 255.0) as u8,
                b_f.clamp(0.0, 255.0) as u8,
            );
            let mut ratatui_style = Style::default().fg(fg);

            if style.font_style.contains(FontStyle::BOLD) {
                ratatui_style = ratatui_style.add_modifier(Modifier::BOLD);
            }
            if style.font_style.contains(FontStyle::ITALIC) {
                ratatui_style = ratatui_style.add_modifier(Modifier::ITALIC);
            }
            if style.font_style.contains(FontStyle::UNDERLINE) {
                ratatui_style = ratatui_style.add_modifier(Modifier::UNDERLINED);
            }

            // Remove trailing newline from text if it exists to avoid double spacing in Ratatui
            let clean_text = text.trim_end_matches('\n').trim_end_matches('\r');
            if !clean_text.is_empty() || text == " " {
                spans.push(Span::styled(clean_text.to_string(), ratatui_style));
            }
        }
        lines.push(Line::from(spans));
    }
    lines
}

pub fn draw_stat_bar(
    label: &str,
    value: f32,
    max: f32,
    width: u16,
    text_color: Color,
) -> Line<'static> {
    let bar_width = width.saturating_sub(label.len() as u16 + 7); // Subtract label, spaces, and percentage text
    let ratio = (value / max).clamp(0.0, 1.0);
    let filled = (ratio * bar_width as f32).round() as usize;

    let mut spans = vec![Span::styled(
        format!("{} ", label),
        Style::default().fg(Color::DarkGray),
    )];

    for i in 0..bar_width as usize {
        let symbol = if i < filled { "█" } else { "░" };
        let color = if ratio < 0.4 {
            Color::Rgb(0, 255, 150) // Cyber Green
        } else if ratio < 0.7 {
            Color::Rgb(255, 255, 0) // Yellow
        } else {
            Color::Rgb(255, 0, 85) // Neon Red
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
        Style::default().fg(text_color).add_modifier(Modifier::BOLD),
    ));
    Line::from(spans)
}
pub fn is_binary_content(bytes: &[u8]) -> bool {
    // Basic binary check: check for null bytes in the first 8KB
    bytes.iter().take(8192).any(|&b| b == 0)
}

pub fn copy_recursive(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    if src.is_dir() {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let ty = entry.file_type()?;
            if ty.is_dir() {
                copy_recursive(&entry.path(), &dst.join(entry.file_name()))?;
            } else {
                std::fs::copy(entry.path(), dst.join(entry.file_name()))?;
            }
        }
    } else {
        std::fs::copy(src, dst)?;
    }
    Ok(())
}

pub fn move_recursive(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    if src == dst {
        return Ok(());
    }

    // Attempt atomic rename first
    if let Err(e) = std::fs::rename(src, dst) {
        // Fallback for cross-device moves (EXDEV = 18)
        let err_code = e.raw_os_error();
        if err_code == Some(18) || e.kind() == std::io::ErrorKind::Other {
            // Safety: Ensure source exists
            if !src.exists() {
                return Err(e);
            }
            // Safety: Don't move into self
            if dst.starts_with(src) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Cannot move into self",
                ));
            }

            copy_recursive(src, dst)?;
            if src.is_dir() {
                std::fs::remove_dir_all(src)?;
            } else {
                std::fs::remove_file(src)?;
            }
        } else {
            return Err(e);
        }
    }
    Ok(())
}
pub fn delete_word_backwards(s: &mut String) {
    if s.is_empty() {
        return;
    }
    let mut i = s.len();

    // Skip trailing whitespace
    while i > 0 {
        let prev = s[..i].chars().next_back().unwrap();
        if prev.is_whitespace() {
            i -= prev.len_utf8();
        } else {
            break;
        }
    }
    // Skip the word
    while i > 0 {
        let prev = s[..i].chars().next_back().unwrap();
        if !prev.is_whitespace() {
            i -= prev.len_utf8();
        } else {
            break;
        }
    }
    s.truncate(i);
}

pub fn spawn_terminal_at(path: &std::path::Path, new_tab: bool, command: Option<&str>) -> bool {
    let log = |msg: &str| {
        use std::io::Write;
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open("debug.log")
        {
            let _ = writeln!(file, "[{}] [TERM_SPAWN] {}", chrono::Local::now(), msg);
        }
    };

    log(&format!(
        "Spawning terminal at {:?} (new_tab={}, command={:?})",
        path, new_tab, command
    ));

    // 1. Context-aware Tab Spawning
    if new_tab {
        // Konsole
        if let (Ok(service), Ok(window)) = (
            std::env::var("KONSOLE_DBUS_SERVICE"),
            std::env::var("KONSOLE_DBUS_WINDOW"),
        ) {
            log(&format!(
                "Konsole detected: service={}, window={}",
                service, window
            ));
            let mut dbus_cmd = "qdbus";
            if !command_exists("qdbus") && command_exists("qdbus6") {
                dbus_cmd = "qdbus6";
            }
            log(&format!("Using DBus command: {}", dbus_cmd));

            let args = vec![
                "--session".to_string(),
                service.clone(),
                window,
                "org.kde.konsole.Window.newSession".to_string(),
                "".to_string(),
                path.to_string_lossy().to_string(),
            ];

            match std::process::Command::new(dbus_cmd).args(&args).output() {
                Ok(output) => {
                    if output.status.success() {
                        let session_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
                        log(&format!("New session created, ID: {}", session_id));
                        if !session_id.is_empty() {
                            if let Some(cmd_str) = command {
                                let session_path = format!("/Sessions/{}", session_id);
                                let _ = std::process::Command::new("qdbus")
                                    .args([
                                        "--session",
                                        &service,
                                        &session_path,
                                        "org.kde.konsole.Session.runCommand",
                                        cmd_str,
                                    ])
                                    .spawn();
                            }
                        }

                        // Try to raise the window
                        let main_win = "/konsole/MainWindow_1";
                        let _ = std::process::Command::new(dbus_cmd)
                            .args([
                                "--session",
                                &service,
                                main_win,
                                "org.qtproject.Qt.QWidget.raise",
                            ])
                            .spawn();

                        return true;
                    } else {
                        log(&format!(
                            "DBus command failed: {}",
                            String::from_utf8_lossy(&output.stderr)
                        ));
                    }
                }
                Err(e) => log(&format!("Failed to execute {}: {}", dbus_cmd, e)),
            }
        }

        // Kitty
        if std::env::var("KITTY_WINDOW_ID").is_ok() {
            log("Kitty detected");
            let mut args = vec![
                "@".to_string(),
                "launch".to_string(),
                "--type=tab".to_string(),
                "--cwd".to_string(),
                path.to_string_lossy().to_string(),
            ];
            if let Some(cmd) = command {
                args.push(cmd.to_string());
            }
            match std::process::Command::new("kitty").args(&args).spawn() {
                Ok(_) => {
                    log("Kitty tab spawned");
                    return true;
                }
                Err(e) => log(&format!("Failed to spawn kitty tab: {}", e)),
            }
        }

        // Wezterm
        if std::env::var("WEZTERM_PANE").is_ok() {
            log("Wezterm detected");
            let mut args = vec![
                "cli".to_string(),
                "spawn".to_string(),
                "--cwd".to_string(),
                path.to_string_lossy().to_string(),
            ];
            if let Some(cmd) = command {
                args.push("--".to_string());
                args.push(cmd.to_string());
            }
            match std::process::Command::new("wezterm").args(&args).spawn() {
                Ok(_) => {
                    log("Wezterm tab spawned");
                    return true;
                }
                Err(e) => log(&format!("Failed to spawn wezterm tab: {}", e)),
            }
        }
    }

    // 2. Generic Spawning (fallback or new window)
    log("Using generic spawning fallback");
    let terminals = [
        "x-terminal-emulator",
        "gnome-terminal",
        "konsole",
        "alacritty",
        "kitty",
        "wezterm",
        "xfce4-terminal",
        "termite",
        "urxvt",
    ];
    for term in terminals {
        let mut args = Vec::new();

        match term {
            "gnome-terminal" => {
                if new_tab {
                    args.push("--tab".to_string());
                }
                args.push(format!("--working-directory={}", path.to_string_lossy()));
                if let Some(cmd) = command {
                    args.push("--".to_string());
                    args.push(cmd.to_string());
                }
            }
            "konsole" => {
                if new_tab {
                    args.push("--new-tab".to_string());
                }
                args.push("--workdir".to_string());
                args.push(path.to_string_lossy().to_string());
                if let Some(cmd) = command {
                    args.push("-e".to_string());
                    args.push(cmd.to_string());
                }
            }
            "xfce4-terminal" => {
                if new_tab {
                    args.push("--tab".to_string());
                }
                args.push("--working-directory".to_string());
                args.push(path.to_string_lossy().to_string());
                if let Some(cmd) = command {
                    args.push("-e".to_string());
                    args.push(cmd.to_string());
                }
            }
            "kitty" => {
                args.push("--directory".to_string());
                args.push(path.to_string_lossy().to_string());
                if let Some(cmd) = command {
                    args.push(cmd.to_string());
                }
            }
            "wezterm" => {
                args.push("start".to_string());
                args.push("--cwd".to_string());
                args.push(path.to_string_lossy().to_string());
                if let Some(cmd) = command {
                    args.push(cmd.to_string());
                }
            }
            _ => {
                args.push("--working-directory".to_string());
                args.push(path.to_string_lossy().to_string());
                if let Some(cmd) = command {
                    args.push("-e".to_string());
                    args.push(cmd.to_string());
                }
            }
        }

        if std::process::Command::new(term)
            .args(&args)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .stdin(std::process::Stdio::null())
            .spawn()
            .is_ok()
        {
            return true;
        }
    }
    false
}

/// Checks if a file is likely binary or too large to preview comfortably.
/// Returns (is_binary, is_too_large, size_in_mb)
pub fn check_file_suitability(path: &std::path::Path, max_bytes: u64) -> (bool, bool, u64) {
    if let Ok(metadata) = std::fs::metadata(path) {
        let size = metadata.len();
        if size > max_bytes {
            return (false, true, size / (1024 * 1024));
        }

        if let Ok(content) = std::fs::read(path) {
            return (is_binary_content(&content), false, size / (1024 * 1024));
        }
    }
    (false, false, 0)
}

pub fn set_clipboard_text(text: &str) {
    // 1. Try OSC 52 (Internal via stdout)
    {
        use std::io::Write;
        let mut stdout = std::io::stdout();
        let _ = crate::visuals::osc::copy_to_clipboard(&mut stdout, text);
        let _ = stdout.flush();
    }

    // 2. Try Local Tools (for desktop environments)
    let _ = std::process::Command::new("wl-copy")
        .arg(text)
        .spawn()
        .or_else(|_| {
            std::process::Command::new("xclip")
                .arg("-selection")
                .arg("clipboard")
                .stdin(std::process::Stdio::piped())
                .spawn()
                .map(|mut child| {
                    use std::io::Write;
                    if let Some(mut stdin) = child.stdin.take() {
                        let _ = stdin.write_all(text.as_bytes());
                    }
                    child
                })
        })
        .or_else(|_| {
            std::process::Command::new("pbcopy")
                .stdin(std::process::Stdio::piped())
                .spawn()
                .map(|mut child| {
                    use std::io::Write;
                    if let Some(mut stdin) = child.stdin.take() {
                        let _ = stdin.write_all(text.as_bytes());
                    }
                    child
                })
        });
}

pub fn get_clipboard_text() -> Option<String> {
    std::process::Command::new("wl-paste")
        .output()
        .or_else(|_| {
            std::process::Command::new("xclip")
                .arg("-o")
                .arg("-selection")
                .arg("clipboard")
                .output()
        })
        .or_else(|_| std::process::Command::new("pbpaste").output())
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .ok()
}

pub fn get_primary_selection_text() -> Option<String> {
    std::process::Command::new("wl-paste")
        .arg("--primary")
        .output()
        .or_else(|_| {
            std::process::Command::new("xclip")
                .arg("-o")
                .arg("-selection")
                .arg("primary")
                .output()
        })
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .ok()
}
