use iroh::{endpoint::Connection, protocol::ProtocolHandler};
use tokio::net::TcpStream;

#[derive(Debug)]
pub(crate) struct PigeonsProtocol {
    ssh_port: u16,
}

impl PigeonsProtocol {
    pub const ALPN: &[u8] = b"/pigeons/0";

    /// create a new pigeons "home" that will forward incoming connections from
    /// a bound endpoint to the given local ssh server port
    pub fn new(ssh_port: u16) -> Self {
        Self { ssh_port }
    }
}

impl ProtocolHandler for PigeonsProtocol {
    async fn accept(&self, connection: Connection) -> Result<(), iroh::protocol::AcceptError> {
        let endpoint_id = connection.remote_id();

        match connection.accept_bi().await {
            Ok((mut iroh_send, mut iroh_recv)) => {
                tracing::info!("pigeon arrived from {endpoint_id}");

                match TcpStream::connect(format!("127.0.0.1:{}", self.ssh_port)).await {
                    Ok(mut ssh_stream) => {
                        tracing::info!("delivering to local sshd on port {}", self.ssh_port);

                        let (mut local_read, mut local_write) = ssh_stream.split();

                        let a_to_b =
                            async move { tokio::io::copy(&mut local_read, &mut iroh_send).await };
                        let b_to_a =
                            async move { tokio::io::copy(&mut iroh_recv, &mut local_write).await };

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
