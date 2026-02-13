//! OtrspBuilder: configure and connect to an OTRSP device.

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

use crate::device::OtrspDevice;
use crate::error::Result;
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
    pub async fn build_with_port<P>(self, port: P) -> Result<OtrspDevice>
    where
        P: AsyncRead + AsyncWrite + Send + Unpin + 'static,
    {
        // Spawn IO task first — single owner of the port from the start.
        let (event_tx, _) = broadcast::channel::<SwitchEvent>(64);
        let _ = event_tx.send(SwitchEvent::Connected);

        let io = spawn_io_task(port, event_tx.clone());

        // Optionally query the device name through the IO task.
        let name = if self.query_name {
            debug!("querying device name");
            match io.command_read(b"?NAME\r".to_vec()).await {
                Ok(response) => {
                    let name = crate::protocol::parse_name_response(response.as_bytes());
                    info!(name = %name, "OTRSP device identified");
                    name
                }
                Err(e) => {
                    warn!("failed to query device name: {e}");
                    "Unknown".to_string()
                }
            }
        } else {
            "Unknown".to_string()
        };

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
