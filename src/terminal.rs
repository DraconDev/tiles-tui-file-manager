use std::path::Path;

/// Spawn a terminal at the given path.
/// If `new_tab` is true, try to open a tab in the current terminal.
/// Falls back to opening a new window.
/// 
/// # IMPORTANT: D-Bus Tool Selection
/// We use `busctl` (systemd) instead of `qdbus` (Qt) because:
/// - `qdbus` crashes with SIGSEGV (exit 139) on Konsole 26.04.0+
/// - The crash triggers "Qt Multimedia SymbolResolver" errors in the UI
/// - `busctl` has no Qt dependency and works reliably
/// 
/// DO NOT revert to `qdbus` without testing on the target Konsole version.
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
        // When we have a command to run, use konsole --new-tab -e directly
        // D-Bus runCommand/sendText are broken in Konsole 26.04+ ("Access denied")
        if command.is_some() && command_exists("konsole") {
            log("Command provided, using konsole --new-tab -e");
            let mut cmd = std::process::Command::new("konsole");
            cmd.arg("--new-tab")
               .arg("--workdir")
               .arg(&path_str);
            if let Some(cmd_str) = command {
                cmd.arg("-e").arg(cmd_str);
            }
            match cmd.output() {
                Ok(output) => {
                    if output.status.success() {
                        log("konsole --new-tab -e succeeded");
                        return true;
                    }
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    log(&format!("konsole --new-tab -e failed: {}", stderr));
                }
                Err(e) => {
                    log(&format!("konsole --new-tab -e execution failed: {:?}", e));
                }
            }
            // Fall through to other methods if konsole fails
        }

        // Try busctl D-Bus for opening empty tabs (no command, or konsole -e failed)
        // Note: runCommand/sendText are broken in Konsole 26.04+, so we only use this
        // for opening tabs without commands
        if command.is_none() {
            if let (Ok(service), Ok(window)) = (
                std::env::var("KONSOLE_DBUS_SERVICE"),
                std::env::var("KONSOLE_DBUS_WINDOW"),
            ) {
                if command_exists("busctl") {
                    log(&format!("Trying busctl D-Bus: service={}, window={}", service, window));

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

        // Try konsole --new-tab without command (fallback for empty tabs)
        if command.is_none() && command_exists("konsole") {
            log("Trying konsole --new-tab (no command)");
            let mut cmd = std::process::Command::new("konsole");
            cmd.arg("--new-tab")
               .arg("--workdir")
               .arg(&path_str);
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
