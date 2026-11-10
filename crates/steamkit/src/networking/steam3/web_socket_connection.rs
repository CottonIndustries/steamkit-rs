//! Steam's WebSocket transport.
//!
//! The socket itself is behind the [`WebSocketFactory`] and [`WebSocketSocket`]
//! traits (see `web_socket_context`), so tests can swap it out.

use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use tokio::sync::{broadcast, mpsc};

use crate::error::SteamError;

use super::disconnected_event_args::DisconnectedEventArgs;
use super::i_connection::{ConnectionEvent, ConnectionEventSenders, ConnectionEvents, IConnection};
use super::net_msg_event_args::NetMsgEventArgs;
use super::protocol_type::ProtocolTypes;
use super::web_socket_context::{
    construct_uri, construct_uri_for_host, TungsteniteWebSocketFactory, WebSocketContext,
    WebSocketFactory, WebSocketSocket,
};

#[derive(Clone)]
struct CurrentConnection {
    context: Arc<WebSocketContext>,
    socket: Arc<dyn WebSocketSocket>,
}

/// A Steam WebSocket connection.
pub struct WebSocketConnection {
    events: ConnectionEventSenders,
    factory: Arc<dyn WebSocketFactory>,
    host: Option<String>,
    current: Arc<Mutex<Option<CurrentConnection>>>,
    current_end_point: Arc<Mutex<Option<SocketAddr>>>,
}

impl std::fmt::Debug for WebSocketConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebSocketConnection")
            .finish_non_exhaustive()
    }
}

impl WebSocketConnection {
    /// Creates a websocket connection that uses the given factory to open
    /// sockets.
    pub fn new(factory: Arc<dyn WebSocketFactory>) -> Self {
        Self {
            events: ConnectionEventSenders::with_default_capacity(),
            factory,
            host: None,
            current: Arc::new(Mutex::new(None)),
            current_end_point: Arc::new(Mutex::new(None)),
        }
    }

    /// Creates a websocket connection that opens `tokio-tungstenite` sockets.
    pub fn new_tungstenite() -> Self {
        Self::new(Arc::new(TungsteniteWebSocketFactory::new()))
    }

    /// Opens `tokio-tungstenite` sockets to the given host name. The host name
    /// is kept because it's the TLS server name.
    pub fn new_tungstenite_for_host(host: impl Into<String>) -> Self {
        let mut connection = Self::new_tungstenite();
        connection.host = Some(host.into());
        connection
    }

    /// Like [`new_tungstenite_for_host`](Self::new_tungstenite_for_host), and
    /// also sends the given `User-Agent` with the upgrade request.
    pub fn new_tungstenite_for_host_with_user_agent(
        host: impl Into<String>,
        user_agent: impl Into<String>,
    ) -> Self {
        let mut connection = Self::new(Arc::new(TungsteniteWebSocketFactory::with_user_agent(
            user_agent,
        )));
        connection.host = Some(host.into());
        connection
    }

    /// Like [`new`](Self::new), for a connection to the given host name. The
    /// host name is kept because it's the TLS server name.
    pub fn new_for_host(factory: Arc<dyn WebSocketFactory>, host: impl Into<String>) -> Self {
        let mut connection = Self::new(factory);
        connection.host = Some(host.into());
        connection
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

    async fn disconnect_specific(&self, expected: &CurrentConnection, user_initiated: bool) {
        let removed = {
            let mut guard = lock(&self.current);
            let is_current = guard
                .as_ref()
                .is_some_and(|current| Arc::ptr_eq(&current.context, &expected.context));

            if is_current {
                guard.take()
            } else {
                None
            }
        };

        if let Some(current) = removed {
            current.context.cancel();
            current.context.notify_disconnected(user_initiated);
            let _ = current.socket.close().await;
            *lock(&self.current_end_point) = None;
        }
    }
}

#[async_trait]
impl IConnection for WebSocketConnection {
    fn current_end_point(&self) -> Option<SocketAddr> {
        lock(&self.current_end_point).clone()
    }

    fn protocol_types(&self) -> ProtocolTypes {
        ProtocolTypes::WEB_SOCKET
    }

    async fn connect(&self, end_point: SocketAddr, timeout: Duration) -> Result<(), SteamError> {
        let uri = match &self.host {
            Some(host) => construct_uri_for_host(host, end_point.port()),
            None => construct_uri(end_point),
        };

        let socket = match self.factory.connect(&uri, timeout).await {
            Ok(socket) => socket,
            Err(err) => {
                tracing::debug!("WebSocketConnection: Exception connecting websocket: {err}");
                self.events
                    .emit_disconnected(DisconnectedEventArgs::new(false));
                return Err(err);
            }
        };

        let context = Arc::new(WebSocketContext::new_with_uri(
            end_point,
            uri,
            self.events.clone(),
        ));

        let old = {
            let mut current = lock(&self.current);
            current.replace(CurrentConnection {
                context: context.clone(),
                socket: socket.clone(),
            })
        };

        if let Some(old) = old {
            tracing::debug!(
                "WebSocketConnection: Attempted to connect while already connected. Closing old connection..."
            );
            old.context.cancel();
            old.context.notify_disconnected(false);
            let _ = old.socket.close().await;
        }

        *lock(&self.current_end_point) = Some(end_point);

        context.start(socket);

        Ok(())
    }

    async fn disconnect(&self, user_initiated: bool) {
        let current = {
            let mut guard = lock(&self.current);
            guard.take()
        };

        if let Some(current) = current {
            current.context.cancel();
            current.context.notify_disconnected(user_initiated);
            let _ = current.socket.close().await;
        }

        *lock(&self.current_end_point) = None;
    }

    async fn send(&self, data: Bytes) -> Result<(), SteamError> {
        let current = lock(&self.current).clone();

        let Some(current) = current else {
            return Ok(());
        };

        match current.socket.send_binary(data).await {
            Ok(()) => Ok(()),
            Err(err) => {
                tracing::debug!("WebSocketConnection: Exception while sending data: {err}");
                self.disconnect_specific(&current, false).await;
                Err(err)
            }
        }
    }

    fn local_ip(&self) -> Option<IpAddr> {
        // Not `None`: SteamKit2 reports `IPAddress.None` (255.255.255.255) here
        // and the logon carries an obfuscated form of it. With `None` the logon
        // isn't sent at all, and with 0.0.0.0 the CM server doesn't answer it.
        Some(IpAddr::V4(std::net::Ipv4Addr::new(255, 255, 255, 255)))
    }
}

impl ConnectionEvents for WebSocketConnection {
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

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::web_socket_context::WebSocketReceive;
    use super::*;
    use async_trait::async_trait;

    struct NoopFactory;

    #[async_trait]
    impl WebSocketFactory for NoopFactory {
        async fn connect(
            &self,
            _uri: &str,
            _timeout: Duration,
        ) -> Result<Arc<dyn WebSocketSocket>, SteamError> {
            Ok(Arc::new(NoopSocket))
        }
    }

    struct NoopSocket;

    #[async_trait]
    impl WebSocketSocket for NoopSocket {
        async fn send_binary(&self, _data: Bytes) -> Result<(), SteamError> {
            Ok(())
        }

        async fn receive(&self) -> Result<WebSocketReceive, SteamError> {
            Err(SteamError::Connection("unused".to_string()))
        }

        async fn close(&self) -> Result<(), SteamError> {
            Ok(())
        }

        fn is_open(&self) -> bool {
            false
        }
    }

    fn context(connection: &WebSocketConnection, port: u16) -> Arc<WebSocketContext> {
        Arc::new(WebSocketContext::new(
            SocketAddr::from(([127, 0, 0, 1], port)),
            connection.events.clone(),
        ))
    }

    #[tokio::test]
    async fn specific_disconnect_keeps_a_newer_context() {
        let connection = WebSocketConnection::new(Arc::new(NoopFactory));
        let old = context(&connection, 1);
        let new = context(&connection, 2);
        let socket: Arc<dyn WebSocketSocket> = Arc::new(NoopSocket);

        *super::lock(&connection.current) = Some(CurrentConnection {
            context: new,
            socket: socket.clone(),
        });

        connection
            .disconnect_specific(
                &CurrentConnection {
                    context: old,
                    socket,
                },
                false,
            )
            .await;

        assert!(super::lock(&connection.current).is_some());
    }

    #[tokio::test]
    async fn specific_disconnect_removes_the_matching_context() {
        let connection = WebSocketConnection::new(Arc::new(NoopFactory));
        let current = context(&connection, 1);
        let socket: Arc<dyn WebSocketSocket> = Arc::new(NoopSocket);

        *super::lock(&connection.current) = Some(CurrentConnection {
            context: current.clone(),
            socket: socket.clone(),
        });

        connection
            .disconnect_specific(
                &CurrentConnection {
                    context: current,
                    socket,
                },
                false,
            )
            .await;

        assert!(super::lock(&connection.current).is_none());
    }

    #[tokio::test]
    async fn cancelled_context_does_not_emit_connected() {
        let connection = WebSocketConnection::new(Arc::new(NoopFactory));
        let mut connected = connection.events.subscribe_connected();
        let mut disconnected = connection.events.subscribe_disconnected();
        let ctx = context(&connection, 1);

        ctx.cancel();
        ctx.start(Arc::new(NoopSocket));

        tokio::time::timeout(Duration::from_secs(1), disconnected.recv())
            .await
            .expect("the context should report the disconnect")
            .expect("the event channel should stay open");

        assert!(connected.try_recv().is_err());
    }
}
