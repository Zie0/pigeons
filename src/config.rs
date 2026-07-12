use std::path::PathBuf;

use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    telemetry_enabled: Option<bool>,
}

impl Config {
    /// Whether sending metrics to the embedded or configured iroh-services endpoint is enabled.
    pub fn telemetry_enabled(&self) -> bool {
        self.telemetry_enabled.unwrap_or(false)
    }

    pub async fn load() -> Result<Self> {
        let config_file_path = Self::config_path()?;
        tokio::fs::create_dir_all(config_file_path.parent().expect("joined path")).await?;

        let mut file = File::options()
            .read(true)
            .create(true)
            .open(&config_file_path)
            .await?;
        let mut config_bytes = Vec::new();
        file.read_to_end(&mut config_bytes).await?;

        let config = toml::from_slice(&config_bytes)
            .context(format!("failed parsing config at {config_file_path:?}"))?;
        Ok(config)
    }

    pub async fn store(&self) -> Result<()> {
        let config_file_path = Self::config_path()?;
        tokio::fs::create_dir_all(config_file_path.parent().expect("joined path")).await?;

        let mut file = File::options()
            .write(true)
            .create(true)
            .open(config_file_path)
            .await?;
        file.write_all(toml::to_string(self)?.as_bytes()).await?;
        Ok(())
    }

    pub fn config_path() -> Result<PathBuf> {
        let config_dir = dirs_next::config_dir()
            .context("can't figure out config dir on this system")?
            .join("pigeons");
        Ok(config_dir.join("config.toml"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_config() {
        let config = toml::from_str::<Config>("").unwrap();
        assert_eq!(config.telemetry_enabled(), false);
    }
}
