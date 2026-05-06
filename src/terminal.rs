use std::path::Path;

/// Spawn a terminal at the given path.
/// If `new_tab` is true, try to open a tab in the current terminal.
/// Falls back to opening a new window.
pub fn spawn_terminal_at(path: &Path, new_tab: bool, command: Option<&str>) -> bool {
    let path_str = path.to_string_lossy().to_string();
    let timestamp = chrono::Local::now();

    let log = |msg: &str| {
        let _ = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open("debug.log")
            .and_then(|mut f| {
                use std::io::Write;
                writeln!(f, "[{}] [LOCAL_TERM] {}", timestamp, msg)
            });
    };

    log(&format!("spawn_terminal_at new_tab={} path={}", new_tab, path_str));

    if new_tab {
        // Try konsole --new-tab first (most reliable method)
        if command_exists("konsole") {
            log("Trying konsole --new-tab");
            let mut cmd = std::process::Command::new("konsole");
            cmd.arg("--new-tab")
               .arg("--workdir")
               .arg(&path_str);
            if let Some(cmd_str) = command {
                cmd.arg("-e").arg(cmd_str);
            }
            if cmd.spawn().is_ok() {
                log("konsole --new-tab succeeded");
                return true;
            }
            log("konsole --new-tab failed");
        }

        // Try Kitty tab
        if std::env::var("KITTY_WINDOW_ID").is_ok() {
            log("Kitty detected, trying to open tab");
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
                log("Kitty tab opened successfully");
                return true;
            }
            log("Kitty tab failed");
        }

        // Try D-Bus for Konsole (only if KONSOLE_DBUS_SERVICE is set and konsole --new-tab didn't work)
        if let Ok(service) = std::env::var("KONSOLE_DBUS_SERVICE") {
            if let Ok(window) = std::env::var("KONSOLE_DBUS_WINDOW") {
                log(&format!("Konsole D-Bus: service={}, window={}", service, window));
                let dbus_cmd = if command_exists("qdbus6") { "qdbus6" } else { "qdbus" };
                log(&format!("Using D-Bus cmd: {}", dbus_cmd));

                // Try newSession with no args first (creates session with default profile/dir)
                if let Ok(output) = std::process::Command::new(dbus_cmd)
                    .args(["--session", &service, &window, "org.kde.konsole.Window.newSession"])
                    .output()
                {
                    log(&format!("DBus stdout: {:?}", String::from_utf8_lossy(&output.stdout).trim()));
                    log(&format!("DBus stderr: {:?}", String::from_utf8_lossy(&output.stderr).trim()));
                    if output.status.success() {
                        let session_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
                        if !session_id.is_empty() {
                            log(&format!("D-Bus newSession created session: {}", session_id));
                            // Try to set the working directory
                            let session_path = format!("/Sessions/{}", session_id);
                            let _ = std::process::Command::new(dbus_cmd)
                                .args(["--session", &service, &session_path, "org.kde.konsole.Session.setWorkingDirectory", &path_str])
                                .spawn();
                            return true;
                        }
                    }
                }
            }
        }

        log("All new_tab methods failed, falling back to new window");
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
            log(&format!("Fallback: opened new {} window", term));
            return true;
        }
    }

    log("All terminal spawning methods failed");
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