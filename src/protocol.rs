use iroh::{
    endpoint::Connection,
    protocol::{AcceptError, ProtocolHandler},
};
use tokio::net::TcpStream;

use crate::tunnel::copy_flush;

#[derive(Debug)]
pub(crate) struct PigeonsProtocol {
    ssh_port: u16,
}

impl PigeonsProtocol {
    pub(crate) const ALPN: &[u8] = b"/pigeons/0";

    /// create a new pigeons "home" that will forward incoming connections from
    /// a bound endpoint to the given local ssh server port
    pub(crate) fn new(ssh_port: u16) -> Self {
        Self { ssh_port }
    }
}

impl ProtocolHandler for PigeonsProtocol {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        let endpoint_id = connection.remote_id();

        match connection.accept_bi().await {
            Ok((mut iroh_send, mut iroh_recv)) => {
                tracing::info!("pigeon arrived from {endpoint_id}");

                match TcpStream::connect(format!("127.0.0.1:{}", self.ssh_port)).await {
                    Ok(ssh_stream) => {
                        tracing::info!("delivering to local sshd on port {}", self.ssh_port);
                        ssh_stream.set_nodelay(true).ok();

                        let (mut local_read, mut local_write) = ssh_stream.into_split();

                        let a_to_b = copy_flush(&mut local_read, &mut iroh_send);
                        let b_to_a = copy_flush(&mut iroh_recv, &mut local_write);

                        tokio::select! {
                            result = a_to_b => {
                                let _ = result;
                                tracing::info!("pigeon from {endpoint_id} returned home");
                            },
                            result = b_to_a => {
                                let _ = result;
                                tracing::info!("pigeon from {endpoint_id} returned home");
                            },
                        };
                    }
                    Err(e) => {
                        tracing::error!("pigeon couldn't reach sshd: {e}");
                    }
                }
            }
            Err(e) => {
                tracing::error!("pigeon dropped its message: {e}");
            }
        }

        Ok(())
    }
}
