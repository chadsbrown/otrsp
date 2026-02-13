//! IO task: single tokio task owns the serial port.
//!
//! Single mpsc channel (no priority split — all OTRSP commands are equal).
//! No unsolicited data from devices, so no read arm in the select loop.

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, trace, warn};

use crate::error::{Error, Result};
use crate::event::SwitchEvent;

/// A request sent to the IO task.
#[derive(Debug)]
pub(crate) enum Request {
    /// Write bytes to the serial port (fire-and-forget with ack).
    Write {
        data: Vec<u8>,
        reply: oneshot::Sender<Result<()>>,
    },
    /// Write bytes and read back a line response (for `?NAME`, `?AUX`).
    WriteAndRead {
        data: Vec<u8>,
        reply: oneshot::Sender<Result<String>>,
    },
    /// Shut down the IO task.
    Shutdown { reply: oneshot::Sender<Result<()>> },
}

/// Handle for communicating with the IO task.
pub(crate) struct IoHandle {
    pub tx: mpsc::Sender<Request>,
    pub cancel: CancellationToken,
    pub _task: JoinHandle<()>,
}

impl IoHandle {
    /// Send a write command and wait for acknowledgment.
    pub async fn command(&self, data: Vec<u8>) -> Result<()> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(Request::Write {
                data,
                reply: reply_tx,
            })
            .await
            .map_err(|_| Error::NotConnected)?;

        match tokio::time::timeout(std::time::Duration::from_secs(5), reply_rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(Error::NotConnected),
            Err(_) => Err(Error::Timeout),
        }
    }

    /// Send a command and read back a line response.
    pub async fn command_read(&self, data: Vec<u8>) -> Result<String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(Request::WriteAndRead {
                data,
                reply: reply_tx,
            })
            .await
            .map_err(|_| Error::NotConnected)?;

        match tokio::time::timeout(std::time::Duration::from_secs(5), reply_rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(Error::NotConnected),
            Err(_) => Err(Error::Timeout),
        }
    }

    /// Request graceful shutdown of the IO task.
    pub async fn shutdown(&self) -> Result<()> {
        let (reply_tx, reply_rx) = oneshot::channel();
        if self
            .tx
            .send(Request::Shutdown { reply: reply_tx })
            .await
            .is_err()
        {
            self.cancel.cancel();
            return Ok(());
        }

        match tokio::time::timeout(std::time::Duration::from_secs(2), reply_rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => {
                self.cancel.cancel();
                Ok(())
            }
            Err(_) => {
                self.cancel.cancel();
                Ok(())
            }
        }
    }
}

/// Spawn the IO task that owns the serial port.
pub(crate) fn spawn_io_task<P>(port: P, event_tx: broadcast::Sender<SwitchEvent>) -> IoHandle
where
    P: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    let (tx, rx) = mpsc::channel::<Request>(32);
    let cancel = CancellationToken::new();

    let task = tokio::spawn(io_loop(port, rx, cancel.clone(), event_tx));

    IoHandle {
        tx,
        cancel,
        _task: task,
    }
}

/// The main IO loop.
async fn io_loop<P>(
    mut port: P,
    mut rx: mpsc::Receiver<Request>,
    cancel: CancellationToken,
    event_tx: broadcast::Sender<SwitchEvent>,
) where
    P: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    debug!("IO task started");

    loop {
        tokio::select! {
            biased;

            _ = cancel.cancelled() => {
                debug!("IO task cancelled");
                break;
            }

            req = rx.recv() => {
                match req {
                    Some(Request::Shutdown { reply }) => {
                        debug!("IO task shutdown requested");
                        let _ = reply.send(Ok(()));
                        return;
                    }
                    Some(req) => {
                        handle_request(req, &mut port, &event_tx).await;
                    }
                    None => {
                        debug!("channel closed");
                        break;
                    }
                }
            }
        }
    }

    let _ = event_tx.send(SwitchEvent::Disconnected);
    debug!("IO task exiting");
}

/// Handle a single request.
async fn handle_request<P>(req: Request, port: &mut P, event_tx: &broadcast::Sender<SwitchEvent>)
where
    P: AsyncRead + AsyncWrite + Send + Unpin,
{
    match req {
        Request::Write { data, reply } => {
            trace!("writing {} bytes: {:02X?}", data.len(), data);
            let result = port.write_all(&data).await.map_err(|e| {
                error!("write error: {e}");
                let _ = event_tx.send(SwitchEvent::Disconnected);
                Error::Io(e)
            });
            let _ = reply.send(result);
        }
        Request::WriteAndRead { data, reply } => {
            trace!("write+read {} bytes", data.len());
            if let Err(e) = port.write_all(&data).await {
                error!("write error: {e}");
                let _ = event_tx.send(SwitchEvent::Disconnected);
                let _ = reply.send(Err(Error::Io(e)));
                return;
            }

            match tokio::time::timeout(std::time::Duration::from_secs(1), read_line(port)).await {
                Ok(Ok(line)) => {
                    let _ = reply.send(Ok(line));
                }
                Ok(Err(e)) => {
                    error!("read error: {e}");
                    let _ = reply.send(Err(Error::Io(e)));
                }
                Err(_) => {
                    warn!("read timeout waiting for response");
                    let _ = reply.send(Err(Error::Timeout));
                }
            }
        }
        Request::Shutdown { reply } => {
            let _ = reply.send(Ok(()));
        }
    }
}

/// Read bytes until CR or LF, returning the line as a string (with terminators).
async fn read_line<P>(port: &mut P) -> std::io::Result<String>
where
    P: AsyncRead + Unpin,
{
    let mut buf = Vec::with_capacity(64);
    let mut byte = [0u8; 1];

    loop {
        let n = port.read(&mut byte).await?;
        if n == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "port closed during read",
            ));
        }
        buf.push(byte[0]);
        if byte[0] == b'\r' || byte[0] == b'\n' {
            break;
        }
    }

    Ok(String::from_utf8_lossy(&buf).into_owned())
}
