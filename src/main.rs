use clap::Parser;
use pigeons::{Cli, Cmd, ServiceCmd, api};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.cmd {
        Cmd::Home(args) => api::home_mode(args, false).await,
        Cmd::Carry(args) => api::carry_mode(args).await,
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
                        extra_relay_url,
                    } => api::service::install(ssh_port, relay_url, extra_relay_url).await,
                    ServiceCmd::Uninstall => api::service::uninstall().await,
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
