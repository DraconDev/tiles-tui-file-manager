use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Expand `~` and `~user` in a path to the full home directory path.
/// Returns the original path if it doesn't start with `~` or home dir is unknown.
pub fn expand_tilde(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    if !s.starts_with('~') {
        return path.to_path_buf();
    }
    if s == "~" || s.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            if s == "~" {
                return home;
            }
            return PathBuf::from(home).join(&s[2..]);
        }
    } else if s.starts_with("~") {
        // ~user/... pattern
        let end = s[1..].find('/').map(|i| i + 1).unwrap_or(s.len());
        let user = &s[1..end];
        // Convention: user homes under /home/<user>
        let user_home = PathBuf::from("/home").join(user);
        let rest = if end < s.len() { &s[end + 1..] } else { "" };
        return if rest.is_empty() { user_home } else { user_home.join(rest) };
    }
    path.to_path_buf()
}

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
                Ok(mut file) => {
                    // Expand ~ in key_paths (ssh config files can use ~ paths)
                    for server in &mut file.server {
                        if let Some(ref kp) = server.key_path {
                            let expanded = expand_tilde(kp);
                            if expanded != *kp {
                                server.key_path = Some(expanded);
                            }
                        }
                    }
                    file.server
                }
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

    // Warn if key file has overly permissive permissions
    if let Some(ref kp) = server.key_path {
        if kp.exists() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(meta) = std::fs::metadata(kp) {
                    let mode = meta.permissions().mode() & 0o777;
                    if mode > 0o600 {
                        errors.push(ValidationError {
                            field: "key_path",
                            message: format!(
                                "Key file permissions are {:03o} (should be 600). Run: chmod 600 {}",
                                mode,
                                kp.display()
                            ),
                        });
                    }
                }
            }
        } else {
            errors.push(ValidationError {
                field: "key_path",
                message: format!("Key file not found: {}", kp.display()),
            });
        }
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

/// Parse an OpenSSH config file and extract server bookmarks.
/// Skips wildcard entries (`Host *`) and entries without a HostName.
/// Splits multi-name Host entries (e.g., `Host host1 host2`) into separate servers.
pub fn parse_ssh_config(content: &str) -> Vec<ServerConfig> {
    let mut servers = Vec::new();
    let mut current_names: Vec<String> = Vec::new();
    let mut current_hostname: Option<String> = None;
    let mut current_user = String::new();
    let mut current_port: u16 = 22;
    let mut current_key: Option<PathBuf> = None;
    let mut in_match_block = false;
    let mut match_is_host = false;

    fn flush(
        servers: &mut Vec<ServerConfig>,
        names: &mut Vec<String>,
        hostname: &mut Option<String>,
        user: &mut String,
        port: &mut u16,
        key: &mut Option<PathBuf>,
    ) {
        if let Some(host) = hostname.take() {
            for name in names.drain(..) {
                // Skip wildcard entries
                if name == "*" {
                    continue;
                }
                servers.push(ServerConfig {
                    name: name.clone(),
                    host: host.clone(),
                    user: user.clone(),
                    port: *port,
                    last_path: PathBuf::from("/"),
                    key_path: key.clone(),
                });
            }
        }
        *user = String::new();
        *port = 22;
        *key = None;
    }

    for line in content.lines() {
        let line = line.trim();
        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Split into keyword and arguments
        let mut parts = line.split_whitespace();
        let Some(keyword) = parts.next() else { continue };
        let args: Vec<&str> = parts.collect();

        match keyword.to_ascii_lowercase().as_str() {
            "host" => {
                in_match_block = false;
                match_is_host = false;
                // Flush previous entry
                flush(&mut servers, &mut current_names, &mut current_hostname, &mut current_user, &mut current_port, &mut current_key);
                current_names = args.into_iter().map(|s| s.to_string()).collect();
            }
            "match" => {
                // End any previous block
                flush(&mut servers, &mut current_names, &mut current_hostname, &mut current_user, &mut current_port, &mut current_key);
                // Check if this is "Match host <pattern>"
                if args.len() >= 2 && args[0].to_ascii_lowercase() == "host" {
                    in_match_block = true;
                    match_is_host = true;
                    current_names = args[1..].iter().map(|s| s.to_string()).collect();
                } else {
                    // Skip other Match types (exec, user, etc.)
                    in_match_block = true;
                    match_is_host = false;
                }
            }
            "hostname" => {
                if !in_match_block || match_is_host {
                    if let Some(v) = args.first() {
                        current_hostname = Some(v.to_string());
                    }
                }
            }
            "user" => {
                if !in_match_block || match_is_host {
                    if let Some(v) = args.first() {
                        current_user = v.to_string();
                    }
                }
            }
            "port" => {
                if !in_match_block || match_is_host {
                    if let Some(v) = args.first() {
                        if let Ok(p) = v.parse::<u16>() {
                            current_port = p;
                        }
                    }
                }
            }
            "identityfile" => {
                if !in_match_block || match_is_host {
                    if let Some(v) = args.first() {
                        current_key = Some(expand_tilde(&PathBuf::from(v.to_string())));
                    }
                }
            }
            _ => {}
        }
    }

    // Flush final entry
    flush(&mut servers, &mut current_names, &mut current_hostname, &mut current_user, &mut current_port, &mut current_key);

    servers
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

    #[test]
    fn parse_ssh_config_sample() {
        let config = r#"
# Comments should be ignored
Host myserver
    HostName 192.168.1.1
    User admin
    Port 2222
    IdentityFile ~/.ssh/id_rsa

Host *
    User default

Host server2 server3
    HostName example.com
    User root
"#;
        let servers = parse_ssh_config(config);
        assert_eq!(servers.len(), 3, "Expected 3 servers (Host * should be skipped, server2/server3 split)");
        
        let s0 = servers.iter().find(|s| s.name == "myserver").expect("myserver");
        assert_eq!(s0.host, "192.168.1.1");
        assert_eq!(s0.user, "admin");
        assert_eq!(s0.port, 2222);
        assert!(s0.key_path.is_some());
        
        let s1 = servers.iter().find(|s| s.name == "server2").expect("server2");
        assert_eq!(s1.host, "example.com");
        assert_eq!(s1.user, "root");
        
        let s2 = servers.iter().find(|s| s.name == "server3").expect("server3");
        assert_eq!(s2.host, "example.com");
        assert_eq!(s2.user, "root");
    }

    #[test]
    fn parse_ssh_config_skips_wildcards_and_globals() {
        let config = r#"
Host *
    User default
    Port 9999

Host *
    ForwardAgent yes
"#;
        let servers = parse_ssh_config(config);
        assert_eq!(servers.len(), 0, "Wildcard-only configs should produce no servers");
    }

    #[test]
    fn parse_ssh_config_defaults() {
        let config = r#"
Host simple
    HostName simple.example.com
"#;
        let servers = parse_ssh_config(config);
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].name, "simple");
        assert_eq!(servers[0].host, "simple.example.com");
        assert_eq!(servers[0].user, "");
        assert_eq!(servers[0].port, 22);
        assert!(servers[0].key_path.is_none());
    }

    #[test]
    fn parse_ssh_config_match_host_directive() {
        let config = r#"
Host main
    HostName main.example.com
    User admin

Match host staging
    HostName staging.example.com
    User deploy
    Port 2222
    IdentityFile ~/.ssh/staging.key

Match exec "test %h = prod"
    HostName prod.example.com
    User root
"#;
        let servers = parse_ssh_config(config);
        assert_eq!(servers.len(), 2, "Should have main and staging; exec Match skipped");
        
        let main = servers.iter().find(|s| s.name == "main").unwrap();
        assert_eq!(main.host, "main.example.com");
        assert_eq!(main.user, "admin");
        
        let staging = servers.iter().find(|s| s.name == "staging").unwrap();
        assert_eq!(staging.host, "staging.example.com");
        assert_eq!(staging.user, "deploy");
        assert_eq!(staging.port, 2222);
        assert_eq!(staging.key_path, Some(expand_tilde(&PathBuf::from("~/.ssh/staging.key"))));
    }

    #[test]
    fn expand_tilde_no_tilde_returns_unchanged() {
        let path = std::path::PathBuf::from("/usr/local/bin");
        assert_eq!(expand_tilde(&path), path);
    }

    #[test]
    fn expand_tilde_plain_home() {
        let home = dirs::home_dir().expect("Home dir should exist");
        let result = expand_tilde(std::path::Path::new("~"));
        assert_eq!(result, home);
    }

    #[test]
    fn expand_tilde_home_subpath() {
        let home = dirs::home_dir().expect("Home dir should exist");
        let result = expand_tilde(std::path::Path::new("~/.ssh/id_rsa"));
        assert_eq!(result, home.join(".ssh/id_rsa"));
    }

    #[test]
    fn expand_tilde_user_fallback() {
        let result = expand_tilde(std::path::Path::new("~root/.bashrc"));
        assert_eq!(result, std::path::PathBuf::from("/home/root/.bashrc"));
    }

    #[test]
    fn expand_tilde_user_no_slash() {
        let result = expand_tilde(std::path::Path::new("~nobody"));
        assert_eq!(result, std::path::PathBuf::from("/home/nobody"));
    }
}
