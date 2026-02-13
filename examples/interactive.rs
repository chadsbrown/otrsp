//! Interactive OTRSP terminal for SO2R switch testing.
//!
//! Commands:
//!
//!   /tx1, /tx2           Set TX to Radio 1 or 2
//!   /rx1, /rx2           Set RX mono to Radio 1 or 2
//!   /rx1s, /rx2s         Set RX stereo
//!   /rx1r, /rx2r         Set RX reverse stereo
//!   /aux <port> <value>  Set AUX output (e.g. /aux 1 4)
//!   /qaux <port>         Query AUX port value
//!   /name                Query device name
//!   /raw <cmd>           Send raw command string
//!   /info                Print device info and capabilities
//!   /help                Print command list
//!   /quit                Close and exit
//!
//! Usage: cargo run --example interactive -- /dev/ttyUSB0

use std::io::Write;

use tokio::io::AsyncBufReadExt;

use otrsp::{OtrspBuilder, Radio, RxMode, So2rSwitch};

#[tokio::main]
async fn main() -> otrsp::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <port>", args[0]);
        std::process::exit(1);
    }

    let port = &args[1];

    eprintln!("Connecting to {port}...");
    let device = OtrspBuilder::new(port).build().await?;

    eprintln!("Connected: {}", device.info().name);
    if let Some(p) = &device.info().port {
        eprintln!("Port: {p}");
    }
    eprintln!();
    eprintln!("Type /help for command list, /quit to exit.");
    eprintln!();

    // Spawn event monitor — prints to stderr so it doesn't mix with prompt
    let mut event_rx = device.subscribe();
    tokio::spawn(async move {
        while let Ok(event) = event_rx.recv().await {
            eprint!("\r  [event: {event:?}]\r\n> ");
            let _ = std::io::stderr().flush();
        }
    });

    let stdin = tokio::io::BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();

    loop {
        eprint!("> ");
        let _ = std::io::stderr().flush();

        let line = match lines.next_line().await {
            Ok(Some(l)) => l,
            Ok(None) => break,
            Err(e) => {
                eprintln!("stdin error: {e}");
                break;
            }
        };

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if !line.starts_with('/') {
            eprintln!("Commands start with /. Type /help for list.");
            continue;
        }

        let parts: Vec<&str> = line.splitn(3, ' ').collect();
        let cmd = parts[0];

        match cmd {
            "/help" | "/h" => {
                eprintln!("Commands:");
                eprintln!("  /tx1, /tx2           Set TX to Radio 1 or 2");
                eprintln!("  /rx1, /rx2           Set RX mono to Radio 1 or 2");
                eprintln!("  /rx1s, /rx2s         Set RX stereo");
                eprintln!("  /rx1r, /rx2r         Set RX reverse stereo");
                eprintln!("  /aux <port> <value>  Set AUX output (e.g. /aux 1 4)");
                eprintln!("  /qaux <port>         Query AUX port value");
                eprintln!("  /name                Query device name");
                eprintln!("  /raw <cmd>           Send raw command string");
                eprintln!("  /info                Print device info and capabilities");
                eprintln!("  /help                Print command list");
                eprintln!("  /quit                Close and exit");
            }
            "/tx1" => match device.set_tx(Radio::Radio1).await {
                Ok(()) => eprintln!("TX -> Radio 1"),
                Err(e) => eprintln!("Error: {e}"),
            },
            "/tx2" => match device.set_tx(Radio::Radio2).await {
                Ok(()) => eprintln!("TX -> Radio 2"),
                Err(e) => eprintln!("Error: {e}"),
            },
            "/rx1" => match device.set_rx(Radio::Radio1, RxMode::Mono).await {
                Ok(()) => eprintln!("RX -> Radio 1 mono"),
                Err(e) => eprintln!("Error: {e}"),
            },
            "/rx2" => match device.set_rx(Radio::Radio2, RxMode::Mono).await {
                Ok(()) => eprintln!("RX -> Radio 2 mono"),
                Err(e) => eprintln!("Error: {e}"),
            },
            "/rx1s" => match device.set_rx(Radio::Radio1, RxMode::Stereo).await {
                Ok(()) => eprintln!("RX -> Radio 1 stereo"),
                Err(e) => eprintln!("Error: {e}"),
            },
            "/rx2s" => match device.set_rx(Radio::Radio2, RxMode::Stereo).await {
                Ok(()) => eprintln!("RX -> Radio 2 stereo"),
                Err(e) => eprintln!("Error: {e}"),
            },
            "/rx1r" => match device.set_rx(Radio::Radio1, RxMode::ReverseStereo).await {
                Ok(()) => eprintln!("RX -> Radio 1 reverse stereo"),
                Err(e) => eprintln!("Error: {e}"),
            },
            "/rx2r" => match device.set_rx(Radio::Radio2, RxMode::ReverseStereo).await {
                Ok(()) => eprintln!("RX -> Radio 2 reverse stereo"),
                Err(e) => eprintln!("Error: {e}"),
            },
            "/aux" => {
                let port_arg = parts.get(1).copied().unwrap_or("");
                let value_arg = parts.get(2).copied().unwrap_or("");
                match (port_arg.parse::<u8>(), value_arg.parse::<u8>()) {
                    (Ok(p), Ok(v)) => match device.set_aux(p, v).await {
                        Ok(()) => eprintln!("AUX{p} = {v}"),
                        Err(e) => eprintln!("Error: {e}"),
                    },
                    _ => eprintln!("Usage: /aux <port> <value> (e.g. /aux 1 4)"),
                }
            }
            "/qaux" => {
                let port_arg = parts.get(1).copied().unwrap_or("");
                match port_arg.parse::<u8>() {
                    Ok(p) => match device.query_aux(p).await {
                        Ok(v) => eprintln!("AUX{p} = {v}"),
                        Err(e) => eprintln!("Error: {e}"),
                    },
                    Err(_) => eprintln!("Usage: /qaux <port> (e.g. /qaux 1)"),
                }
            }
            "/name" => match device.device_name().await {
                Ok(name) => eprintln!("Device name: {name}"),
                Err(e) => eprintln!("Error: {e}"),
            },
            "/raw" => {
                let raw_cmd = line.strip_prefix("/raw").unwrap().trim();
                if raw_cmd.is_empty() {
                    eprintln!("Usage: /raw <command> (e.g. /raw TX1)");
                } else {
                    match device.send_raw(raw_cmd).await {
                        Ok(()) => eprintln!("Sent: {raw_cmd}"),
                        Err(e) => eprintln!("Error: {e}"),
                    }
                }
            }
            "/info" => {
                let info = device.info();
                let caps = device.capabilities();
                eprintln!("Device: {}", info.name);
                if let Some(p) = &info.port {
                    eprintln!("Port: {p}");
                }
                eprintln!("Stereo: {}", caps.stereo);
                eprintln!("Reverse stereo: {}", caps.reverse_stereo);
                eprintln!("AUX ports: {}", caps.aux_ports);
            }
            "/quit" | "/exit" | "/q" => {
                break;
            }
            _ => {
                eprintln!("Unknown command: {cmd} (type /help for list)");
            }
        }
    }

    eprintln!("Closing...");
    device.close().await?;
    eprintln!("Done.");
    Ok(())
}
