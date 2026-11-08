use std::path::Path;
use std::process::Command;

pub fn spawn_terminal(path: &Path, new_tab: bool, command: Option<&str>) -> bool {
    let path_str = path.to_string_lossy().to_string();

    if new_tab {
        // Konsole: use qdbus to call Window.newSession() on the existing window.
        // konsole --new-tab from a non-terminal process opens a NEW window,
        // not a tab in the existing one. The D-Bus API is the only reliable way.
        if let Ok(service) = std::env::var("KONSOLE_DBUS_SERVICE") {
            if let Ok(window) = std::env::var("KONSOLE_DBUS_WINDOW") {
                // Try qdbus first (KDE native, handles types properly)
                let qdbus_result = Command::new("qdbus")
                    .args([
                        &service,
                        &window,
                        "org.kde.konsole.Window.newSession",
                        "",  // default profile
                        &path_str,
                    ])
                    .output();
                if let Ok(output) = qdbus_result {
                    if output.status.success() {
                        return true;
                    }
                }
                // Fallback: try qdbus6 (KDE 6)
                let qdbus6_result = Command::new("qdbus6")
                    .args([
                        &service,
                        &window,
                        "org.kde.konsole.Window.newSession",
                        "",
                        &path_str,
                    ])
                    .output();
                if let Ok(output) = qdbus6_result {
                    if output.status.success() {
                        return true;
                    }
                }
            }
        }

        if std::env::var("KITTY_WINDOW_ID").is_ok() {
            let mut args = vec!["@".to_string(), "launch".to_string(), "--type=tab".to_string(), "--cwd".to_string()];
            args.push(path_str.clone());
            if let Some(cmd_str) = command {
                for arg in split_command(cmd_str) {
                    args.push(arg);
                }
            }
            if Command::new("kitty").args(&args).stderr(std::process::Stdio::null()).spawn().is_ok() {
                return true;
            }
        }

        if std::env::var("WEZTERM_PANE").is_ok() {
            let mut args = vec!["cli".to_string(), "spawn".to_string(), "--cwd".to_string()];
            args.push(path_str.clone());
            if let Some(cmd_str) = command {
                args.push("--".to_string());
                for arg in split_command(cmd_str) {
                    args.push(arg);
                }
            }
            if Command::new("wezterm").args(&args).stderr(std::process::Stdio::null()).spawn().is_ok() {
                return true;
            }
        }
    }

    let terminals = [
        "konsole",
        "gnome-terminal",
        "xfce4-terminal",
        "x-terminal-emulator",
        "alacritty",
        "kitty",
        "wezterm",
        "termite",
        "urxvt",
    ];

    for term in terminals {
        if !command_exists(term) {
            continue;
        }

        let mut args: Vec<String> = Vec::new();
        let mut did_build = false;

        match term {
            "konsole" => {
                if new_tab {
                    args.push("--new-tab".to_string());
                }
                args.push("--workdir".to_string());
                args.push(path_str.clone());
                if let Some(cmd_str) = command {
                    let split = split_command(cmd_str);
                    if !split.is_empty() {
                        args.push("-e".to_string());
                        args.extend(split);
                    }
                }
                did_build = true;
            }
            "gnome-terminal" => {
                if new_tab {
                    args.push("--tab".to_string());
                }
                args.push(format!("--working-directory={}", path_str));
                if let Some(cmd_str) = command {
                    let split = split_command(cmd_str);
                    if !split.is_empty() {
                        args.push("--".to_string());
                        args.extend(split);
                    }
                }
                did_build = true;
            }
            "xfce4-terminal" => {
                if new_tab {
                    args.push("--tab".to_string());
                }
                args.push("--working-directory".to_string());
                args.push(path_str.clone());
                if let Some(cmd_str) = command {
                    let split = split_command(cmd_str);
                    if !split.is_empty() {
                        args.push("-e".to_string());
                        args.extend(split);
                    }
                }
                did_build = true;
            }
            "kitty" => {
                args.push("@".to_string());
                args.push("launch".to_string());
                if new_tab {
                    args.push("--type=tab".to_string());
                }
                args.push("--cwd".to_string());
                args.push(path_str.clone());
                if let Some(cmd_str) = command {
                    args.extend(split_command(cmd_str));
                }
                did_build = true;
            }
            "wezterm" => {
                args.push("cli".to_string());
                args.push("spawn".to_string());
                args.push("--cwd".to_string());
                args.push(path_str.clone());
                if let Some(cmd_str) = command {
                    let split = split_command(cmd_str);
                    if !split.is_empty() {
                        args.push("--".to_string());
                        args.extend(split);
                    }
                }
                did_build = true;
            }
            "x-terminal-emulator" | "alacritty" | "termite" | "urxvt" => {
                args.push("--working-directory".to_string());
                args.push(path_str.clone());
                if let Some(cmd_str) = command {
                    let split = split_command(cmd_str);
                    if !split.is_empty() {
                        args.push("-e".to_string());
                        args.extend(split);
                    }
                }
                did_build = true;
            }
            _ => {}
        }

        if did_build && Command::new(term)
            .args(&args)
            .stderr(std::process::Stdio::null())
            .spawn().is_ok()
        {
            return true;
        }
    }

    false
}

fn command_exists(cmd: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {} > /dev/null 2>&1", cmd))
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Quote one argument for safe interpolation into an `sh -c` command.
/// Single-quote wrapping with `'` escaped as `'\''`; always safe, including
/// for args containing spaces or shell metacharacters.
pub fn shell_quote_arg(arg: &str) -> String {
    format!("'{}'", arg.replace('\'', "'\\''"))
}

fn split_command(cmd: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut current = String::new();
    // A quoted empty string is a real (empty) argument: track token starts
    // separately from content so `""` yields one empty arg, not zero.
    let mut token_started = false;
    let mut chars = cmd.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' if !in_single => {
                // Backslash escape: next char is taken literally.
                if let Some(next) = chars.next() {
                    token_started = true;
                    current.push(next);
                }
            }
            '\'' if !in_double => {
                in_single = !in_single;
                token_started = true;
            }
            '"' if !in_single => {
                in_double = !in_double;
                token_started = true;
            }
            ' ' | '\t' | '\n' if !in_single && !in_double => {
                if token_started {
                    args.push(current.clone());
                    current.clear();
                    token_started = false;
                }
            }
            _ => {
                token_started = true;
                current.push(c);
            }
        }
    }
    if token_started {
        args.push(current);
    }
    args
}