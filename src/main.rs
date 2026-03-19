use clap::{ArgAction, Args, Parser, Subcommand};
use iroh::{EndpointId, RelayUrl};
use pigeons::api;
use std::str::FromStr;

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
    Home(HomeArgs),
    /// Send a pigeon to a remote roost, opening a local tunnel for SSH
    Carry(CarryArgs),
    /// See what pigeons are currently in flight
    List,
    /// Shoo away a pigeon (remove a tunnel entry from ~/.ssh/config)
    Remove(RemoveArgs),
    /// Coop management: install or uninstall pigeons as a system service
    Service {
        #[command(subcommand)]
        op: ServiceCmd,
    },
    /// Show the identity of your local roost
    Info,
    /// Coo the version number
    Version,
    #[command(hide = true)]
    RunService(ServiceArgs),
}

#[derive(Args, Clone, Debug)]
pub struct HomeArgs {
    /// Which port your local sshd is nesting on
    #[arg(long, default_value = "22")]
    pub ssh_port: u16,

    /// Remember this roost's identity across restarts
    #[arg(short, long, default_value_t = false)]
    pub persist: bool,

    #[arg(long, value_name = "URL", help = RELAY_URL_HELP, action = ArgAction::Append)]
    pub relay_url: Vec<String>,
}

impl TryInto<pigeons::api::HomeArgs> for HomeArgs {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<pigeons::api::HomeArgs, Self::Error> {
        let relay_urls = parse_relay_urls(&self.relay_url)?;
        Ok(pigeons::api::HomeArgs {
            ssh_port: self.ssh_port,
            persist: self.persist,
            relay_urls,
            install_service: false,
        })
    }
}

#[derive(Args, Clone, Debug)]
pub struct CarryArgs {
    /// The public key of the remote roost to fly to
    #[arg()]
    pub public_key: String,

    /// A friendly name for this pigeon route (used as SSH Host name). Defaults to pigeon-<first8chars>
    #[arg()]
    pub tunnel_name: Option<String>,

    /// Local port to perch on (default: whichever is available)
    #[arg(long)]
    pub bind_port: Option<u16>,

    /// Don't touch ~/.ssh/config (the pigeon prefers to freelance)
    #[arg(long, default_value_t = false)]
    pub no_ssh_config: bool,

    #[arg(long, value_name = "URL", help = RELAY_URL_HELP, action = ArgAction::Append)]
    pub relay_url: Vec<String>,
}

impl TryInto<pigeons::api::CarryArgs> for CarryArgs {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<pigeons::api::CarryArgs, Self::Error> {
        let relay_urls = parse_relay_urls(&self.relay_url)?;
        let endpoint_id = parse_endpoint_id(&self.public_key)?;
        let key_str = self.public_key;

        let tunnel_name = self.tunnel_name.unwrap_or_else(|| {
            let key_str = format!("{}", endpoint_id);
            format!("pigeon-{}", &key_str[..8.min(key_str.len())])
        });

        Ok(pigeons::api::CarryArgs {
            endpoint_id,
            tunnel_name,
            relay_urls: relay_urls,
            bind_port: self.bind_port,
            no_ssh_config: self.no_ssh_config,
        })
    }
}

#[derive(Args, Clone, Debug)]
pub struct RemoveArgs {
    /// The name of the pigeon route to remove
    #[arg()]
    pub tunnel_name: String,
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
pub struct ServiceArgs {
    #[arg(long, default_value = "22")]
    pub ssh_port: u16,

    #[arg(long, value_name = "URL", help = RELAY_URL_HELP, action = ArgAction::Append)]
    pub relay_url: Vec<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.cmd {
        Cmd::Home(args) => api::run_home(args.try_into()?).await,
        Cmd::Carry(args) => api::run_carry(args.try_into()?).await,
        Cmd::List => api::list_mode().await,
        Cmd::Remove(args) => api::remove_mode(&args.tunnel_name).await,
        Cmd::Service { op } => {
            if !self_runas::is_elevated() {
                self_runas::admin()?;
                return Ok(());
            } else {
                match op {
                    ServiceCmd::Install {
                        ssh_port,
                        relay_url,
                    } => api::install_sevice(ssh_port, relay_url).await,
                    ServiceCmd::Uninstall => api::uninstall_service().await,
                }
            }
        }
        Cmd::Info => api::info_mode().await,
        Cmd::Version => {
            println!("pigeons v{}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        #[cfg(target_os = "windows")]
        Cmd::RunService(args) => {
            pigeons::run_service(args.ssh_port, args.relay_url, args.extra_relay_url).await
        }
        #[cfg(not(target_os = "windows"))]
        Cmd::RunService(_) => {
            anyhow::bail!("service runtime is only available on windows");
        }
    }
}

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
        anyhow::bail!(
            "invalid endpoint id: expected 64 hex characters, got {}",
            key.len()
        );
    };
    EndpointId::from_str(id_str).map_err(|e| anyhow::anyhow!("invalid endpoint id: {e}"))
}
