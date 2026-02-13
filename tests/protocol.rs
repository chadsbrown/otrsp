use otrsp::protocol;
use otrsp::{Radio, RxMode};

#[test]
fn encode_tx_radio1() {
    assert_eq!(protocol::encode_tx(Radio::Radio1), b"TX1\r");
}

#[test]
fn encode_tx_radio2() {
    assert_eq!(protocol::encode_tx(Radio::Radio2), b"TX2\r");
}

#[test]
fn encode_rx_all_modes() {
    assert_eq!(protocol::encode_rx(Radio::Radio1, RxMode::Mono), b"RX1\r");
    assert_eq!(protocol::encode_rx(Radio::Radio2, RxMode::Mono), b"RX2\r");
    assert_eq!(
        protocol::encode_rx(Radio::Radio1, RxMode::Stereo),
        b"RX1S\r"
    );
    assert_eq!(
        protocol::encode_rx(Radio::Radio2, RxMode::Stereo),
        b"RX2S\r"
    );
    assert_eq!(
        protocol::encode_rx(Radio::Radio1, RxMode::ReverseStereo),
        b"RX1R\r"
    );
    assert_eq!(
        protocol::encode_rx(Radio::Radio2, RxMode::ReverseStereo),
        b"RX2R\r"
    );
}

#[test]
fn encode_aux_valid() {
    assert_eq!(protocol::encode_aux(1, 4).unwrap(), b"AUX14\r");
    assert_eq!(protocol::encode_aux(2, 255).unwrap(), b"AUX2255\r");
    assert_eq!(protocol::encode_aux(0, 0).unwrap(), b"AUX00\r");
    assert_eq!(protocol::encode_aux(9, 128).unwrap(), b"AUX9128\r");
}

#[test]
fn encode_aux_port_out_of_range() {
    assert!(protocol::encode_aux(10, 0).is_err());
    assert!(protocol::encode_aux(255, 0).is_err());
}

#[test]
fn encode_query_name() {
    assert_eq!(protocol::encode_query_name(), b"?NAME\r");
}

#[test]
fn encode_query_aux_valid() {
    assert_eq!(protocol::encode_query_aux(1).unwrap(), b"?AUX1\r");
    assert_eq!(protocol::encode_query_aux(0).unwrap(), b"?AUX0\r");
    assert_eq!(protocol::encode_query_aux(9).unwrap(), b"?AUX9\r");
}

#[test]
fn encode_query_aux_invalid() {
    assert!(protocol::encode_query_aux(10).is_err());
}

#[test]
fn encode_raw_appends_cr() {
    assert_eq!(protocol::encode_raw("HELLO"), b"HELLO\r");
    assert_eq!(protocol::encode_raw("TX1"), b"TX1\r");
    assert_eq!(protocol::encode_raw(""), b"\r");
}

#[test]
fn parse_name_strips_terminators() {
    assert_eq!(protocol::parse_name_response(b"SO2RDUINO\r"), "SO2RDUINO");
    assert_eq!(
        protocol::parse_name_response(b"RigSelect Pro\r\n"),
        "RigSelect Pro"
    );
    assert_eq!(protocol::parse_name_response(b"YCCC SO2R\n"), "YCCC SO2R");
    assert_eq!(protocol::parse_name_response(b"DeviceName"), "DeviceName");
}

#[test]
fn parse_name_trims_whitespace() {
    assert_eq!(
        protocol::parse_name_response(b"  YCCC SO2R  \r"),
        "YCCC SO2R"
    );
}

#[test]
fn parse_aux_response_valid() {
    assert_eq!(protocol::parse_aux_response(b"AUX14\r").unwrap(), (1, 4));
    assert_eq!(
        protocol::parse_aux_response(b"AUX2255\r\n").unwrap(),
        (2, 255)
    );
    assert_eq!(protocol::parse_aux_response(b"AUX00\r").unwrap(), (0, 0));
    assert_eq!(
        protocol::parse_aux_response(b"AUX9128\r").unwrap(),
        (9, 128)
    );
}

#[test]
fn parse_aux_response_invalid() {
    assert!(protocol::parse_aux_response(b"NOTAUX\r").is_err());
    assert!(protocol::parse_aux_response(b"AUX\r").is_err());
    assert!(protocol::parse_aux_response(b"AUXabc\r").is_err());
}
