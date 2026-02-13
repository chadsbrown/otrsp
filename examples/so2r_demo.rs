use otrsp::{OtrspBuilder, Radio, RxMode, So2rSwitch};

#[tokio::main]
async fn main() -> otrsp::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let port = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/dev/ttyUSB0".to_string());

    println!("Connecting to OTRSP device on {port}...");

    let device = OtrspBuilder::new(&port).build().await?;

    println!("Connected to: {}", device.info().name);

    // Subscribe to events
    let mut events = device.subscribe();
    tokio::spawn(async move {
        while let Ok(event) = events.recv().await {
            println!("  Event: {event:?}");
        }
    });

    // Focus on Radio 1, mono audio
    println!("\nSetting TX to Radio 1, RX mono...");
    device.set_tx(Radio::Radio1).await?;
    device.set_rx(Radio::Radio1, RxMode::Mono).await?;

    // Switch to Radio 2 with stereo audio
    println!("Setting TX to Radio 2, RX stereo...");
    device.set_tx(Radio::Radio2).await?;
    device.set_rx(Radio::Radio2, RxMode::Stereo).await?;

    // Set AUX band decoder values
    println!("Setting AUX1=4 (20m), AUX2=7 (40m)...");
    device.set_aux(1, 4).await?;
    device.set_aux(2, 7).await?;

    // Back to Radio 1
    println!("Setting TX back to Radio 1, RX reverse stereo...");
    device.set_tx(Radio::Radio1).await?;
    device.set_rx(Radio::Radio1, RxMode::ReverseStereo).await?;

    println!("\nDone. Closing connection.");
    device.close().await?;

    Ok(())
}
