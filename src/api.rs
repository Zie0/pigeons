use std::str::FromStr as _;

use anyhow::bail;
use homedir::my_home;
use iroh::{EndpointId, RelayUrl, SecretKey};
use tokio::net::{TcpListener, TcpStream};

use crate::{
    IrohTunnel,
    cli::{CarryArgs, HomeArgs},
    dot_ssh, ssh_config,
};

fn parse_relay_urls(urls: &[String]) -> anyhow::Result<Vec<RelayUrl>> {
    urls.iter()
        .map(|s| RelayUrl::from_str(s).map_err(|e| anyhow::anyhow!("invalid relay URL '{s}': {e}")))
        .collect()
}

fn parse_endpoint_id(key: &str) -> anyhow::Result<EndpointId> {
    let id_str = if key.len() == 64 {
        key
    } else if key.len() > 64 {
        &key[key.len() - 64..]
    } else {
        bail!(
            "invalid endpoint id: expected 64 hex characters, got {}",
            key.len()
        );
    };
    EndpointId::from_str(id_str).map_err(|e| anyhow::anyhow!("invalid endpoint id: {e}"))
}

pub async fn info_mode() -> anyhow::Result<()> {
    let server_key = dot_ssh(&SecretKey::generate(&mut rand::rng()), false, false).ok();
    let service_key = dot_ssh(&SecretKey::generate(&mut rand::rng()), false, true).ok();

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

pub mod service {
    use crate::{ServiceParams, install_service, uninstall_service};

    pub async fn install(
        ssh_port: u16,
        relay_url: Vec<String>,
        extra_relay_url: Vec<String>,
    ) -> anyhow::Result<()> {
        if install_service(ServiceParams {
            ssh_port,
            relay_url,
            extra_relay_url,
        })
        .await
        .is_err()
        {
            anyhow::bail!("coop installation is only supported on linux, macos, and windows");
        }
        Ok(())
    }

    pub async fn uninstall() -> anyhow::Result<()> {
        if uninstall_service().await.is_err() {
            anyhow::bail!("coop removal is only supported on linux, macos, or windows");
        }
        Ok(())
    }
}

pub async fn home_mode(args: HomeArgs, service: bool) -> anyhow::Result<()> {
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

    let mut builder = IrohTunnel::builder()
        .accept_incoming(true)
        .accept_port(args.ssh_port)
        .relay_urls(parse_relay_urls(&args.relay_url)?)
        .extra_relay_urls(parse_relay_urls(&args.extra_relay_url)?);
    if args.persist {
        builder = builder.dot_ssh_integration(true, service);
    }
    let tunnel = builder.build().await?;

    println!("Roost is open for business.");
    println!();
    println!("  Roost ID: {}", tunnel.endpoint_id());
    println!();
    println!("  From another machine, send a pigeon:");
    println!("    pigeons carry {}", tunnel.endpoint_id());
    println!();
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
    println!("  Delivering to local sshd on port {}", args.ssh_port);
    println!("  Awaiting pigeons... (Ctrl+C to close the roost)");

    tokio::signal::ctrl_c().await?;
    Ok(())
}

pub async fn carry_mode(args: CarryArgs) -> anyhow::Result<()> {
    let endpoint_id = parse_endpoint_id(&args.public_key)?;

    let tunnel_name = args.tunnel_name.unwrap_or_else(|| {
        let key_str = format!("{}", endpoint_id);
        format!("pigeon-{}", &key_str[..8.min(key_str.len())])
    });

    let tunnel = IrohTunnel::builder()
        .accept_incoming(false)
        .relay_urls(parse_relay_urls(&args.relay_url)?)
        .extra_relay_urls(parse_relay_urls(&args.extra_relay_url)?)
        .build()
        .await?;

    let bind_addr = format!("127.0.0.1:{}", args.bind_port.unwrap_or(0));
    let listener = TcpListener::bind(&bind_addr).await?;
    let local_port = listener.local_addr()?.port();

    let manage_ssh_config = !args.no_ssh_config;
    if manage_ssh_config {
        ssh_config::add_tunnel_host(&tunnel_name, local_port)?;
        println!("Trained pigeon route '{tunnel_name}' in ~/.ssh/config");
    }

    println!();
    println!(
        "Pigeon '{}' perched on 127.0.0.1:{}",
        tunnel_name, local_port
    );
    println!();
    println!("  Fly with: ssh <user>@{tunnel_name}");
    println!();
    println!("  Waiting for messages to carry... (Ctrl+C to recall)");

    let result = accept_loop(&tunnel, &listener, endpoint_id).await;

    if manage_ssh_config {
        if let Err(e) = ssh_config::remove_tunnel_host(&tunnel_name) {
            eprintln!("Warning: couldn't clean up pigeon route: {e}");
        } else {
            println!("Pigeon '{}' returned to the coop.", tunnel_name);
        }
    }

    result
}

async fn accept_loop(
    tunnel: &IrohTunnel,
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
    let conn = endpoint.connect(endpoint_id, IrohTunnel::ALPN).await?;
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
    let entries = ssh_config::list_tunnel_hosts()?;

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
    ssh_config::remove_tunnel_host(tunnel_name)?;
    println!("Pigeon route '{tunnel_name}' removed. That pigeon is free now.");
    Ok(())
}
