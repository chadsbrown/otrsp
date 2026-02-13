//! Serial port transport and MockPort for testing.

use std::io;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// Open a serial port for OTRSP communication.
///
/// Parameters: 9600 baud, 8N1, no flow control. RTS and DTR set low per spec.
pub fn open_serial(path: &str) -> crate::Result<tokio_serial::SerialStream> {
    let builder = tokio_serial::new(path, 9600)
        .data_bits(tokio_serial::DataBits::Eight)
        .parity(tokio_serial::Parity::None)
        .stop_bits(tokio_serial::StopBits::One)
        .flow_control(tokio_serial::FlowControl::None);

    let port = tokio_serial::SerialStream::open(&builder)
        .map_err(|e| crate::Error::Transport(format!("failed to open {path}: {e}")))?;

    Ok(port)
}

// ---------------------------------------------------------------------------
// MockPort for testing
// ---------------------------------------------------------------------------

struct MockState {
    /// Bytes available for the reader (device → host).
    read_buf: Vec<u8>,
    /// All bytes written by the host (host → device).
    write_log: Vec<u8>,
    /// Whether the port is "closed".
    closed: bool,
    /// Whether only the read side is closed (writes still succeed).
    read_closed: bool,
    /// Waker to notify when new data is queued.
    read_waker: Option<Waker>,
}

/// A mock serial port implementing `AsyncRead + AsyncWrite` for testing.
///
/// Pre-load response bytes with [`queue_read()`](MockPort::queue_read), then
/// inspect what was written with [`written_data()`](MockPort::written_data).
#[derive(Clone)]
pub struct MockPort {
    state: Arc<Mutex<MockState>>,
}

impl MockPort {
    /// Create a new MockPort with no queued data.
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(MockState {
                read_buf: Vec::new(),
                write_log: Vec::new(),
                closed: false,
                read_closed: false,
                read_waker: None,
            })),
        }
    }

    /// Queue bytes that will be returned by reads (simulating device → host).
    /// Wakes any pending readers.
    pub fn queue_read(&self, data: &[u8]) {
        let mut state = self.state.lock().unwrap();
        state.read_buf.extend_from_slice(data);
        if let Some(waker) = state.read_waker.take() {
            waker.wake();
        }
    }

    /// Get all bytes written to the port (host → device).
    pub fn written_data(&self) -> Vec<u8> {
        self.state.lock().unwrap().write_log.clone()
    }

    /// Check if there are pending read bytes.
    pub fn has_pending_reads(&self) -> bool {
        !self.state.lock().unwrap().read_buf.is_empty()
    }

    /// Mark the port as closed (subsequent reads/writes return error).
    pub fn close(&self) {
        let mut state = self.state.lock().unwrap();
        state.closed = true;
        if let Some(waker) = state.read_waker.take() {
            waker.wake();
        }
    }

    /// Close only the read side (writes still succeed).
    ///
    /// This simulates a half-broken connection where the host can still
    /// send data but receives no response — useful for testing the
    /// read-error code path in `WriteAndRead`.
    pub fn close_read(&self) {
        let mut state = self.state.lock().unwrap();
        state.read_closed = true;
        if let Some(waker) = state.read_waker.take() {
            waker.wake();
        }
    }
}

impl Default for MockPort {
    fn default() -> Self {
        Self::new()
    }
}

impl AsyncRead for MockPort {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let mut state = self.state.lock().unwrap();
        if state.closed || state.read_closed {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "mock port closed",
            )));
        }

        if state.read_buf.is_empty() {
            state.read_waker = Some(cx.waker().clone());
            return Poll::Pending;
        }

        let n = buf.remaining().min(state.read_buf.len());
        buf.put_slice(&state.read_buf[..n]);
        state.read_buf.drain(..n);
        Poll::Ready(Ok(()))
    }
}

impl AsyncWrite for MockPort {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let mut state = self.state.lock().unwrap();
        if state.closed {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "mock port closed",
            )));
        }

        state.write_log.extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let state = self.state.lock().unwrap();
        if state.closed {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "mock port closed",
            )));
        }
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let mut state = self.state.lock().unwrap();
        state.closed = true;
        if let Some(waker) = state.read_waker.take() {
            waker.wake();
        }
        Poll::Ready(Ok(()))
    }
}
