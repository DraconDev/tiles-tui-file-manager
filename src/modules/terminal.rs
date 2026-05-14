use std::path::Path;
use std::process::Command;

pub fn spawn_terminal(path: &Path, new_tab: bool, command: Option<&str>) -> bool {
    let path_str = path.to_string_lossy().to_string();

    if new_tab {
        if let (Ok(service), Ok(window)) = (
            std::env::var("KONSOLE_DBUS_SERVICE"),
            std::env::var("KONSOLE_DBUS_WINDOW"),
        ) {
            let dbus_cmd = if command_exists("qdbus") {
                "qdbus"
            } else if command_exists("qdbus6") {
                "qdbus6"
            } else {
                None
            };

            if let Some(dbus) = dbus_cmd {
                let args = vec![
                    "--session".to_string(),
                    service.clone(),
                    window.clone(),
                    "org.kde.konsole.Window.newSession".to_string(),
                    "".to_string(),
                    path_str.clone(),
                ];

                if let Ok(output) = Command::new(dbus).args(&args).output() {
                    if output.status.success() {
                        let session_id =
                            String::from_utf8_lossy(&output.stdout).trim().to_string();
                        if !session_id.is_empty() {
                            if let Some(cmd_str) = command {
                                let session_path = format!("/Sessions/{}", session_id);
                                let _ = Command::new(dbus)
                                    .args([
                                        "--session",
                                        &service,
                                        &session_path,
                                        "org.kde.konsole.Session.runCommand",
                                        cmd_str,
                                    ])
                                    .spawn();
                            }
                            let main_win = "/konsole/MainWindow_1";
                            let _ = Command::new(dbus)
                                .args([
                                    "--session",
                                    &service,
                                    main_win,
                                    "org.qtproject.Qt.QWidget.raise",
                                ])
                                .spawn();
                            return true;
                        }
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
            if Command::new("kitty").args(&args).spawn().is_ok() {
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
            if Command::new("wezterm").args(&args).spawn().is_ok() {
                return true;
            }
        }
    }

    let terminals: Vec<(&str, Vec<&str>)> = vec![
        ("konsole", vec!["--new-tab", "--workdir", "-e"]),
        ("gnome-terminal", vec!["--tab", "--"]),
        ("xfce4-terminal", vec!["--tab", "-e"]),
        ("x-terminal-emulator", vec!["-e"]),
        ("alacritty", vec!["-e"]),
        ("kitty", vec!["@", "launch", "--type=tab", "--cwd"]),
        ("wezterm", vec!["cli", "spawn", "--cwd", "--"]),
        ("termite", vec!["-e"]),
        ("urxvt", vec!["-e"]),
    ];

    for (term, base_args) in terminals {
        if !command_exists(term) {
            continue;
        }

        let mut args: Vec<String> = base_args.iter().map(|s| (*s).to_string()).collect();
        args.push(path_str.clone());

        if let Some(cmd_str) = command {
            let split_args = split_command(cmd_str);
            if split_args.is_empty() {
                continue;
            }
            if term == "kitty" {
                args.push(split_args[0].clone());
                if split_args.len() > 1 {
                    for arg in &split_args[1..] {
                        args.push(arg.clone());
                    }
                }
            } else {
                for arg in split_args {
                    args.push(arg);
                }
            }
        }

        if Command::new(term).args(&args).spawn().is_ok() {
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

fn split_command(cmd: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut current = String::new();
    for c in cmd.chars() {
        match c {
            '\'' if !in_double => {
                in_single = !in_single;
            }
            '"' if !in_single => {
                in_double = !in_double;
            }
            ' ' | '\t' | '\n' if !in_single && !in_double => {
                if !current.is_empty() {
                    args.push(current.clone());
                    current.clear();
                }
            }
            _ => {
                current.push(c);
            }
        }
    }
    if !current.is_empty() {
        args.push(current);
    }
    args
}