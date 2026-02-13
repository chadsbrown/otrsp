//! OtrspBuilder: configure and connect to an OTRSP device.

use std::time::Duration;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

use crate::device::OtrspDevice;
use crate::error::{Error, Result};
use crate::event::SwitchEvent;
use crate::io::spawn_io_task;
use crate::switch::{SwitchCapabilities, SwitchInfo};
use crate::transport;

/// Builder for creating an OTRSP device connection.
///
/// # Example
///
/// ```no_run
/// # use otrsp::OtrspBuilder;
/// # async fn example() -> otrsp::Result<()> {
/// let device = OtrspBuilder::new("/dev/ttyUSB0")
///     .build()
///     .await?;
/// # Ok(())
/// # }
/// ```
pub struct OtrspBuilder {
    port_path: String,
    query_name: bool,
}

impl OtrspBuilder {
    /// Create a new builder for the given serial port path.
    pub fn new(port: &str) -> Self {
        Self {
            port_path: port.to_string(),
            query_name: true,
        }
    }

    /// Whether to query the device name during build (default: true).
    pub fn query_name(mut self, enabled: bool) -> Self {
        self.query_name = enabled;
        self
    }

    /// Build the OTRSP connection using a real serial port.
    pub async fn build(self) -> Result<OtrspDevice> {
        let port = transport::open_serial(&self.port_path)?;
        self.build_with_port(port).await
    }

    /// Build using a pre-opened port (for testing with MockPort).
    pub async fn build_with_port<P>(self, mut port: P) -> Result<OtrspDevice>
    where
        P: AsyncRead + AsyncWrite + Send + Unpin + 'static,
    {
        // OTRSP has no init handshake. Optionally query the device name.
        let name = if self.query_name {
            debug!("querying device name");
            port.write_all(b"?NAME\r")
                .await
                .map_err(|e| Error::Transport(format!("failed to send ?NAME: {e}")))?;

            match tokio::time::timeout(Duration::from_secs(1), read_line(&mut port)).await {
                Ok(Ok(response)) => {
                    let name = crate::protocol::parse_name_response(response.as_bytes());
                    info!(name = %name, "OTRSP device identified");
                    name
                }
                Ok(Err(e)) => {
                    warn!("failed to read device name: {e}");
                    "Unknown".to_string()
                }
                Err(_) => {
                    warn!("timeout querying device name");
                    "Unknown".to_string()
                }
            }
        } else {
            "Unknown".to_string()
        };

        // Spawn IO task
        let (event_tx, _) = broadcast::channel::<SwitchEvent>(64);
        let _ = event_tx.send(SwitchEvent::Connected);

        let io = spawn_io_task(port, event_tx.clone());

        Ok(OtrspDevice {
            io,
            info: SwitchInfo {
                name,
                port: Some(self.port_path),
            },
            capabilities: SwitchCapabilities {
                stereo: true,
                reverse_stereo: true,
                aux_ports: 2,
            },
            event_tx,
        })
    }
}

/// Read bytes until CR or LF, returning the line as a string.
async fn read_line<P>(port: &mut P) -> std::io::Result<String>
where
    P: AsyncRead + Unpin,
{
    let mut buf = Vec::with_capacity(64);
    let mut byte = [0u8; 1];

    loop {
        let n = port.read(&mut byte).await?;
        if n == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "port closed during read",
            ));
        }
        buf.push(byte[0]);
        if byte[0] == b'\r' || byte[0] == b'\n' {
            break;
        }
    }

    Ok(String::from_utf8_lossy(&buf).into_owned())
}
