use std::{
    io,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use tokio::{
    fs::{self, File},
    io::AsyncWriteExt,
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
        Self::load_from(&Self::config_path()?).await
    }

    /// Read and parse the config at `path`. Loading is read-only: a config that
    /// has not been written yet simply yields the defaults.
    async fn load_from(path: &Path) -> Result<Self> {
        let config_bytes = match fs::read(path).await {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(err) => {
                return Err(err)
                    .with_context(|| format!("failed reading config at {}", path.display()));
            }
        };

        toml::from_slice(&config_bytes)
            .with_context(|| format!("failed parsing config at {}", path.display()))
    }

    /// Loads the config, falling back to the default config on error.
    pub async fn load_or_default() -> Self {
        match Self::load().await {
            Ok(config) => config,
            Err(err) => {
                tracing::error!("failed to load config, using default: {err:#?}");
                Self::default()
            }
        }
    }

    pub async fn store(&self) -> Result<()> {
        let config_file_path = Self::config_path()?;
        fs::create_dir_all(config_file_path.parent().expect("joined path")).await?;

        let mut file = File::options()
            .write(true)
            .truncate(true)
            .create(true)
            .open(config_file_path)
            .await?;
        file.write_all(toml::to_string(self)?.as_bytes()).await?;
        Ok(())
    }

    pub fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
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
        assert!(!config.telemetry_enabled());
    }

    /// Not having written a config yet is the normal case, not an error.
    #[tokio::test]
    async fn load_from_missing_file_yields_defaults() {
        let dir = tempfile::tempdir().unwrap();

        let config = Config::load_from(&dir.path().join("config.toml"))
            .await
            .unwrap();

        assert!(!config.telemetry_enabled());
    }

    /// Regression: `load` used to open the file with `create(true)` and no write
    /// access, which fails unconditionally, so settings on disk were silently
    /// discarded in favour of the defaults.
    #[tokio::test]
    async fn load_from_reads_settings_off_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "telemetry_enabled = true").await.unwrap();

        let config = Config::load_from(&path).await.unwrap();

        assert!(
            config.telemetry_enabled(),
            "settings on disk must take precedence over the defaults"
        );
    }
}
