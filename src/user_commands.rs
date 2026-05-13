use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserCommand {
    pub name: String,
    #[serde(default)]
    pub key: String,
    pub exec: String,
    #[serde(default)]
    pub context: CommandContext,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CommandContext {
    #[default]
    File,
    Directory,
    Any,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserCommandsFile {
    #[serde(default)]
    pub commands: Vec<UserCommand>,
}

pub fn load_user_commands() -> Vec<UserCommand> {
    let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    let path = config_dir.join("tiles/commands.toml");

    if !path.exists() {
        return Vec::new();
    }

    match std::fs::read_to_string(&path) {
        Ok(content) => {
            match toml::from_str::<UserCommandsFile>(&content) {
                Ok(file) => file.commands,
                Err(e) => {
                    crate::app::log_debug(&format!("Failed to parse commands.toml: {}", e));
                    Vec::new()
                }
            }
        }
        Err(e) => {
            crate::app::log_debug(&format!("Failed to read commands.toml: {}", e));
            Vec::new()
        }
    }
}

pub fn create_default_commands_toml() -> String {
    r#"# Tiles Custom Commands
# Place this file at ~/.config/tiles/commands.toml
# Each command can use {path} as a placeholder for the selected file/directory

[[commands]]
name = "Open in VS Code"
key = "v"
exec = "code {path}"
context = "file"

[[commands]]
name = "Open in Vim"
key = "V"
exec = "vim {path}"
context = "file"

[[commands]]
name = "Copy to Clipboard"
key = "y"
exec = "echo {path} | xclip -selection clipboard"
context = "any"

[[commands]]
name = "Terminal Here"
key = "t"
exec = "gnome-terminal --working-directory={path}"
context = "directory"
"#.to_string()
}

pub fn maybe_create_default_commands_toml() {
    let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    let path = config_dir.join("tiles/commands.toml");

    if path.exists() {
        return;
    }

    let _ = std::fs::create_dir_all(path.parent().unwrap());
    let _ = std::fs::write(&path, create_default_commands_toml());
}

pub fn expand_command_template(template: &str, path: &std::path::Path) -> Vec<String> {
    let path_str = path.to_string_lossy().to_string();
    template
        .split_whitespace()
        .map(|part| part.replace("{path}", &path_str))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_command_template() {
        let path = PathBuf::from("/home/user/file.txt");
        let parts = expand_command_template("code {path}", &path);
        assert_eq!(parts, vec!["code", "/home/user/file.txt"]);
    }

    #[test]
    fn test_expand_with_multiple_placeholders() {
        let path = PathBuf::from("/tmp/test");
        let parts = expand_command_template("cp {path} {path}.bak", &path);
        assert_eq!(parts, vec!["cp", "/tmp/test", "/tmp/test.bak"]);
    }
}
