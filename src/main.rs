use std::str::FromStr;

use clap::{ArgAction, Args, Parser, Subcommand};
use iroh::EndpointId;

const RELAY_URL_HELP: &str = "use this relay server, replacing the defaults (repeatable)";

#[derive(Parser, Debug)]
#[command(
    name = "pigeons",
    about = "carrier pigeons for your SSH connections. no IP addresses, no problem."
)]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(Subcommand, Debug)]
pub enum Cmd {
    /// Set up a roost. Accepts incoming pigeons and delivers them to your local sshd
    Roost(RoostArgs),
    /// Send a pigeon to a remote roost, opening a local tunnel for SSH
    Fly(FlyArgs),
    /// Train a pigeon route (add an SSH config entry for a remote roost)
    Add(AddArgs),
    /// See what pigeon routes are configured
    List,
    /// Forget a pigeon route (remove an SSH config entry)
    Remove(RemoveArgs),
    /// Coop management: install or uninstall pigeons as a system service
    Service {
        #[command(subcommand)]
        op: ServiceCmd,
    },
    /// Show pigeons status
    Status,
}

#[derive(Subcommand, Clone, Debug)]
pub enum ServiceCmd {
    /// Build a permanent coop (install as system service)
    Install {
        #[arg(long, default_value = "22")]
        ssh_port: u16,

        #[arg(long, value_name = "URL", help = RELAY_URL_HELP, action = ArgAction::Append)]
        relay_url: Vec<String>,
    },
    /// Tear down the coop (uninstall system service)
    Uninstall,
}

#[derive(Args, Clone, Debug)]
pub struct RoostArgs {
    /// Which port your local sshd is nesting on
    #[arg(long, default_value = "22")]
    pub ssh_port: u16,

    /// Use a throwaway identity instead of persisting keys
    #[arg(short, long, default_value_t = false)]
    pub ephemeral: bool,

    #[arg(long, value_name = "URL", help = RELAY_URL_HELP, action = ArgAction::Append)]
    pub relay_url: Vec<String>,
}

#[derive(Args, Clone, Debug)]
pub struct FlyArgs {
    /// The public key of the remote roost to fly to
    #[arg()]
    pub public_key: String,

    /// Bridge stdin/stdout instead of binding a local port (for use as SSH ProxyCommand)
    #[arg(long, default_value_t = false)]
    pub stdio: bool,

    #[arg(long, value_name = "URL", help = RELAY_URL_HELP, action = ArgAction::Append)]
    pub relay_url: Vec<String>,
}

#[derive(Args, Clone, Debug)]
pub struct AddArgs {
    /// The endpoint ID of the remote roost
    #[arg(long)]
    pub id: String,

    /// A friendly name for this pigeon route (used as SSH Host name)
    #[arg(long)]
    pub name: Option<String>,
}

#[derive(Args, Clone, Debug)]
pub struct RemoveArgs {
    /// The name of the pigeon route to remove
    #[arg()]
    pub name: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.cmd {
        Cmd::Roost(_args) => {
            let tunnel = pigeons::Tunnel::builder_ephemeral().build().await?;
            let id = tunnel.endpoint().id();
            println!("roost is running! id: {}", id);
            tokio::signal::ctrl_c().await?;
            tunnel.close().await?;
            Ok(())
        }
        Cmd::Fly(args) => {
            let tunnel = pigeons::Tunnel::builder_ephemeral().build().await?;
            let remote_id = EndpointId::from_str(&args.public_key)?;

            if args.stdio {
                tunnel.fly_stdio(remote_id).await?;
            } else {
                let fut = tunnel.fly(remote_id);
                tokio::select! {
                    res = fut => {
                        if let Err(err) = res {
                            eprintln!("error: {err}");
                        };
                    }
                    _ = tokio::signal::ctrl_c() => {
                        println!("shutting down...");
                    }
                };
            }

            tunnel.close().await?;
            Ok(())
        }
        Cmd::Add(args) => {
            let name = args.name.unwrap_or_else(|| {
                let id = &args.id;
                format!("pigeon-{}", &id[..8.min(id.len())])
            });
            pigeons::add_tunnel_host(&name, &args.id)?;
            println!("Pigeon route '{name}' added to ~/.ssh/config");
            println!();
            println!("  Fly with: ssh <user>@{name}");
            Ok(())
        }
        Cmd::List => {
            let entries = pigeons::list_tunnel_hosts()?;
            if entries.is_empty() {
                println!("No pigeon routes configured.");
            } else {
                println!("Pigeon routes:");
                println!();
                for entry in &entries {
                    println!("  {:<20} {}", entry.name, entry.endpoint_id);
                }
            }
            Ok(())
        }
        Cmd::Remove(args) => {
            pigeons::remove_tunnel_host(&args.name)?;
            println!("Pigeon route '{}' removed.", args.name);
            Ok(())
        }
        Cmd::Service { op } => {
            if !self_runas::is_elevated() {
                self_runas::admin()?;
                return Ok(());
            }
            match op {
                ServiceCmd::Install { ssh_port, relay_url } => {
                    pigeons::install_service(pigeons::ServiceParams { ssh_port, relay_url }).await?;
                    println!("Pigeons service installed.");
                    Ok(())
                }
                ServiceCmd::Uninstall => {
                    pigeons::uninstall_service().await?;
                    println!("Pigeons service uninstalled.");
                    Ok(())
                }
            }
        }
        Cmd::Status => {
            let routes = pigeons::list_tunnel_hosts()?;

            println!("Pigeon routes: {}", routes.len());
            match pigeons::service_endpoint_id() {
                Some(id) => {
                    println!("Service:       running");
                    println!();
                    println!("  Roost ID: {id}");
                    println!();
                    println!("  Connect with:");
                    println!("    pigeons fly {id}");
                    println!("    pigeons add --id {id} --name my-roost");
                }
                None => {
                    println!("Service:       not installed");
                }
            }
            Ok(())
        }
    }
}
