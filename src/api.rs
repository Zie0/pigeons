use anyhow::bail;
use homedir::my_home;
use iroh::{EndpointId, RelayUrl, SecretKey};
use tokio::net::{TcpListener, TcpStream};

use crate::{
    protocol::PigeonsProtocol, service::ServiceParams, ssh, ssh::dot_ssh_secret_key, tunnel::Tunnel,
};

pub async fn info_mode() -> anyhow::Result<()> {
    let server_key = dot_ssh_secret_key(&SecretKey::generate(&mut rand::rng()), false, false).ok();
    let service_key = dot_ssh_secret_key(&SecretKey::generate(&mut rand::rng()), false, true).ok();

    if server_key.is_none() && service_key.is_none() {
        println!("No roost found. Run 'pigeons home --persist' to set one up.");
        bail!("No keys found")
    }

    println!("pigeons v{}", env!("CARGO_PKG_VERSION"));
    println!();

    if let Some(key) = server_key {
        println!("Your roost ID (user keys):");
        println!("  {}", key.public());
        println!();
        println!("  Send a pigeon with: pigeons carry {}", key.public());
        println!();
    }

    if let Some(key) = service_key {
        println!("Your roost ID (service keys):");
        println!("  {}", key.public());
        println!();
        println!("  Send a pigeon with: pigeons carry {}", key.public());
        println!();
    }

    Ok(())
}

pub fn print_roost_info(endpoint_id: impl std::fmt::Display, ssh_port: u16) {
    println!("Roost is open for business.");
    println!();
    println!("  Roost ID: {}", endpoint_id);
    println!();
    println!("  From another machine, send a pigeon:");
    println!("    pigeons carry {}", endpoint_id);
    println!();
    println!("  Delivering to local sshd on port {}", ssh_port);
}

pub async fn install_sevice(ssh_port: u16, relay_url: Vec<String>) -> anyhow::Result<()> {
    if crate::service::install(ServiceParams {
        ssh_port,
        relay_url,
    })
    .await
    .is_err()
    {
        anyhow::bail!("coop installation is only supported on linux, macos, and windows");
    }

    // Give the daemon a moment to start and generate keys
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    match dot_ssh_secret_key(&SecretKey::generate(&mut rand::rng()), false, true) {
        Ok(key) => {
            println!();
            print_roost_info(key.public(), ssh_port);
        }
        Err(_) => {
            println!("Service installed. Run 'pigeons info' to see your roost ID.");
        }
    }

    Ok(())
}

pub async fn uninstall_service() -> anyhow::Result<()> {
    if crate::service::uninstall().await.is_err() {
        anyhow::bail!("coop removal is only supported on linux, macos, or windows");
    }
    Ok(())
}

pub struct HomeArgs {
    pub ssh_port: u16,
    pub persist: bool,
    pub relay_urls: Vec<RelayUrl>,
    pub install_service: bool,
}

pub async fn run_home(args: HomeArgs) -> anyhow::Result<()> {
    match TcpStream::connect(format!("127.0.0.1:{}", args.ssh_port)).await {
        Ok(_) => {}
        Err(_) => {
            eprintln!(
                "Warning: no sshd detected on port {}. Pigeons won't be able to deliver connections.",
                args.ssh_port
            );
            eprintln!("  Make sure sshd is running before sending pigeons to this roost.");
            eprintln!();
        }
    }

    let mut builder = Tunnel::builder()
        .accept_incoming(true)
        .accept_port(args.ssh_port)
        .relay_urls(args.relay_urls);
    if args.persist {
        builder = builder.dot_ssh_integration(true, args.install_service);
    }
    let tunnel = builder.build().await?;

    print_roost_info(tunnel.endpoint().id(), args.ssh_port);
    if args.persist {
        let distro_home = my_home()?.ok_or_else(|| anyhow::anyhow!("home directory not found"))?;
        let ssh_dir = distro_home.join(".ssh");
        println!(
            "  Nest secured with persistent keys in {}",
            ssh_dir.display()
        );
    } else {
        println!(
            "  Warning: using temporary nest. Run with --persist / -p so pigeons can find their way back."
        );
    }
    println!();
    println!("  Awaiting pigeons... (Ctrl+C to close the roost)");

    tokio::signal::ctrl_c().await?;
    Ok(())
}

pub struct CarryArgs {
    pub endpoint_id: EndpointId,
    pub tunnel_name: String,
    pub relay_urls: Vec<RelayUrl>,
    pub bind_port: Option<u16>,
    pub no_ssh_config: bool,
}

pub async fn run_carry(args: CarryArgs) -> anyhow::Result<()> {
    let tunnel_name = args.tunnel_name;
    let tunnel = Tunnel::builder()
        .accept_incoming(false)
        .relay_urls(args.relay_urls.clone())
        .build()
        .await?;

    let bind_addr = format!("127.0.0.1:{}", args.bind_port.unwrap_or(0));
    let listener = TcpListener::bind(&bind_addr).await?;
    let local_port = listener.local_addr()?.port();

    let manage_ssh_config = !args.no_ssh_config;
    if manage_ssh_config {
        ssh::add_tunnel_host(&tunnel_name, local_port)?;
        println!("Trained pigeon route '{tunnel_name}' in ~/.ssh/config");
    }

    println!();
    println!(
        "Pigeon '{}' perched on 127.0.0.1:{}",
        &tunnel_name, local_port
    );
    println!();
    println!("  Fly with: ssh <user>@{tunnel_name}");
    println!();
    println!("  Waiting for messages to carry... (Ctrl+C to recall)");

    let result = accept_loop(&tunnel, &listener, args.endpoint_id).await;

    if manage_ssh_config {
        if let Err(e) = ssh::remove_tunnel_host(&tunnel_name) {
            eprintln!("Warning: couldn't clean up pigeon route: {e}");
        } else {
            println!("Pigeon '{}' returned to the coop.", tunnel_name);
        }
    }

    result
}

async fn accept_loop(
    tunnel: &Tunnel,
    listener: &TcpListener,
    endpoint_id: EndpointId,
) -> anyhow::Result<()> {
    let endpoint = tunnel.endpoint().clone();

    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((tcp_stream, peer_addr)) => {
                        println!("Pigeon departing from {peer_addr}");
                        let endpoint = endpoint.clone();
                        tokio::spawn(async move {
                            if let Err(e) = bridge_connection(tcp_stream, &endpoint, endpoint_id).await {
                                eprintln!("Pigeon lost in transit: {e}");
                            }
                        });
                    }
                    Err(e) => {
                        eprintln!("Failed to accept connection: {e}");
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
                println!("\nRecalling all pigeons...");
                break;
            }
        }
    }

    Ok(())
}

async fn bridge_connection(
    mut tcp_stream: tokio::net::TcpStream,
    endpoint: &iroh::Endpoint,
    endpoint_id: EndpointId,
) -> anyhow::Result<()> {
    let conn = endpoint.connect(endpoint_id, PigeonsProtocol::ALPN).await?;
    let (mut iroh_send, mut iroh_recv) = conn.open_bi().await?;
    let (mut tcp_read, mut tcp_write) = tcp_stream.split();

    let tcp_to_iroh = async { tokio::io::copy(&mut tcp_read, &mut iroh_send).await };
    let iroh_to_tcp = async { tokio::io::copy(&mut iroh_recv, &mut tcp_write).await };

    tokio::select! {
        result = tcp_to_iroh => {
            let _ = result;
        }
        result = iroh_to_tcp => {
            let _ = result;
        }
    }

    Ok(())
}

pub async fn list_mode() -> anyhow::Result<()> {
    let entries = ssh::list_tunnel_hosts()?;

    if entries.is_empty() {
        println!("No pigeons in flight.");
        return Ok(());
    }

    println!("Active pigeon routes:");
    println!();
    for entry in &entries {
        println!("  {:<20} -> 127.0.0.1:{}", entry.name, entry.port);
    }
    println!();

    Ok(())
}

pub async fn remove_mode(tunnel_name: &str) -> anyhow::Result<()> {
    ssh::remove_tunnel_host(tunnel_name)?;
    println!("Pigeon route '{tunnel_name}' removed. That pigeon is free now.");
    Ok(())
}
