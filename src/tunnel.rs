use crate::{Builder, Inner, IrohTunnel};

use anyhow::bail;
use ed25519_dalek::SECRET_KEY_LENGTH;
use homedir::my_home;
use std::sync::Arc;

use iroh::{
    Endpoint, EndpointId, RelayConfig, RelayUrl, SecretKey,
    endpoint::{Connection, RelayMode, presets},
    protocol::{ProtocolHandler, Router},
};
use tokio::net::TcpStream;

impl Builder {
    pub fn new() -> Self {
        Self {
            secret_key: SecretKey::generate(&mut rand::rng()).to_bytes(),
            accept_incoming: false,
            accept_port: None,
            relay_urls: Vec::new(),
            extra_relay_urls: Vec::new(),
        }
    }

    pub fn accept_incoming(mut self, accept_incoming: bool) -> Self {
        self.accept_incoming = accept_incoming;
        self
    }

    pub fn accept_port(mut self, accept_port: u16) -> Self {
        self.accept_port = Some(accept_port);
        self
    }

    pub fn secret_key(mut self, secret_key: &[u8; SECRET_KEY_LENGTH]) -> Self {
        self.secret_key = *secret_key;
        self
    }

    pub fn relay_urls(mut self, urls: Vec<RelayUrl>) -> Self {
        self.relay_urls = urls;
        self
    }

    pub fn extra_relay_urls(mut self, urls: Vec<RelayUrl>) -> Self {
        self.extra_relay_urls = urls;
        self
    }

    pub fn dot_ssh_integration(mut self, persist: bool, service: bool) -> Self {
        tracing::info!(
            "dot_ssh_integration: persist={}, service={}",
            persist,
            service
        );

        match dot_ssh(&SecretKey::from_bytes(&self.secret_key), persist, service) {
            Ok(secret_key) => {
                tracing::info!("dot_ssh_integration: Successfully loaded/created SSH keys");
                self.secret_key = secret_key.to_bytes();
            }
            Err(e) => {
                tracing::error!(
                    "dot_ssh_integration: Failed to load/create SSH keys: {:#}",
                    e
                );
                eprintln!("Warning: Failed to load/create persistent SSH keys: {e:#}");
                eprintln!("Continuing with ephemeral keys...");
            }
        }
        self
    }

    pub async fn build(&mut self) -> anyhow::Result<IrohTunnel> {
        let secret_key = SecretKey::from_bytes(&self.secret_key);
        let mut builder = Endpoint::builder(presets::N0).secret_key(secret_key);

        if !self.relay_urls.is_empty() {
            let relay_map = self.relay_urls.iter().cloned().collect();
            builder = builder.relay_mode(RelayMode::Custom(relay_map));
        } else if !self.extra_relay_urls.is_empty() {
            let relay_map = RelayMode::Default.relay_map();
            for url in &self.extra_relay_urls {
                relay_map.insert(url.clone(), Arc::new(RelayConfig::from(url.clone())));
            }
            builder = builder.relay_mode(RelayMode::Custom(relay_map));
        }

        let endpoint = builder.bind().await?;

        let mut tunnel = IrohTunnel {
            public_key: *endpoint.id().as_bytes(),
            secret_key: self.secret_key,
            inner: None,
            ssh_port: self.accept_port.unwrap_or(22),
        };

        let router = if self.accept_incoming {
            Router::builder(endpoint.clone()).accept(IrohTunnel::ALPN, tunnel.clone())
        } else {
            Router::builder(endpoint.clone())
        }
        .spawn();

        tunnel.add_inner(endpoint, router);

        Ok(tunnel)
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}

impl IrohTunnel {
    pub const ALPN: &[u8] = b"/iroh-tunnel/0";

    pub fn builder() -> Builder {
        Builder::new()
    }

    fn add_inner(&mut self, endpoint: Endpoint, router: Router) {
        self.inner = Some(Inner { endpoint, router });
    }

    pub fn endpoint(&self) -> &Endpoint {
        &self.inner.as_ref().expect("inner not set").endpoint
    }

    pub fn endpoint_id(&self) -> EndpointId {
        self.inner.as_ref().expect("inner not set").endpoint.id()
    }
}

impl ProtocolHandler for IrohTunnel {
    async fn accept(&self, connection: Connection) -> Result<(), iroh::protocol::AcceptError> {
        let endpoint_id = connection.remote_id();

        match connection.accept_bi().await {
            Ok((mut iroh_send, mut iroh_recv)) => {
                println!("Pigeon arrived from {endpoint_id}");

                match TcpStream::connect(format!("127.0.0.1:{}", self.ssh_port)).await {
                    Ok(mut ssh_stream) => {
                        println!("Delivering to local sshd on port {}", self.ssh_port);

                        let (mut local_read, mut local_write) = ssh_stream.split();

                        let a_to_b =
                            async move { tokio::io::copy(&mut local_read, &mut iroh_send).await };
                        let b_to_a =
                            async move { tokio::io::copy(&mut iroh_recv, &mut local_write).await };

                        tokio::select! {
                            result = a_to_b => {
                                let _ = result;
                                println!("Pigeon from {endpoint_id} returned home.");
                            },
                            result = b_to_a => {
                                let _ = result;
                                println!("Pigeon from {endpoint_id} returned home.");
                            },
                        };
                    }
                    Err(e) => {
                        println!("Pigeon couldn't reach sshd: {e}");
                    }
                }
            }
            Err(e) => {
                println!("Pigeon dropped its message: {e}");
            }
        }

        Ok(())
    }
}

pub fn dot_ssh(
    default_secret_key: &SecretKey,
    persist: bool,
    _service: bool,
) -> anyhow::Result<SecretKey> {
    tracing::info!(
        "dot_ssh: Function called, persist={}, service={}",
        persist,
        _service
    );

    let distro_home = my_home()?.ok_or_else(|| anyhow::anyhow!("home directory not found"))?;
    #[allow(unused_mut)]
    let mut ssh_dir = distro_home.join(".ssh");

    #[cfg(target_os = "linux")]
    if _service {
        ssh_dir = std::path::PathBuf::from("/root/.ssh");
    }

    #[cfg(target_os = "macos")]
    if _service {
        ssh_dir = std::path::PathBuf::from("/var/root/.ssh");
    }

    #[cfg(target_os = "windows")]
    if _service {
        ssh_dir = std::path::PathBuf::from(crate::service::WindowsService::SERVICE_SSH_DIR);
        tracing::info!("dot_ssh: Using service SSH dir: {}", ssh_dir.display());

        if !ssh_dir.exists() {
            tracing::info!("dot_ssh: Service SSH dir doesn't exist, creating it");
            std::fs::create_dir_all(&ssh_dir)?;
        }
    }

    let pub_key = ssh_dir.join("iroh_tunnel_ed25519.pub");
    let priv_key = ssh_dir.join("iroh_tunnel_ed25519");

    // Also check legacy key names for backward compat
    let legacy_pub_key = ssh_dir.join("irohssh_ed25519.pub");
    let legacy_priv_key = ssh_dir.join("irohssh_ed25519");

    tracing::debug!("dot_ssh: ssh_dir exists = {}", ssh_dir.exists());
    tracing::debug!("dot_ssh: pub_key path = {}", pub_key.display());
    tracing::debug!("dot_ssh: priv_key path = {}", priv_key.display());

    // Determine which key files to use (prefer new names, fall back to legacy)
    let (active_pub, active_priv) = if pub_key.exists() && priv_key.exists() {
        (pub_key, priv_key)
    } else if legacy_pub_key.exists() && legacy_priv_key.exists() {
        (legacy_pub_key, legacy_priv_key)
    } else {
        (pub_key, priv_key)
    };

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
            dot_ssh(default_secret_key, persist, _service)
        }
        (true, true) => {
            if active_pub.exists() && active_priv.exists() {
                if let Ok(secret_key) = std::fs::read(&active_priv) {
                    let mut sk_bytes = [0u8; SECRET_KEY_LENGTH];
                    sk_bytes.copy_from_slice(z32::decode(secret_key.as_slice())?.as_slice());
                    Ok(SecretKey::from_bytes(&sk_bytes))
                } else {
                    bail!("failed to read secret key from {}", active_priv.display())
                }
            } else {
                let secret_key = default_secret_key.clone();
                let public_key = secret_key.public();

                std::fs::write(&active_pub, z32::encode(public_key.as_bytes()))?;
                std::fs::write(&active_priv, z32::encode(&secret_key.to_bytes()))?;

                Ok(secret_key)
            }
        }
        (true, false) => {
            if active_pub.exists() && active_priv.exists() {
                if let Ok(secret_key) = std::fs::read(&active_priv) {
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
