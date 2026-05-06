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
        // Try busctl D-Bus first (reliable, no Qt crash)
        if let (Ok(service), Ok(window)) = (
            std::env::var("KONSOLE_DBUS_SERVICE"),
            std::env::var("KONSOLE_DBUS_WINDOW"),
        ) {
            if command_exists("busctl") {
                log(&format!("Trying busctl D-Bus: service={}, window={}", service, window));

                // Call newSession with profile and directory args
                // Signature: newSession(s profile, s directory) -> i session_id
                let args = vec![
                    "--user".to_string(),
                    "call".to_string(),
                    service.clone(),
                    window.clone(),
                    "org.kde.konsole.Window".to_string(),
                    "newSession".to_string(),
                    "ss".to_string(),
                    "".to_string(),
                    path_str.clone(),
                ];

                match std::process::Command::new("busctl").args(&args).output() {
                    Ok(output) => {
                        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                        log(&format!("busctl stdout: '{}'", stdout));
                        if !stderr.is_empty() {
                            log(&format!("busctl stderr: '{}'", stderr));
                        }

                        if output.status.success() && stdout.starts_with("i ") {
                            if let Some(session_id) = stdout.split_whitespace().nth(1) {
                                log(&format!("D-Bus newSession created session: {}", session_id));

                                // Run command in new session if provided
                                if let Some(cmd_str) = command {
                                    let session_path = format!("/Sessions/{}", session_id);
                                    let _ = std::process::Command::new("busctl")
                                        .args([
                                            "--user", "call", &service, &session_path,
                                            "org.kde.konsole.Session", "runCommand", "s", cmd_str,
                                        ])
                                        .spawn();
                                }
                                return true;
                            }
                        }
                        log("busctl call failed or returned unexpected output");
                    }
                    Err(e) => {
                        log(&format!("busctl command failed: {:?}", e));
                    }
                }
            } else {
                log("busctl not found, skipping D-Bus");
            }
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

        // Try konsole --new-tab (requires single-process mode)
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