use std::fmt;
use std::path::PathBuf;

use anyhow::{Context, bail};
use ed25519_dalek::SECRET_KEY_LENGTH;
use homedir::my_home;
use iroh::SecretKey;

pub(crate) fn dot_ssh_secret_key(
    default_secret_key: &SecretKey,
    persist: bool,
    service: bool,
) -> anyhow::Result<SecretKey> {
    tracing::info!(
        "dot_ssh: Function called, persist={}, service={}",
        persist,
        service
    );

    let distro_home = my_home()?.ok_or_else(|| anyhow::anyhow!("home directory not found"))?;
    #[allow(unused_mut)]
    let mut ssh_dir = distro_home.join(".ssh");

    #[cfg(target_os = "linux")]
    if service {
        ssh_dir = std::path::PathBuf::from("/root/.ssh");
    }

    #[cfg(target_os = "macos")]
    if service {
        ssh_dir = std::path::PathBuf::from("/var/root/.ssh");
    }

    #[cfg(target_os = "windows")]
    if service {
        ssh_dir = std::path::PathBuf::from(crate::service::WindowsService::SERVICE_SSH_DIR);
        tracing::info!("dot_ssh: Using service SSH dir: {}", ssh_dir.display());

        if !ssh_dir.exists() {
            tracing::info!("dot_ssh: Service SSH dir doesn't exist, creating it");
            std::fs::create_dir_all(&ssh_dir)?;
        }
    }

    let pub_key = ssh_dir.join("pigeons_ed25519.pub");
    let priv_key = ssh_dir.join("pigeons_ed25519");

    tracing::debug!("dot_ssh: ssh_dir exists = {}", ssh_dir.exists());
    tracing::debug!("dot_ssh: pub_key path = {}", pub_key.display());
    tracing::debug!("dot_ssh: priv_key path = {}", priv_key.display());

    match (ssh_dir.exists(), persist) {
        (false, false) => {
            bail!(
                "no .ssh folder found in {}, use --persist flag to create it",
                distro_home.display()
            )
        }
        (false, true) => {
            std::fs::create_dir_all(&ssh_dir)?;
            println!("[INFO] created .ssh folder: {}", ssh_dir.display());
            dot_ssh_secret_key(default_secret_key, persist, service)
        }
        (true, true) => {
            if pub_key.exists() && priv_key.exists() {
                if let Ok(secret_key) = std::fs::read(&priv_key) {
                    let mut sk_bytes = [0u8; SECRET_KEY_LENGTH];
                    sk_bytes.copy_from_slice(z32::decode(secret_key.as_slice())?.as_slice());
                    Ok(SecretKey::from_bytes(&sk_bytes))
                } else {
                    bail!("failed to read secret key from {}", priv_key.display())
                }
            } else {
                let secret_key = default_secret_key.clone();
                let public_key = secret_key.public();

                std::fs::write(&pub_key, z32::encode(public_key.as_bytes()))?;
                std::fs::write(&priv_key, z32::encode(&secret_key.to_bytes()))?;

                Ok(secret_key)
            }
        }
        (true, false) => {
            if pub_key.exists() && priv_key.exists() {
                if let Ok(secret_key) = std::fs::read(&priv_key) {
                    let mut sk_bytes = [0u8; SECRET_KEY_LENGTH];
                    sk_bytes.copy_from_slice(z32::decode(secret_key.as_slice())?.as_slice());
                    return Ok(SecretKey::from_bytes(&sk_bytes));
                }
            }
            bail!(
                "no pigeon keys found in {}, use --persist flag to create them",
                ssh_dir.display()
            )
        }
    }
}

const BEGIN_MARKER: &str = "# <pigeons>";
const END_MARKER: &str = "# </pigeons>";

#[derive(Debug, Clone)]
pub struct SshConfigTunnelEntry {
    pub name: String,
    pub port: u16,
}

impl fmt::Display for SshConfigTunnelEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} -> 127.0.0.1:{}", self.name, self.port)
    }
}

fn ssh_config_path() -> anyhow::Result<PathBuf> {
    let home = my_home()?.ok_or_else(|| anyhow::anyhow!("home directory not found"))?;
    Ok(home.join(".ssh").join("config"))
}

/// Add or update a tunnel host entry in ~/.ssh/config
pub fn add_tunnel_host(name: &str, port: u16) -> anyhow::Result<()> {
    let config_path = ssh_config_path()?;

    // Ensure ~/.ssh directory exists
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let existing = if config_path.exists() {
        std::fs::read_to_string(&config_path)
            .with_context(|| format!("failed to read {}", config_path.display()))?
    } else {
        String::new()
    };

    // Remove existing entry for this name if present
    let cleaned = remove_entry_from_content(&existing, name);

    // Build new entry
    let block = format!(
        "{BEGIN_MARKER}{name}\n\
         Host {name}\n\
         \x20   HostName 127.0.0.1\n\
         \x20   Port {port}\n\
         \x20   UserKnownHostsFile /dev/null\n\
         \x20   StrictHostKeyChecking no\n\
         {END_MARKER}{name}\n"
    );

    // Append to config
    let mut new_content = cleaned;
    if !new_content.is_empty() && !new_content.ends_with('\n') {
        new_content.push('\n');
    }
    if !new_content.is_empty() {
        new_content.push('\n');
    }
    new_content.push_str(&block);

    // Write atomically: write to temp file then rename
    let dir = config_path.parent().unwrap();
    let temp_path = dir.join(".config.pigeons.tmp");
    std::fs::write(&temp_path, &new_content)
        .with_context(|| format!("failed to write {}", temp_path.display()))?;
    std::fs::rename(&temp_path, &config_path)
        .with_context(|| format!("failed to rename temp file to {}", config_path.display()))?;

    Ok(())
}

/// Remove a tunnel host entry from ~/.ssh/config
pub fn remove_tunnel_host(name: &str) -> anyhow::Result<()> {
    let config_path = ssh_config_path()?;

    if !config_path.exists() {
        bail!("no ssh config found at {}", config_path.display());
    }

    let existing = std::fs::read_to_string(&config_path)
        .with_context(|| format!("failed to read {}", config_path.display()))?;

    let cleaned = remove_entry_from_content(&existing, name);

    if cleaned == existing {
        bail!("no tunnel entry '{}' found in ssh config", name);
    }

    let dir = config_path.parent().unwrap();
    let temp_path = dir.join(".config.pigeons.tmp");
    std::fs::write(&temp_path, &cleaned)?;
    std::fs::rename(&temp_path, &config_path)?;

    Ok(())
}

/// List all iroh-tunnel managed entries in ~/.ssh/config
pub fn list_tunnel_hosts() -> anyhow::Result<Vec<SshConfigTunnelEntry>> {
    let config_path = ssh_config_path()?;

    if !config_path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&config_path)
        .with_context(|| format!("failed to read {}", config_path.display()))?;

    let mut entries = Vec::new();
    let mut in_block = false;
    let mut current_name = String::new();
    let mut current_port: Option<u16> = None;

    for line in content.lines() {
        if let Some(name) = line.strip_prefix(BEGIN_MARKER) {
            in_block = true;
            current_name = name.trim().to_string();
            current_port = None;
        } else if line.starts_with(END_MARKER) && in_block {
            if let Some(port) = current_port {
                entries.push(SshConfigTunnelEntry {
                    name: current_name.clone(),
                    port,
                });
            }
            in_block = false;
        } else if in_block {
            let trimmed = line.trim();
            if let Some(port_str) = trimmed.strip_prefix("Port ") {
                current_port = port_str.trim().parse().ok();
            }
        }
    }

    Ok(entries)
}

/// Remove a named entry from ssh config content, returning the cleaned string
fn remove_entry_from_content(content: &str, name: &str) -> String {
    let begin = format!("{BEGIN_MARKER}{name}");
    let end = format!("{END_MARKER}{name}");

    let mut result = String::new();
    let mut in_block = false;
    let mut skip_next_blank = false;

    for line in content.lines() {
        if line.starts_with(&begin) {
            in_block = true;
            skip_next_blank = true;
            continue;
        }
        if line.starts_with(&end) && in_block {
            in_block = false;
            continue;
        }
        if in_block {
            continue;
        }
        // Skip blank lines that immediately preceded or follow a removed block
        if skip_next_blank && line.trim().is_empty() {
            skip_next_blank = false;
            continue;
        }
        skip_next_blank = false;
        result.push_str(line);
        result.push('\n');
    }

    // Trim trailing whitespace
    while result.ends_with("\n\n") {
        result.pop();
    }

    result
}
