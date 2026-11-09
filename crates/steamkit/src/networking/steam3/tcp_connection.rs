//! Steam's TCP transport.
//!
//! # Wire format
//!
//! ```text
//! offset  size  field
//! 0       4     payload length (u32, little endian)
//! 4       4     magic 0x31305456 ("VT01", little endian)
//! 8       ..    payload
//! ```

use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio::sync::{broadcast, mpsc};
use tokio_util::sync::CancellationToken;

use crate::error::SteamError;

use super::disconnected_event_args::DisconnectedEventArgs;
use super::i_connection::{ConnectionEvent, ConnectionEventSenders, ConnectionEvents, IConnection};
use super::net_msg_event_args::NetMsgEventArgs;
use super::protocol_type::ProtocolTypes;

/// The magic value that prefixes every Steam TCP frame: `"VT01"`.
const MAGIC: u32 = 0x31305456;

/// The largest frame payload this connection is willing to allocate.
const MAX_PACKET_SIZE: usize = 16 * 1024 * 1024;

/// A Steam TCP connection.
#[derive(Debug)]
pub struct TcpConnection {
    events: ConnectionEventSenders,
    inner: Arc<TcpInner>,
}

#[derive(Debug)]
struct TcpInner {
    writer: tokio::sync::Mutex<Option<OwnedWriteHalf>>,
    current_end_point: Mutex<Option<SocketAddr>>,
    local_ip: Mutex<Option<IpAddr>>,
    cancel: Mutex<Option<CancellationToken>>,
    io_timeout: Mutex<Duration>,
    disconnect_notified: AtomicBool,
}

impl TcpConnection {
    /// Creates a new, unconnected TCP connection.
    pub fn new() -> Self {
        Self {
            events: ConnectionEventSenders::with_default_capacity(),
            inner: Arc::new(TcpInner {
                writer: tokio::sync::Mutex::new(None),
                current_end_point: Mutex::new(None),
                local_ip: Mutex::new(None),
                cancel: Mutex::new(None),
                io_timeout: Mutex::new(Duration::from_secs(5)),
                disconnect_notified: AtomicBool::new(false),
            }),
        }
    }

    /// Subscribes to the `NetMsgReceived` event.
    pub fn subscribe_net_msg_received(&self) -> broadcast::Receiver<NetMsgEventArgs> {
        self.events.subscribe_net_msg_received()
    }

    /// Subscribes to the `Connected` event.
    pub fn subscribe_connected(&self) -> broadcast::Receiver<()> {
        self.events.subscribe_connected()
    }

    /// Subscribes to the `Disconnected` event.
    pub fn subscribe_disconnected(&self) -> broadcast::Receiver<DisconnectedEventArgs> {
        self.events.subscribe_disconnected()
    }
}

impl Default for TcpConnection {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl IConnection for TcpConnection {
    fn current_end_point(&self) -> Option<SocketAddr> {
        lock(&self.inner.current_end_point).clone()
    }

    fn protocol_types(&self) -> ProtocolTypes {
        ProtocolTypes::TCP
    }

    async fn connect(&self, end_point: SocketAddr, timeout: Duration) -> Result<(), SteamError> {
        tracing::debug!("Connecting to {end_point}...");

        let stream = match tokio::time::timeout(timeout, TcpStream::connect(end_point)).await {
            Ok(Ok(stream)) => stream,
            Ok(Err(err)) => {
                return Err(SteamError::Connection(format!(
                    "failed connecting to {end_point}: {err}"
                )))
            }
            Err(_) => return Err(SteamError::Timeout(timeout)),
        };

        let local_ip = stream.local_addr().ok().map(|addr| addr.ip());
        let (read_half, write_half) = stream.into_split();

        *lock(&self.inner.current_end_point) = Some(end_point);
        *lock(&self.inner.local_ip) = local_ip;
        *lock(&self.inner.io_timeout) = timeout;

        let token = CancellationToken::new();
        *lock(&self.inner.cancel) = Some(token.clone());

        {
            let mut writer = self.inner.writer.lock().await;
            *writer = Some(write_half);
        }

        self.inner.disconnect_notified.store(false, Ordering::SeqCst);

        // Raise `Connected` before starting the read loop. The server talks
        // first (`ChannelEncryptRequest` right after the socket opens), and a
        // read loop on another worker could report that message, or a
        // disconnect, before `Connected`.
        tracing::debug!("Connected to {end_point}");
        self.events.emit_connected();

        let inner = self.inner.clone();
        let events = self.events.clone();
        let io_timeout = timeout;
        tokio::spawn(async move {
            tcp_net_loop(read_half, token, end_point, io_timeout, events, inner).await;
        });

        Ok(())
    }

    async fn disconnect(&self, user_initiated: bool) {
        let token = lock(&self.inner.cancel).take();
        if let Some(token) = token {
            token.cancel();
        }

        {
            let mut writer = self.inner.writer.lock().await;
            *writer = None;
        }

        if !self.inner.disconnect_notified.swap(true, Ordering::SeqCst) {
            self.events
                .emit_disconnected(DisconnectedEventArgs::new(user_initiated));
        }
    }

    async fn send(&self, data: Bytes) -> Result<(), SteamError> {
        let mut writer = self.inner.writer.lock().await;

        let Some(writer) = writer.as_mut() else {
            tracing::debug!("TcpConnection: Attempting to send client data when not connected.");
            return Ok(());
        };

        let timeout_duration = *lock(&self.inner.io_timeout);

        let write = async {
            let mut header = [0u8; 8];
            header[..4].copy_from_slice(&(data.len() as u32).to_le_bytes());
            header[4..].copy_from_slice(&MAGIC.to_le_bytes());

            let buffers = [std::io::IoSlice::new(&header), std::io::IoSlice::new(&data)];
            let written = writer.write_vectored(&buffers).await?;

            if written < header.len() {
                writer.write_all(&header[written..]).await?;
                writer.write_all(&data).await?;
            } else if written < header.len() + data.len() {
                writer.write_all(&data[written - header.len()..]).await?;
            }

            writer.flush().await?;

            Ok::<(), SteamError>(())
        };

        match tokio::time::timeout(timeout_duration, write).await {
            Ok(result) => result,
            Err(_) => Err(SteamError::Timeout(timeout_duration)),
        }
    }

    fn local_ip(&self) -> Option<IpAddr> {
        lock(&self.inner.local_ip).clone()
    }
}

impl ConnectionEvents for TcpConnection {
    fn events(&self) -> mpsc::UnboundedReceiver<ConnectionEvent> {
        self.events.subscribe_events()
    }

    fn net_msg_received(&self) -> broadcast::Receiver<NetMsgEventArgs> {
        self.events.subscribe_net_msg_received()
    }

    fn connected(&self) -> broadcast::Receiver<()> {
        self.events.subscribe_connected()
    }

    fn disconnected(&self) -> broadcast::Receiver<DisconnectedEventArgs> {
        self.events.subscribe_disconnected()
    }
}

async fn tcp_net_loop(
    mut reader: OwnedReadHalf,
    token: CancellationToken,
    end_point: SocketAddr,
    io_timeout: Duration,
    events: ConnectionEventSenders,
    inner: Arc<TcpInner>,
) {
    loop {
        let mut header = [0u8; 8];
        if read_exact_timed(&mut reader, &mut header, &token, io_timeout, false)
            .await
            .is_err()
        {
            break;
        }

        let packet_len = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
        let magic = u32::from_le_bytes([header[4], header[5], header[6], header[7]]);

        if magic != MAGIC {
            tracing::debug!("TcpConnection: Got a packet with invalid magic!");
            break;
        }

        if packet_len as usize > MAX_PACKET_SIZE {
            tracing::debug!("TcpConnection: Got a packet with an implausible length!");
            break;
        }

        let mut payload = vec![0u8; packet_len as usize];
        if read_exact_timed(&mut reader, &mut payload, &token, io_timeout, true)
            .await
            .is_err()
        {
            break;
        }

        events.emit_net_msg_received(NetMsgEventArgs {
            data: payload,
            end_point,
        });
    }

    if !inner.disconnect_notified.swap(true, Ordering::SeqCst) {
        events.emit_disconnected(DisconnectedEventArgs::new(false));
    }
}

/// Reads exactly `buffer.len()` bytes. The first read waits indefinitely, like
/// the C# `Socket.Poll` loop; every following read gets `timeout`, like
/// `Socket.ReceiveTimeout`.
async fn read_exact_timed(
    reader: &mut OwnedReadHalf,
    buffer: &mut [u8],
    token: &CancellationToken,
    timeout: Duration,
    timeout_first: bool,
) -> Result<(), ()> {
    let mut filled = 0;

    while filled < buffer.len() {
        let read = reader.read(&mut buffer[filled..]);

        let result = if filled == 0 && !timeout_first {
            tokio::select! {
                biased;
                _ = token.cancelled() => return Err(()),
                result = read => result,
            }
        } else {
            tokio::select! {
                biased;
                _ = token.cancelled() => return Err(()),
                result = tokio::time::timeout(timeout, read) => match result {
                    Ok(result) => result,
                    Err(_) => return Err(()),
                },
            }
        };

        let read = result.map_err(|_| ())?;
        if read == 0 {
            return Err(());
        }

        filled += read;
    }

    Ok(())
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}
