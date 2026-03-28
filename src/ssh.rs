use std::{fmt, path::PathBuf};

use anyhow::{Context, bail};
use ed25519_dalek::SECRET_KEY_LENGTH;
use homedir::my_home;
use iroh::SecretKey;
use tokio::net::TcpStream;

pub fn home_ssh_dir() -> anyhow::Result<PathBuf> {
    let distro_home = my_home()?.ok_or_else(|| anyhow::anyhow!("home directory not found"))?;
    #[allow(unused_mut)]
    let mut ssh_dir = distro_home.join(".ssh");

    Ok(ssh_dir)
}

pub fn dot_ssh_secret_key(ssh_dir: PathBuf, persist: bool) -> anyhow::Result<SecretKey> {
    tracing::info!("dot_ssh: Function called, persist={}", persist);

    let pub_key = ssh_dir.join("pigeons_ed25519.pub");
    let priv_key = ssh_dir.join("pigeons_ed25519");

    tracing::debug!("dot_ssh: ssh_dir exists = {}", ssh_dir.exists());
    tracing::debug!("dot_ssh: pub_key path = {}", pub_key.display());
    tracing::debug!("dot_ssh: priv_key path = {}", priv_key.display());

    match (ssh_dir.exists(), persist) {
        (false, false) => {
            bail!(
                "no .ssh folder found in {}, use --persist flag to create it",
                ssh_dir.display()
            )
        }
        (false, true) => {
            std::fs::create_dir_all(&ssh_dir)?;
            println!("[INFO] created .ssh folder: {}", ssh_dir.display());
            dot_ssh_secret_key(ssh_dir, persist)
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
                let secret_key = SecretKey::generate(&mut rand::rng());
                let public_key = secret_key.public();

                std::fs::write(&pub_key, z32::encode(public_key.as_bytes()))?;
                std::fs::write(&priv_key, z32::encode(&secret_key.to_bytes()))?;

                Ok(secret_key)
            }
        }
        (true, false) => {
            if pub_key.exists()
                && priv_key.exists()
                && let Ok(secret_key) = std::fs::read(&priv_key)
            {
                let mut sk_bytes = [0u8; SECRET_KEY_LENGTH];
                sk_bytes.copy_from_slice(z32::decode(secret_key.as_slice())?.as_slice());
                return Ok(SecretKey::from_bytes(&sk_bytes));
            }
            bail!(
                "no pigeon keys found in {}, use --persist flag to create them",
                ssh_dir.display()
            )
        }
    }
}

fn ssh_config_path() -> anyhow::Result<PathBuf> {
    let home = my_home()?.ok_or_else(|| anyhow::anyhow!("home directory not found"))?;
    Ok(home.join(".ssh").join("config"))
}

#[derive(Debug, Clone)]
pub struct SshConfigPigeonEntry {
    pub name: String,
    pub endpoint_id: String,
}

impl fmt::Display for SshConfigPigeonEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} -> {}", self.name, self.endpoint_id)
    }
}

/// Add or update a pigeon host entry in ~/.ssh/config using ProxyCommand
pub fn add_tunnel_host(name: &str, endpoint_id: &str) -> anyhow::Result<()> {
    tracing::debug!("adding tunnel host name={name} endpoint={endpoint_id}");
    let config_path = ssh_config_path()?;

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
    let cleaned = remove_host_block(&existing, name);

    let block = format!(
        "Host {name}\n\
         \x20   ProxyCommand pigeons fly --stdio {endpoint_id}\n\
         \x20   UserKnownHostsFile /dev/null\n\
         \x20   StrictHostKeyChecking no\n"
    );

    let mut new_content = cleaned;
    if !new_content.is_empty() && !new_content.ends_with('\n') {
        new_content.push('\n');
    }
    if !new_content.is_empty() {
        new_content.push('\n');
    }
    new_content.push_str(&block);

    atomic_write(&config_path, &new_content)
}

/// Remove a pigeon host entry from ~/.ssh/config
pub fn remove_tunnel_host(name: &str) -> anyhow::Result<()> {
    tracing::debug!("removing tunnel host name={name}");
    let config_path = ssh_config_path()?;

    if !config_path.exists() {
        bail!("no ssh config found at {}", config_path.display());
    }

    let existing = std::fs::read_to_string(&config_path)
        .with_context(|| format!("failed to read {}", config_path.display()))?;

    let cleaned = remove_host_block(&existing, name);

    if cleaned == existing {
        bail!("no pigeon entry '{}' found in ssh config", name);
    }

    atomic_write(&config_path, &cleaned)
}

/// List all pigeons-managed entries in ~/.ssh/config by finding Host blocks
/// whose ProxyCommand starts with "pigeons fly"
pub fn list_tunnel_hosts() -> anyhow::Result<Vec<SshConfigPigeonEntry>> {
    let config_path = ssh_config_path()?;

    if !config_path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&config_path)
        .with_context(|| format!("failed to read {}", config_path.display()))?;

    let mut entries = Vec::new();
    let mut current_host: Option<String> = None;
    let mut current_endpoint: Option<String> = None;

    for line in content.lines() {
        let trimmed = line.trim();

        if let Some(rest) = trimmed.strip_prefix("Host ") {
            // Flush previous block if it was a pigeon entry
            if let (Some(host), Some(endpoint)) = (current_host.take(), current_endpoint.take()) {
                entries.push(SshConfigPigeonEntry {
                    name: host,
                    endpoint_id: endpoint,
                });
            }
            current_host = Some(rest.trim().to_string());
            current_endpoint = None;
        } else if let Some(proxy_cmd) = trimmed.strip_prefix("ProxyCommand ") {
            if let Some(endpoint_id) = parse_pigeons_proxy_command(proxy_cmd.trim()) {
                current_endpoint = Some(endpoint_id);
            }
        }
    }

    // Flush last block
    if let (Some(host), Some(endpoint)) = (current_host, current_endpoint) {
        entries.push(SshConfigPigeonEntry {
            name: host,
            endpoint_id: endpoint,
        });
    }

    Ok(entries)
}

/// Parse a ProxyCommand value like "pigeons fly --stdio <endpoint_id>"
/// and return the endpoint_id if it matches
fn parse_pigeons_proxy_command(cmd: &str) -> Option<String> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    // expect: ["pigeons", "fly", "--stdio", "<endpoint_id>"]
    if parts.len() >= 4
        && parts[0] == "pigeons"
        && parts[1] == "fly"
        && parts[2] == "--stdio"
    {
        Some(parts[3].to_string())
    } else {
        None
    }
}

/// Remove a Host block by name from ssh config content.
/// A Host block starts with "Host <name>" and ends at the next "Host " line
/// or end of file.
fn remove_host_block(content: &str, name: &str) -> String {
    let mut result = String::new();
    let mut skipping = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if let Some(rest) = trimmed.strip_prefix("Host ") {
            if rest.trim() == name {
                skipping = true;
                continue;
            } else {
                skipping = false;
            }
        }

        if skipping {
            continue;
        }

        result.push_str(line);
        result.push('\n');
    }

    // Trim trailing blank lines
    while result.ends_with("\n\n") {
        result.pop();
    }

    result
}

fn atomic_write(path: &PathBuf, content: &str) -> anyhow::Result<()> {
    let dir = path.parent().unwrap();
    let temp_path = dir.join(".config.pigeons.tmp");
    std::fs::write(&temp_path, content)
        .with_context(|| format!("failed to write {}", temp_path.display()))?;
    std::fs::rename(&temp_path, path)
        .with_context(|| format!("failed to rename temp file to {}", path.display()))?;
    Ok(())
}

pub(crate) async fn ensure_local_ssh_server_exists(ssh_port: u16) -> anyhow::Result<()> {
    tracing::debug!("probing sshd on port {ssh_port}");
    match TcpStream::connect(format!("127.0.0.1:{}", ssh_port)).await {
        Ok(_) => {
            tracing::debug!("sshd found on port {ssh_port}");
            Ok(())
        }
        Err(_) => Err(anyhow::anyhow!(format!(
            "no sshd detected on port {ssh_port}. Make sure sshd is running before sending pigeons to this roost",
        ))),
    }
}
