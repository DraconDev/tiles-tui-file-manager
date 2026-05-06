use std::path::Path;

/// Spawn a terminal at the given path.
/// If `new_tab` is true, try to open a tab in the current terminal.
/// Falls back to opening a new window.
pub fn spawn_terminal_at(path: &Path, new_tab: bool, command: Option<&str>) -> bool {
    let path_str = path.to_string_lossy().to_string();
    
    // Write to debug.log to confirm this code runs (not old engine)
    let _ = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open("debug.log")
        .and_then(|mut f| {
            use std::io::Write;
            writeln!(f, "[{}] [LOCAL_TERM] spawn_terminal_at new_tab={} path={}", 
                chrono::Local::now(), new_tab, path_str)
        });
    
    if new_tab {
        // Try D-Bus for Konsole
        if let (Ok(service), Ok(window)) = (
            std::env::var("KONSOLE_DBUS_SERVICE"),
            std::env::var("KONSOLE_DBUS_WINDOW"),
        ) {
            let dbus_cmd = if command_exists("qdbus6") { "qdbus6" } else { "qdbus" };
            
            let args = vec![
                "--session".to_string(),
                service.clone(),
                window,
                "org.kde.konsole.Window.newSession".to_string(),
                "".to_string(),
                path_str.clone(),
            ];

            if let Ok(output) = std::process::Command::new(dbus_cmd).args(&args).output() {
                if output.status.success() {
                    let session_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !session_id.is_empty() {
                        if let Some(cmd_str) = command {
                            let session_path = format!("/Sessions/{}", session_id);
                            let _ = std::process::Command::new(dbus_cmd)
                                .args([
                                    "--session", &service, &session_path,
                                    "org.kde.konsole.Session.runCommand", cmd_str,
                                ])
                                .spawn();
                        }
                    }
                    return true;
                }
            }
        }

        // Kitty tab
        if std::env::var("KITTY_WINDOW_ID").is_ok() {
            let mut args = vec![
                "@".to_string(), 
                "launch".to_string(), 
                "--type=tab".to_string(), 
                "--cwd".to_string(),
                path_str.clone(),
            ];
            if let Some(cmd) = command {
                args.push(cmd.to_string());
            }
            if std::process::Command::new("kitty").args(&args).spawn().is_ok() {
                return true;
            }
        }
    }

    // Generic fallback: open new window
    let terminals = [
        "konsole", "gnome-terminal", "alacritty", "kitty", "wezterm",
        "xfce4-terminal", "x-terminal-emulator",
    ];
    
    for term in terminals {
        let mut cmd = std::process::Command::new(term);
        match term {
            "konsole" => {
                cmd.args(["--workdir", &path_str]);
            }
            "gnome-terminal" => {
                cmd.arg(format!("--working-directory={}", path_str));
            }
            "kitty" => {
                cmd.args(["--directory", &path_str]);
            }
            _ => {
                cmd.args(["--working-directory", &path_str]);
            }
        }
        
        if let Some(cmd_str) = command {
            cmd.arg(cmd_str);
        }
        
        if cmd.spawn().is_ok() {
            return true;
        }
    }
    
    false
}

fn command_exists(cmd: &str) -> bool {
    std::process::Command::new("which")
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
