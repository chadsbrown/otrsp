pub mod builder;
pub mod device;
pub mod error;
pub mod event;
pub(crate) mod io;
pub mod protocol;
pub mod switch;
pub mod transport;
pub mod types;

pub use builder::OtrspBuilder;
pub use device::OtrspDevice;
pub use error::{Error, Result};
pub use event::SwitchEvent;
pub use switch::{So2rSwitch, SwitchCapabilities, SwitchInfo};
pub use transport::MockPort;
pub use types::{Radio, RxMode};
