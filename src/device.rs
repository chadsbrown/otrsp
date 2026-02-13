use async_trait::async_trait;
use tokio::sync::broadcast;

use crate::error::Result;
use crate::event::SwitchEvent;
use crate::io::IoHandle;
use crate::protocol;
use crate::switch::{So2rSwitch, SwitchCapabilities, SwitchInfo};
use crate::types::{Radio, RxMode};

/// An OTRSP device connected via serial port.
///
/// Implements [`So2rSwitch`] for SO2R control. Created via [`OtrspBuilder`](crate::OtrspBuilder).
pub struct OtrspDevice {
    pub(crate) io: IoHandle,
    pub(crate) info: SwitchInfo,
    pub(crate) capabilities: SwitchCapabilities,
    pub(crate) event_tx: broadcast::Sender<SwitchEvent>,
}

#[async_trait]
impl So2rSwitch for OtrspDevice {
    fn info(&self) -> &SwitchInfo {
        &self.info
    }

    fn capabilities(&self) -> &SwitchCapabilities {
        &self.capabilities
    }

    async fn set_tx(&self, radio: Radio) -> Result<()> {
        let data = protocol::encode_tx(radio);
        self.io.command(data).await?;
        let _ = self.event_tx.send(SwitchEvent::TxChanged { radio });
        Ok(())
    }

    async fn set_rx(&self, radio: Radio, mode: RxMode) -> Result<()> {
        let data = protocol::encode_rx(radio, mode);
        self.io.command(data).await?;
        let _ = self.event_tx.send(SwitchEvent::RxChanged { radio, mode });
        Ok(())
    }

    async fn set_aux(&self, port: u8, value: u8) -> Result<()> {
        let data = protocol::encode_aux(port, value)?;
        self.io.command(data).await?;
        let _ = self.event_tx.send(SwitchEvent::AuxChanged { port, value });
        Ok(())
    }

    async fn device_name(&self) -> Result<String> {
        let data = protocol::encode_query_name();
        let response = self.io.command_read(data).await?;
        Ok(protocol::parse_name_response(response.as_bytes()))
    }

    async fn query_aux(&self, port: u8) -> Result<u8> {
        let data = protocol::encode_query_aux(port)?;
        let response = self.io.command_read(data).await?;
        let (returned_port, value) = protocol::parse_aux_response(response.as_bytes())?;
        if returned_port != port {
            return Err(crate::error::Error::Protocol(format!(
                "AUX port mismatch: requested port {port}, got port {returned_port}"
            )));
        }
        Ok(value)
    }

    async fn send_raw(&self, command: &str) -> Result<()> {
        let data = protocol::encode_raw(command);
        self.io.command(data).await
    }

    fn subscribe(&self) -> broadcast::Receiver<SwitchEvent> {
        self.event_tx.subscribe()
    }

    async fn close(&self) -> Result<()> {
        self.io.shutdown().await
    }
}

impl OtrspDevice {
    /// Get a reference to the device info.
    pub fn info(&self) -> &SwitchInfo {
        &self.info
    }

    /// Get a reference to the device capabilities.
    pub fn capabilities(&self) -> &SwitchCapabilities {
        &self.capabilities
    }
}
