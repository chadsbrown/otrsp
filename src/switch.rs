use async_trait::async_trait;
use tokio::sync::broadcast;

use crate::error::Result;
use crate::event::SwitchEvent;
use crate::types::{Radio, RxMode};

/// Information about a connected SO2R switch device.
pub struct SwitchInfo {
    /// Device name (from `?NAME` query, or default).
    pub name: String,
    /// Serial port path, if connected via serial.
    pub port: Option<String>,
}

/// Capabilities of the SO2R switch device.
pub struct SwitchCapabilities {
    /// Whether the device supports stereo RX mode.
    pub stereo: bool,
    /// Whether the device supports reverse stereo RX mode.
    pub reverse_stereo: bool,
    /// Number of AUX ports (typically 2).
    pub aux_ports: u8,
}

/// Backend-agnostic trait for SO2R switch control.
///
/// Implemented by [`OtrspDevice`](crate::OtrspDevice) for serial OTRSP devices.
/// Future backends (microHAM, FlexRadio) can implement this trait as well.
#[async_trait]
pub trait So2rSwitch: Send + Sync {
    /// Get device info.
    fn info(&self) -> &SwitchInfo;

    /// Get device capabilities.
    fn capabilities(&self) -> &SwitchCapabilities;

    /// Select which radio receives transmit focus (key, mic, PTT).
    async fn set_tx(&self, radio: Radio) -> Result<()>;

    /// Set receive audio routing.
    async fn set_rx(&self, radio: Radio, mode: RxMode) -> Result<()>;

    /// Set an auxiliary BCD output value (band decoder).
    async fn set_aux(&self, port: u8, value: u8) -> Result<()>;

    /// Query the device name.
    async fn device_name(&self) -> Result<String>;

    /// Query the current value of an auxiliary port.
    async fn query_aux(&self, port: u8) -> Result<u8>;

    /// Send a raw OTRSP command (CR terminator appended automatically).
    async fn send_raw(&self, command: &str) -> Result<()>;

    /// Subscribe to switch events.
    fn subscribe(&self) -> broadcast::Receiver<SwitchEvent>;

    /// Close the connection.
    async fn close(&self) -> Result<()>;
}
