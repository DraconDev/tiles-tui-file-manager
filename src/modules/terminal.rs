use std::path::Path;
use std::process::Command;

pub fn spawn_terminal(path: &Path, new_tab: bool, command: Option<&str>) -> bool {
    if new_tab {
        if let Ok(service) = std::env::var("KONSOLE_DBUS_SERVICE") {
            let window = std::env::var("KONSOLE_DBUS_WINDOW").unwrap_or_default();
            std::env::remove_var("KONSOLE_DBUS_SERVICE");
            std::env::remove_var("KONSOLE_DBUS_WINDOW");
            if let Some(cmd_str) = command {
                let args = split_command(cmd_str);
                if args.is_empty() {
                    std::env::set_var("KONSOLE_DBUS_SERVICE", service);
                    std::env::set_var("KONSOLE_DBUS_WINDOW", window);
                    return false;
                }
                let mut cmd = Command::new("konsole");
                cmd.arg("--new-tab");
                cmd.arg("--workdir").arg(path);
                cmd.arg("-e").arg(&args[0]);
                if args.len() > 1 {
                    cmd.args(&args[1..]);
                }
                if cmd.spawn().is_ok() {
                    std::env::set_var("KONSOLE_DBUS_SERVICE", service);
                    std::env::set_var("KONSOLE_DBUS_WINDOW", window);
                    return true;
                }
                std::env::set_var("KONSOLE_DBUS_SERVICE", service);
                std::env::set_var("KONSOLE_DBUS_WINDOW", window);
                return false;
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
        args.push(path.to_string_lossy().to_string());

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
    let mut chars = cmd.chars().peekable();

    while let Some(c) = chars.next() {
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