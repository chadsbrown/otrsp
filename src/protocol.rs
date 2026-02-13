//! OTRSP command encoding and response parsing.
//!
//! All functions are pure (no I/O), fully unit-testable.

use crate::error::{Error, Result};
use crate::types::{Radio, RxMode};

/// Encode a TX selection command (`TX1\r` or `TX2\r`).
pub fn encode_tx(radio: Radio) -> Vec<u8> {
    match radio {
        Radio::Radio1 => b"TX1\r".to_vec(),
        Radio::Radio2 => b"TX2\r".to_vec(),
    }
}

/// Encode an RX audio routing command.
///
/// Produces `RX1\r`, `RX2\r`, `RX1S\r`, `RX2S\r`, `RX1R\r`, or `RX2R\r`.
pub fn encode_rx(radio: Radio, mode: RxMode) -> Vec<u8> {
    let num = match radio {
        Radio::Radio1 => '1',
        Radio::Radio2 => '2',
    };
    let suffix = match mode {
        RxMode::Mono => "",
        RxMode::Stereo => "S",
        RxMode::ReverseStereo => "R",
    };
    format!("RX{num}{suffix}\r").into_bytes()
}

/// Encode an AUX output command (`AUXpv\r`).
///
/// `port` must be 0-9, `value` is 0-255 (decimal encoding, variable width).
pub fn encode_aux(port: u8, value: u8) -> Result<Vec<u8>> {
    if port > 9 {
        return Err(Error::InvalidParameter(format!(
            "AUX port must be 0-9, got {port}"
        )));
    }
    Ok(format!("AUX{port}{value}\r").into_bytes())
}

/// Encode a `?NAME` query command.
pub fn encode_query_name() -> Vec<u8> {
    b"?NAME\r".to_vec()
}

/// Encode a `?AUXp` query command.
///
/// `port` must be 0-9.
pub fn encode_query_aux(port: u8) -> Result<Vec<u8>> {
    if port > 9 {
        return Err(Error::InvalidParameter(format!(
            "AUX port must be 0-9, got {port}"
        )));
    }
    Ok(format!("?AUX{port}\r").into_bytes())
}

/// Encode a raw command string with CR terminator appended.
pub fn encode_raw(cmd: &str) -> Vec<u8> {
    format!("{cmd}\r").into_bytes()
}

/// Parse a `?NAME` response, stripping the `NAME` prefix and CR/LF terminators.
///
/// Real OTRSP devices respond with `NAME<devicename>\r` (e.g. `NAMESO2Rduino\r`).
pub fn parse_name_response(bytes: &[u8]) -> String {
    let s = String::from_utf8_lossy(bytes);
    let s = s.trim_end_matches(['\r', '\n']).trim();
    s.strip_prefix("NAME")
        .map(|s| s.trim())
        .unwrap_or(s)
        .to_string()
}

/// Parse a `?AUXpv` response into `(port, value)`.
///
/// Expected format: `AUX<port><value>` possibly followed by CR/LF.
pub fn parse_aux_response(bytes: &[u8]) -> Result<(u8, u8)> {
    let s = String::from_utf8_lossy(bytes);
    let s = s.trim_end_matches(['\r', '\n']).trim();

    let rest = s
        .strip_prefix("AUX")
        .ok_or_else(|| Error::Protocol(format!("expected AUX prefix, got: {s}")))?;

    if rest.is_empty() {
        return Err(Error::Protocol(
            "AUX response missing port and value".into(),
        ));
    }

    let port = rest.as_bytes()[0]
        .checked_sub(b'0')
        .filter(|&p| p <= 9)
        .ok_or_else(|| {
            Error::Protocol(format!(
                "invalid AUX port digit: {}",
                rest.as_bytes()[0] as char
            ))
        })?;

    let value_str = &rest[1..];
    let value: u8 = value_str
        .parse()
        .map_err(|_| Error::Protocol(format!("invalid AUX value: {value_str}")))?;

    Ok((port, value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_tx() {
        assert_eq!(encode_tx(Radio::Radio1), b"TX1\r");
        assert_eq!(encode_tx(Radio::Radio2), b"TX2\r");
    }

    #[test]
    fn test_encode_rx_mono() {
        assert_eq!(encode_rx(Radio::Radio1, RxMode::Mono), b"RX1\r");
        assert_eq!(encode_rx(Radio::Radio2, RxMode::Mono), b"RX2\r");
    }

    #[test]
    fn test_encode_rx_stereo() {
        assert_eq!(encode_rx(Radio::Radio1, RxMode::Stereo), b"RX1S\r");
        assert_eq!(encode_rx(Radio::Radio2, RxMode::Stereo), b"RX2S\r");
    }

    #[test]
    fn test_encode_rx_reverse_stereo() {
        assert_eq!(encode_rx(Radio::Radio1, RxMode::ReverseStereo), b"RX1R\r");
        assert_eq!(encode_rx(Radio::Radio2, RxMode::ReverseStereo), b"RX2R\r");
    }

    #[test]
    fn test_encode_aux() {
        assert_eq!(encode_aux(1, 4).unwrap(), b"AUX14\r");
        assert_eq!(encode_aux(2, 255).unwrap(), b"AUX2255\r");
        assert_eq!(encode_aux(0, 0).unwrap(), b"AUX00\r");
        assert_eq!(encode_aux(9, 128).unwrap(), b"AUX9128\r");
    }

    #[test]
    fn test_encode_aux_invalid_port() {
        assert!(encode_aux(10, 0).is_err());
    }

    #[test]
    fn test_encode_query_name() {
        assert_eq!(encode_query_name(), b"?NAME\r");
    }

    #[test]
    fn test_encode_query_aux() {
        assert_eq!(encode_query_aux(1).unwrap(), b"?AUX1\r");
        assert_eq!(encode_query_aux(0).unwrap(), b"?AUX0\r");
        assert!(encode_query_aux(10).is_err());
    }

    #[test]
    fn test_encode_raw() {
        assert_eq!(encode_raw("HELLO"), b"HELLO\r");
        assert_eq!(encode_raw("TX1"), b"TX1\r");
    }

    #[test]
    fn test_parse_name_response() {
        // Real devices respond with NAME prefix
        assert_eq!(parse_name_response(b"NAMESO2RDUINO\r"), "SO2RDUINO");
        assert_eq!(parse_name_response(b"NAMERigSelect Pro\r\n"), "RigSelect Pro");
        assert_eq!(parse_name_response(b"NAME  YCCC SO2R  \r"), "YCCC SO2R");
        assert_eq!(parse_name_response(b"NAMEDeviceName"), "DeviceName");
        // Graceful handling of responses without NAME prefix
        assert_eq!(parse_name_response(b"SO2RDUINO\r"), "SO2RDUINO");
    }

    #[test]
    fn test_parse_aux_response() {
        assert_eq!(parse_aux_response(b"AUX14\r").unwrap(), (1, 4));
        assert_eq!(parse_aux_response(b"AUX2255\r\n").unwrap(), (2, 255));
        assert_eq!(parse_aux_response(b"AUX00\r").unwrap(), (0, 0));
    }

    #[test]
    fn test_parse_aux_response_invalid() {
        assert!(parse_aux_response(b"NOTAUX\r").is_err());
        assert!(parse_aux_response(b"AUX\r").is_err());
        assert!(parse_aux_response(b"AUXabc\r").is_err());
    }
}
