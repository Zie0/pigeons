use anyhow::bail;
use ed25519_dalek::SECRET_KEY_LENGTH;
use homedir::my_home;
use std::sync::Arc;

use iroh::{
    Endpoint, RelayConfig, RelayUrl, SecretKey,
    endpoint::{RelayMode, presets},
    protocol::Router,
};

use crate::{protocol::PigeonsProtocol, ssh::dot_ssh_secret_key};

#[derive(Debug, Clone)]
pub struct Tunnel {
    router: Router,
}

impl Tunnel {
    pub fn builder() -> TunnelBuilder {
        TunnelBuilder::default()
    }

    pub fn endpoint(&self) -> &Endpoint {
        self.router.endpoint()
    }
}

#[derive(Debug, Clone)]
pub struct TunnelBuilder {
    secret_key: SecretKey,
    accept_incoming: bool,
    accept_port: Option<u16>,
    relay_urls: Vec<RelayUrl>,
    extra_relay_urls: Vec<RelayUrl>,
}

impl Default for TunnelBuilder {
    fn default() -> Self {
        TunnelBuilder {
            secret_key: SecretKey::generate(&mut rand::rng()),
            accept_incoming: false,
            accept_port: None,
            relay_urls: Vec::new(),
            extra_relay_urls: Vec::new(),
        }
    }
}

impl TunnelBuilder {
    pub fn accept_incoming(mut self, accept_incoming: bool) -> Self {
        self.accept_incoming = accept_incoming;
        self
    }

    pub fn accept_port(mut self, accept_port: u16) -> Self {
        self.accept_port = Some(accept_port);
        self
    }

    pub fn secret_key(mut self, secret_key: SecretKey) -> Self {
        self.secret_key = secret_key;
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

        match dot_ssh_secret_key(&self.secret_key, persist, service) {
            Ok(secret_key) => {
                tracing::info!("dot_ssh_integration: Successfully loaded/created SSH keys");
                self.secret_key = secret_key;
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

    pub async fn build(&mut self) -> anyhow::Result<Tunnel> {
        let mut builder = Endpoint::builder(presets::N0).secret_key(self.secret_key.clone());

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
        let mut router = Router::builder(endpoint.clone());

        if self.accept_incoming {
            let ssh_port = self.accept_port.unwrap_or(22);
            let handler = PigeonsProtocol::new(ssh_port);
            router = router.accept(PigeonsProtocol::ALPN, handler);
        }

        let router = router.spawn();

        Ok(Tunnel { router })
    }
}
