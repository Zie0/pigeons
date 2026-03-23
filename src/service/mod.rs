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

#[cfg(target_os = "windows")]
pub async fn run_service(ssh_port: u16, relay_url: Vec<String>) -> anyhow::Result<()> {
    WindowsService::run_service(ServiceParams {
        ssh_port,
        relay_url,
    })
    .await
}

// pub async fn install_sevice(ssh_port: u16, relay_url: Vec<String>) -> anyhow::Result<()> {
//     if crate::service::install(ServiceParams {
//         ssh_port,
//         relay_url,
//     })
//     .await
//     .is_err()
//     {
//         anyhow::bail!("coop installation is only supported on linux, macos, and windows");
//     }

//     // Give the daemon a moment to start and generate keys
//     tokio::time::sleep(std::time::Duration::from_secs(2)).await;

//     match dot_ssh_secret_key(&SecretKey::generate(&mut rand::rng()), false, true) {
//         Ok(key) => {
//             println!();
//             print_roost_info(key.public(), ssh_port);
//         }
//         Err(_) => {
//             println!("Service installed. Run 'pigeons info' to see your roost ID.");
//         }
//     }

//     Ok(())
// }

// pub async fn uninstall_service() -> anyhow::Result<()> {
//     if crate::service::uninstall().await.is_err() {
//         anyhow::bail!("coop removal is only supported on linux, macos, or windows");
//     }
//     Ok(())
// }

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

pub async fn install(_service_params: ServiceParams) -> anyhow::Result<()> {
    match std::env::consts::OS {
        #[cfg(target_os = "linux")]
        "linux" => LinuxService::install(_service_params).await,
        #[cfg(target_os = "macos")]
        "macos" => MacosService::install(_service_params).await,
        #[cfg(target_os = "windows")]
        "windows" => WindowsService::install(_service_params).await,
        _ => anyhow::bail!("service mode is only supported on linux, macos, and windows"),
    }
}

pub async fn uninstall() -> anyhow::Result<()> {
    match std::env::consts::OS {
        #[cfg(target_os = "linux")]
        "linux" => LinuxService::uninstall().await,
        #[cfg(target_os = "windows")]
        "windows" => WindowsService::uninstall().await,
        _ => anyhow::bail!("service mode is only supported on linux and windows"),
    }
}
