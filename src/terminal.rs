use std::path::Path;

/// Spawn a terminal at the given path.
/// If `new_tab` is true, try to open a tab in the current terminal.
/// Falls back to opening a new window.
pub fn spawn_terminal_at(path: &Path, new_tab: bool, command: Option<&str>) -> bool {
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
                path.to_string_lossy().to_string(),
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
                "@", "launch", "--type=tab", "--cwd",
                &path.to_string_lossy(),
            ];
            if let Some(cmd) = command {
                args.push(cmd);
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
                cmd.args(["--workdir", &path.to_string_lossy()]);
            }
            "gnome-terminal" => {
                cmd.arg(format!("--working-directory={}", path.display()));
            }
            "kitty" => {
                cmd.args(["--directory", &path.to_string_lossy()]);
            }
            _ => {
                cmd.args(["--working-directory", &path.to_string_lossy()]);
            }
        }
        
        if command.is_some() {
            cmd.arg(command.unwrap());
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
