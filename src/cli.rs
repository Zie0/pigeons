use clap::{ArgAction, Args, Parser, Subcommand};

const RELAY_URL_HELP: &str = "Use only these relay servers, replacing the defaults (repeatable)";
const EXTRA_RELAY_URL_HELP: &str = "Add relay servers alongside the defaults (repeatable)";

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

    #[arg(long, value_name = "URL", help = EXTRA_RELAY_URL_HELP, action = ArgAction::Append)]
    pub extra_relay_url: Vec<String>,
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

    #[arg(long, value_name = "URL", help = EXTRA_RELAY_URL_HELP, action = ArgAction::Append)]
    pub extra_relay_url: Vec<String>,
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

        #[arg(long, value_name = "URL", help = EXTRA_RELAY_URL_HELP, action = ArgAction::Append)]
        extra_relay_url: Vec<String>,
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

    #[arg(long, value_name = "URL", help = EXTRA_RELAY_URL_HELP, action = ArgAction::Append)]
    pub extra_relay_url: Vec<String>,
}
