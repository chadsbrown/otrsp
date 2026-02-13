use otrsp::{MockPort, OtrspBuilder, Radio, RxMode, So2rSwitch, SwitchEvent};

#[tokio::test]
async fn build_and_query_name() {
    let mock = MockPort::new();
    // Queue a name response for the builder's ?NAME query
    mock.queue_read(b"SO2RDUINO\r");

    let device = OtrspBuilder::new("/dev/mock")
        .build_with_port(mock.clone())
        .await
        .unwrap();

    assert_eq!(device.info().name, "SO2RDUINO");

    // Verify the ?NAME query was sent
    let written = mock.written_data();
    assert_eq!(&written[..], b"?NAME\r");

    device.close().await.unwrap();
}

#[tokio::test]
async fn build_without_name_query() {
    let mock = MockPort::new();

    let device = OtrspBuilder::new("/dev/mock")
        .query_name(false)
        .build_with_port(mock.clone())
        .await
        .unwrap();

    assert_eq!(device.info().name, "Unknown");

    // Nothing should have been written during build
    assert!(mock.written_data().is_empty());

    device.close().await.unwrap();
}

#[tokio::test]
async fn set_tx_sends_correct_command() {
    let mock = MockPort::new();

    let device = OtrspBuilder::new("/dev/mock")
        .query_name(false)
        .build_with_port(mock.clone())
        .await
        .unwrap();

    device.set_tx(Radio::Radio1).await.unwrap();
    device.set_tx(Radio::Radio2).await.unwrap();

    let written = mock.written_data();
    assert_eq!(&written[..], b"TX1\rTX2\r");

    device.close().await.unwrap();
}

#[tokio::test]
async fn set_rx_sends_correct_commands() {
    let mock = MockPort::new();

    let device = OtrspBuilder::new("/dev/mock")
        .query_name(false)
        .build_with_port(mock.clone())
        .await
        .unwrap();

    device.set_rx(Radio::Radio1, RxMode::Mono).await.unwrap();
    device.set_rx(Radio::Radio2, RxMode::Stereo).await.unwrap();
    device
        .set_rx(Radio::Radio1, RxMode::ReverseStereo)
        .await
        .unwrap();

    let written = mock.written_data();
    assert_eq!(&written[..], b"RX1\rRX2S\rRX1R\r");

    device.close().await.unwrap();
}

#[tokio::test]
async fn set_aux_sends_correct_command() {
    let mock = MockPort::new();

    let device = OtrspBuilder::new("/dev/mock")
        .query_name(false)
        .build_with_port(mock.clone())
        .await
        .unwrap();

    device.set_aux(1, 4).await.unwrap();
    device.set_aux(2, 255).await.unwrap();

    let written = mock.written_data();
    assert_eq!(&written[..], b"AUX14\rAUX2255\r");

    device.close().await.unwrap();
}

#[tokio::test]
async fn device_name_query_via_trait() {
    let mock = MockPort::new();

    let device = OtrspBuilder::new("/dev/mock")
        .query_name(false)
        .build_with_port(mock.clone())
        .await
        .unwrap();

    // Queue a name response for the device_name() query
    mock.queue_read(b"RigSelect Pro\r");

    let name = device.device_name().await.unwrap();
    assert_eq!(name, "RigSelect Pro");

    let written = mock.written_data();
    assert_eq!(&written[..], b"?NAME\r");

    device.close().await.unwrap();
}

#[tokio::test]
async fn query_aux_via_trait() {
    let mock = MockPort::new();

    let device = OtrspBuilder::new("/dev/mock")
        .query_name(false)
        .build_with_port(mock.clone())
        .await
        .unwrap();

    // Queue an AUX response
    mock.queue_read(b"AUX14\r");

    let value = device.query_aux(1).await.unwrap();
    assert_eq!(value, 4);

    let written = mock.written_data();
    assert_eq!(&written[..], b"?AUX1\r");

    device.close().await.unwrap();
}

#[tokio::test]
async fn send_raw_command() {
    let mock = MockPort::new();

    let device = OtrspBuilder::new("/dev/mock")
        .query_name(false)
        .build_with_port(mock.clone())
        .await
        .unwrap();

    device.send_raw("CUSTOM").await.unwrap();

    let written = mock.written_data();
    assert_eq!(&written[..], b"CUSTOM\r");

    device.close().await.unwrap();
}

#[tokio::test]
async fn events_emitted_on_state_changes() {
    let mock = MockPort::new();

    let device = OtrspBuilder::new("/dev/mock")
        .query_name(false)
        .build_with_port(mock.clone())
        .await
        .unwrap();

    let mut rx = device.subscribe();

    device.set_tx(Radio::Radio1).await.unwrap();

    match rx.recv().await.unwrap() {
        SwitchEvent::TxChanged { radio } => assert_eq!(radio, Radio::Radio1),
        other => panic!("expected TxChanged, got {other:?}"),
    }

    device.set_rx(Radio::Radio2, RxMode::Stereo).await.unwrap();

    match rx.recv().await.unwrap() {
        SwitchEvent::RxChanged { radio, mode } => {
            assert_eq!(radio, Radio::Radio2);
            assert_eq!(mode, RxMode::Stereo);
        }
        other => panic!("expected RxChanged, got {other:?}"),
    }

    device.set_aux(1, 42).await.unwrap();

    match rx.recv().await.unwrap() {
        SwitchEvent::AuxChanged { port, value } => {
            assert_eq!(port, 1);
            assert_eq!(value, 42);
        }
        other => panic!("expected AuxChanged, got {other:?}"),
    }

    device.close().await.unwrap();
}

#[tokio::test]
async fn capabilities_defaults() {
    let mock = MockPort::new();

    let device = OtrspBuilder::new("/dev/mock")
        .query_name(false)
        .build_with_port(mock.clone())
        .await
        .unwrap();

    let caps = device.capabilities();
    assert!(caps.stereo);
    assert!(caps.reverse_stereo);
    assert_eq!(caps.aux_ports, 2);

    device.close().await.unwrap();
}
