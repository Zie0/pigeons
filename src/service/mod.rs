#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use crate::service::linux::LinuxService;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use crate::service::macos::MacosService;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub(crate) use crate::service::windows::WindowsService;

#[derive(Debug, Clone)]
pub struct ServiceParams {
    pub ssh_port: u16,
    pub relay_url: Vec<String>,
}

pub trait Service {
    fn install(
        service_params: ServiceParams,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send;
    fn info() -> impl std::future::Future<Output = anyhow::Result<()>> + Send;
    fn uninstall() -> impl std::future::Future<Output = anyhow::Result<()>> + Send;
}

pub async fn install(service_params: ServiceParams) -> anyhow::Result<()> {
    match std::env::consts::OS {
        #[cfg(target_os = "linux")]
        "linux" => LinuxService::install(service_params).await,
        #[cfg(target_os = "macos")]
        "macos" => MacosService::install(service_params).await,
        #[cfg(target_os = "windows")]
        "windows" => WindowsService::install(service_params).await,
        _ => anyhow::bail!("service mode is only supported on linux, macos, and windows"),
    }
}

pub async fn uninstall() -> anyhow::Result<()> {
    match std::env::consts::OS {
        #[cfg(target_os = "linux")]
        "linux" => LinuxService::uninstall().await,
        #[cfg(target_os = "macos")]
        "macos" => MacosService::uninstall().await,
        #[cfg(target_os = "windows")]
        "windows" => WindowsService::uninstall().await,
        _ => anyhow::bail!("service mode is only supported on linux, macos, and windows"),
    }
}

/// Try to read the endpoint ID of the installed pigeons service.
/// The install script copies the public key to /etc/pigeons/endpoint_id
/// so unprivileged users can read it.
/// Returns Some(endpoint_id) if found, None otherwise.
pub fn service_endpoint_id() -> Option<iroh::EndpointId> {
    let pub_key_bytes = std::fs::read("/etc/pigeons/endpoint_id").ok()?;
    let decoded = z32::decode(&pub_key_bytes).ok()?;
    let bytes: [u8; 32] = decoded.as_slice().try_into().ok()?;
    let public_key = iroh::PublicKey::from_bytes(&bytes).ok()?;
    Some(public_key.into())
}
