use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// A single server bookmark (replaces RemoteBookmark)
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ServerConfig {
    pub name: String,
    pub host: String,
    pub user: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub key_path: Option<PathBuf>,
    #[serde(default)]
    pub last_path: PathBuf,
}

fn default_port() -> u16 {
    22
}

/// Top-level structure for servers.toml
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ServersFile {
    #[serde(default)]
    pub server: Vec<ServerConfig>,
}

impl ServersFile {
    pub fn is_empty(&self) -> bool {
        self.server.is_empty()
    }

    pub fn len(&self) -> usize {
        self.server.len()
    }
}

/// Path to the servers.toml config file
pub fn servers_toml_path() -> Option<PathBuf> {
    let config_dir = dirs::config_dir()?.join("tiles");
    Some(config_dir.join("servers.toml"))
}

/// Load servers from servers.toml. Returns empty list if file doesn't exist.
pub fn load_servers() -> Vec<ServerConfig> {
    let path = match servers_toml_path() {
        Some(p) => p,
        None => return Vec::new(),
    };

    if !path.exists() {
        return Vec::new();
    }

    match fs::read_to_string(&path) {
        Ok(content) => {
            match toml::from_str::<ServersFile>(&content) {
                Ok(file) => file.server,
                Err(e) => {
                    crate::app::log_debug(&format!("Failed to parse servers.toml: {}", e));
                    Vec::new()
                }
            }
        }
        Err(e) => {
            crate::app::log_debug(&format!("Failed to read servers.toml: {}", e));
            Vec::new()
        }
    }
}

/// Save servers to servers.toml
pub fn save_servers(servers: &[ServerConfig]) -> Result<(), Box<dyn std::error::Error>> {
    let path = match servers_toml_path() {
        Some(p) => p,
        None => return Err("Could not find config dir".into()),
    };

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let file = ServersFile {
        server: servers.to_vec(),
    };

    let toml_string = toml::to_string_pretty(&file)?;
    fs::write(&path, toml_string)?;
    Ok(())
}

/// Save servers quietly (logs error but doesn't propagate)
pub fn save_servers_quiet(servers: &[ServerConfig]) {
    if let Err(e) = save_servers(servers) {
        crate::app::log_debug(&format!("save_servers failed: {}", e));
    }
}

/// Migrate legacy remote_bookmarks from state.json to servers.toml.
/// Only runs if servers.toml doesn't exist but bookmarks do.
pub fn maybe_migrate_legacy_bookmarks(legacy: &[crate::state::RemoteBookmark]) {
    let path = match servers_toml_path() {
        Some(p) => p,
        None => return,
    };

    if path.exists() {
        return; // Already migrated
    }

    if legacy.is_empty() {
        return; // Nothing to migrate
    }

    let servers: Vec<ServerConfig> = legacy
        .iter()
        .map(|b| ServerConfig {
            name: b.name.clone(),
            host: b.host.clone(),
            user: b.user.clone(),
            port: b.port,
            key_path: b.key_path.clone(),
            last_path: b.last_path.clone(),
        })
        .collect();

    if let Err(e) = save_servers(&servers) {
        crate::app::log_debug(&format!("Failed to migrate legacy bookmarks: {}", e));
    } else {
        crate::app::log_debug(&format!(
            "Migrated {} legacy remote bookmarks to servers.toml",
            servers.len()
        ));
    }
}

/// Convert ServerConfig to RemoteBookmark (for backward compat with connection code)
impl From<ServerConfig> for crate::state::RemoteBookmark {
    fn from(s: ServerConfig) -> Self {
        crate::state::RemoteBookmark {
            name: s.name,
            host: s.host,
            user: s.user,
            port: s.port,
            key_path: s.key_path,
            last_path: s.last_path,
        }
    }
}

/// Convert RemoteBookmark to ServerConfig
impl From<crate::state::RemoteBookmark> for ServerConfig {
    fn from(b: crate::state::RemoteBookmark) -> Self {
        ServerConfig {
            name: b.name,
            host: b.host,
            user: b.user,
            port: b.port,
            key_path: b.key_path,
            last_path: b.last_path,
        }
    }
}

/// Read servers.toml content as a String (for "Edit as TOML" feature)
pub fn read_servers_toml_raw() -> Option<String> {
    let path = servers_toml_path()?;
    if !path.exists() {
        // Return a template with comments
        return Some(
            r#"# Tiles Server Configuration
# Add your remote servers here.
# Each [[server]] block defines one bookmark.

[[server]]
name = "Example Server"
host = "192.168.1.100"
user = "admin"
port = 22
# key_path = "~/.ssh/id_rsa"
"#
            .to_string(),
        );
    }
    fs::read_to_string(&path).ok()
}

/// Export servers to a TOML file (for backup/sharing)
pub fn export_servers_to_toml(servers: &[ServerConfig]) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let config_dir = dirs::config_dir()
        .ok_or("Could not find config dir")?
        .join("tiles");
    let export_path = config_dir.join("servers-export.toml");
    
    let file = ServersFile {
        server: servers.to_vec(),
    };
    
    let toml_string = toml::to_string_pretty(&file)?;
    fs::write(&export_path, toml_string)?;
    Ok(export_path)
}

/// Validation result for server config
pub struct ValidationError {
    pub field: &'static str,
    pub message: String,
}

/// Validate a server config. Returns list of errors (empty if valid).
pub fn validate_server(
    server: &ServerConfig,
    existing: &[ServerConfig],
    editing_index: Option<usize>,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if server.name.trim().is_empty() {
        errors.push(ValidationError {
            field: "name",
            message: "Name is required".to_string(),
        });
    }

    if server.host.trim().is_empty() {
        errors.push(ValidationError {
            field: "host",
            message: "Host is required".to_string(),
        });
    }

    if server.user.trim().is_empty() {
        errors.push(ValidationError {
            field: "user",
            message: "User is required".to_string(),
        });
    }

    if server.port == 0 {
        errors.push(ValidationError {
            field: "port",
            message: "Port must be greater than 0".to_string(),
        });
    }

    // Check for duplicate name (excluding the one being edited)
    let is_duplicate = existing.iter().enumerate().any(|(i, s)| {
        s.name == server.name && Some(i) != editing_index
    });
    if is_duplicate {
        errors.push(ValidationError {
            field: "name",
            message: format!("A server named '{}' already exists", server.name),
        });
    }

    errors
}

/// Write raw TOML content (for "Edit as TOML" feature)
pub fn write_servers_toml_raw(content: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = servers_toml_path().ok_or("Could not find config dir")?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sample_toml() {
        let toml = r#"
[[server]]
name = "main1"
host = "84.8.158.35"
user = "ubuntu"
port = 22
key_path = "~/.ssh/main1.key"

[[server]]
name = "micro1"
host = "141.147.94.186"
user = "ubuntu"
port = 22
key_path = "~/.ssh/micro1.key"
"#;
        let file: ServersFile = toml::from_str(toml).expect("Should parse");
        assert_eq!(file.server.len(), 2);
        assert_eq!(file.server[0].name, "main1");
        assert_eq!(file.server[1].host, "141.147.94.186");
    }

    #[test]
    fn load_actual_servers_toml() {
        let servers = load_servers();
        println!("Loaded {} servers from ~/.config/tiles/servers.toml", servers.len());
        for s in &servers {
            println!("  - {}@{}:{}", s.user, s.host, s.port);
        }
        // The file exists and should have 4 servers
        assert!(
            servers.len() >= 4,
            "Expected at least 4 servers, got {}. Check ~/.config/tiles/servers.toml exists and is valid.",
            servers.len()
        );
    }
}
