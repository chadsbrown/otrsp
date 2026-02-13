use otrsp::{Error, MockPort, OtrspBuilder, Radio, RxMode, So2rSwitch, SwitchEvent};

#[tokio::test]
async fn build_and_query_name() {
    let mock = MockPort::new();
    // Queue a name response for the builder's ?NAME query (real devices send NAME prefix)
    mock.queue_read(b"NAMESO2RDUINO\r");

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

    // Queue a name response for the device_name() query (real devices send NAME prefix)
    mock.queue_read(b"NAMERigSelect Pro\r");

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

#[tokio::test]
async fn query_aux_rejects_mismatched_port() {
    let mock = MockPort::new();

    let device = OtrspBuilder::new("/dev/mock")
        .query_name(false)
        .build_with_port(mock.clone())
        .await
        .unwrap();

    // Request ?AUX1 but queue a response for port 2
    mock.queue_read(b"AUX24\r");

    let result = device.query_aux(1).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        Error::Protocol(msg) => {
            assert!(msg.contains("mismatch"), "expected mismatch message, got: {msg}");
        }
        other => panic!("expected Error::Protocol, got {other:?}"),
    }

    device.close().await.unwrap();
}

#[tokio::test]
async fn close_emits_disconnected_event() {
    let mock = MockPort::new();

    let device = OtrspBuilder::new("/dev/mock")
        .query_name(false)
        .build_with_port(mock.clone())
        .await
        .unwrap();

    let mut rx = device.subscribe();

    device.close().await.unwrap();

    // The IO task should emit Disconnected on graceful shutdown
    let event = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("timed out waiting for Disconnected event")
        .expect("channel closed");
    assert!(
        matches!(event, SwitchEvent::Disconnected),
        "expected Disconnected, got {event:?}"
    );
}

#[tokio::test]
async fn read_error_emits_disconnected_event() {
    let mock = MockPort::new();

    let device = OtrspBuilder::new("/dev/mock")
        .query_name(false)
        .build_with_port(mock.clone())
        .await
        .unwrap();

    let mut rx = device.subscribe();

    // Close only the read side so that write_all succeeds but the
    // subsequent read fails — exercising the read-error branch.
    mock.close_read();

    let _ = device.query_aux(1).await;

    // Should receive Disconnected from the read error path
    let event = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("timed out waiting for Disconnected event")
        .expect("channel closed");
    assert!(
        matches!(event, SwitchEvent::Disconnected),
        "expected Disconnected, got {event:?}"
    );
}

#[tokio::test]
async fn single_disconnected_event_on_failure() {
    let mock = MockPort::new();

    let device = OtrspBuilder::new("/dev/mock")
        .query_name(false)
        .build_with_port(mock.clone())
        .await
        .unwrap();

    let mut rx = device.subscribe();

    // Close mock to force errors
    mock.close();

    // Trigger two commands that will both fail
    let _ = device.set_tx(Radio::Radio1).await;
    let _ = device.set_tx(Radio::Radio2).await;
    device.close().await.unwrap();

    // Collect all Disconnected events (drain with short timeout)
    let mut disconnect_count = 0;
    loop {
        match tokio::time::timeout(std::time::Duration::from_millis(200), rx.recv()).await {
            Ok(Ok(SwitchEvent::Disconnected)) => disconnect_count += 1,
            Ok(Ok(_)) => {} // skip non-disconnect events
            _ => break,
        }
    }

    assert_eq!(
        disconnect_count, 1,
        "expected exactly 1 Disconnected event, got {disconnect_count}"
    );
}

#[tokio::test]
async fn build_name_timeout_does_not_corrupt_next_query() {
    let mock = MockPort::new();

    // Don't queue any data — the ?NAME query will time out via the IO task.
    let device = OtrspBuilder::new("/dev/mock")
        .build_with_port(mock.clone())
        .await
        .unwrap();

    // Name should be "Unknown" since the query timed out
    assert_eq!(device.info().name, "Unknown");

    // Simulate stale NAME bytes that arrived late (after timeout).
    // These are now sitting in the port buffer.
    mock.queue_read(b"NAMESO2RDUINO\r");

    // Queue the real AUX response with a short delay so the drain
    // can distinguish stale data (already buffered) from the legitimate
    // response (arrives after the command is sent).
    let mock2 = mock.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        mock2.queue_read(b"AUX14\r");
    });

    let value = device.query_aux(1).await.unwrap();
    assert_eq!(value, 4, "AUX query should not be corrupted by late NAME response");

    device.close().await.unwrap();
}
