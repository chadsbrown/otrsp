use crate::types::{Radio, RxMode};

/// Events emitted by the OTRSP library when commands succeed.
///
/// These are library-generated state transitions (not device-originated data,
/// since OTRSP devices send no unsolicited messages).
#[derive(Debug, Clone)]
pub enum SwitchEvent {
    /// TX routing changed to the specified radio.
    TxChanged { radio: Radio },
    /// RX audio routing changed.
    RxChanged { radio: Radio, mode: RxMode },
    /// AUX output changed.
    AuxChanged { port: u8, value: u8 },
    /// Connected to the device.
    Connected,
    /// Disconnected from the device.
    Disconnected,
}
