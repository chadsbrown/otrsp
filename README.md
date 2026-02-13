# otrsp

Async Rust library for the [Open Two Radio Switching Protocol (OTRSP)](https://www.k1xm.org/OTRSP/), used to control SO2R (Single Operator Two Radio) switching devices in contest stations.

## Usage

```rust
use otrsp::{OtrspBuilder, Radio, RxMode, So2rSwitch};

let device = OtrspBuilder::new("/dev/ttyUSB0").build().await?;

println!("Connected to: {}", device.info().name);

// Route TX and audio to Radio 1
device.set_tx(Radio::Radio1).await?;
device.set_rx(Radio::Radio1, RxMode::Mono).await?;

// Switch to Radio 2 with stereo audio
device.set_tx(Radio::Radio2).await?;
device.set_rx(Radio::Radio2, RxMode::Stereo).await?;

// Set band decoder output
device.set_aux(1, 4).await?;

device.close().await?;
```

## Events

Subscribe to state change events via broadcast channel:

```rust
let mut events = device.subscribe();
tokio::spawn(async move {
    while let Ok(event) = events.recv().await {
        println!("{event:?}");
    }
});
```

## Supported Devices

| Device | Manufacturer | Notes |
|--------|-------------|-------|
| RigSelect Pro | KD6X Designs | 4-radio switch, embedded WK3, FTDI dual-port |
| YCCC SO2R Box / SO2R+ | YCCC | Original OTRSP reference hardware |
| SO2RDuino | K1XM / community | Arduino-based, open-source |
| microHAM MK2R+ | microHAM | Also has embedded WinKeyer |
| microHAM Station Master | microHAM | Full station controller |

## Protocol

OTRSP is a simple ASCII serial protocol (9600/8N1) with ~10 commands. It is write-mostly — only `?NAME` and `?AUXn` produce responses. No unsolicited device data.

See the [OTRSP specification (v0.9)](https://k1xm.org/OTRSP/OTRSP_Protocol.pdf) for details.

## Architecture

This crate follows the same async patterns as the companion `winkey` library: async trait (`So2rSwitch`), tokio IO task, broadcast event stream, `MockPort` transport for testing, and builder pattern. All three libraries (`otrsp`, `winkey`, `riglib`) compose in the contest logger via `tokio::select!`.

## License

MIT
