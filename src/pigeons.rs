// use std::path::PathBuf;

// use anyhow::Result;
// use iroh::{EndpointId, RelayUrl, SecretKey};
// use tokio::net::TcpStream;

// use crate::{
//     Tunnel, dot_ssh_secret_key,
//     ssh::home_ssh_dir,
//     tunnel::{PigeonConfig, RoostConfig, TunnelBuilder},
// };

// pub struct RunArgs {
//     pub roost_config: Option<RoostConfig>,
//     pub relay_urls: Vec<RelayUrl>,
//     pub config_path: Option<PathBuf>,
//     pub ssh_dir: Option<PathBuf>,
// }

// pub async fn run(args: RunArgs) -> Result<()> {
//     let secret_key = if args.ephemeral {
//         println!(
//             "  Warning: using temporary nest. Run with --persist / -p so pigeons can find their way back."
//         );
//         SecretKey::generate(&mut rand::rng())
//     } else {
//         let ssh_dir = home_ssh_dir()?;
//         dot_ssh_secret_key(ssh_dir, true)?
//     };

//     let mut tunnel = Tunnel::open(TunnelBuilder {
//         roost: Some(RoostConfig {
//             ssh_port: args.ssh_port,
//         }),
//         pigeons: vec![],
//         secret_key,
//         relay_urls: args.relay_urls,
//         isvc_client_secret: None,
//     })
//     .await?;

//     // print_roost_info(tunnel.endpoint().id(), args.ssh_port);

//     println!();
//     println!("  Awaiting pigeons... (Ctrl+C to close the roost)");

//     tokio::signal::ctrl_c().await?;
//     tunnel.close().await?;
//     Ok(())
// }

// pub struct RunCarryArgs {
//     pub endpoint_id: EndpointId,
//     pub tunnel_name: String,
//     pub relay_urls: Vec<RelayUrl>,
//     pub bind_port: Option<u16>,
//     pub no_ssh_config: bool,
// }

// pub async fn run_carry(_args: RunCarryArgs) -> Result<()> {
//     todo!();
//     // let tunnel_name = args.tunnel_name;
//     // let tunnel = Tunnel::builder()
//     //     .relay_urls(args.relay_urls.clone())
//     //     .build()
//     //     .await?;

//     // let bind_addr = format!("127.0.0.1:{}", args.bind_port.unwrap_or(0));
//     // let listener = TcpListener::bind(&bind_addr).await?;
//     // let local_port = listener.local_addr()?.port();

//     // let manage_ssh_config = !args.no_ssh_config;
//     // if manage_ssh_config {
//     //     ssh::add_tunnel_host(&tunnel_name, local_port)?;
//     //     println!("Trained pigeon route '{tunnel_name}' in ~/.ssh/config");
//     // }

//     // println!();
//     // println!(
//     //     "Pigeon '{}' perched on 127.0.0.1:{}",
//     //     &tunnel_name, local_port
//     // );
//     // println!();
//     // println!("  Fly with: ssh <user>@{tunnel_name}");
//     // println!();
//     // println!("  Waiting for messages to carry... (Ctrl+C to recall)");

//     // let result = accept_loop(&tunnel, &listener, args.endpoint_id).await;

//     // if manage_ssh_config {
//     //     if let Err(e) = ssh::remove_tunnel_host(&tunnel_name) {
//     //         eprintln!("Warning: couldn't clean up pigeon route: {e}");
//     //     } else {
//     //         println!("Pigeon '{}' returned to the coop.", tunnel_name);
//     //     }
//     // }

//     // result
// }

// pub struct RunDaemonArgs {
//     pub endpoint_id: EndpointId,
//     pub tunnel_name: String,
//     pub relay_urls: Vec<RelayUrl>,
//     pub bind_port: Option<u16>,
//     pub no_ssh_config: bool,
// }

// pub async fn run_daemon(_cfg: RunDaemonArgs) -> Result<()> {
//     todo!();
// }

// pub async fn status() -> Result<()> {
//     todo!();
// }

// pub async fn list_pigeons() -> anyhow::Result<()> {
//     todo!();
// }

// pub async fn remove(tunnel_name: &str) -> Result<()> {
//     todo!();
// }

// pub struct InstallServiceArgs {}

// pub async fn install_service(_args: InstallServiceArgs) -> Result<()> {
//     todo!();
// }

// pub async fn uninstall_service() -> Result<()> {
//     todo!();
// }
