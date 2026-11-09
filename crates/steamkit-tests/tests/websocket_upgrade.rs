//! `ReqwestWebSocketFactory` tests: the upgrade goes through the configured
//! `reqwest::Client`, so its proxy, TLS and handlers apply.

use std::time::Duration;

use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use steamkit::networking::steam3::web_socket_context::{
    ReqwestWebSocketFactory, WebSocketFactory, WebSocketSocket,
};
use tokio::net::TcpListener;

#[tokio::test]
async fn the_upgrade_goes_through_the_configured_http_client() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = tokio_tungstenite::accept_async(stream).await.unwrap();

        let message = socket.next().await.unwrap().unwrap();
        socket
            .send(tokio_tungstenite::tungstenite::Message::Binary(
                message.into_data(),
            ))
            .await
            .unwrap();
    });

    let factory = ReqwestWebSocketFactory::new(reqwest::Client::new());
    let socket = factory
        .connect(
            &format!("ws://{address}/cmsocket/"),
            Duration::from_secs(5),
        )
        .await
        .expect("the upgrade should succeed");

    socket
        .send_binary(Bytes::from_static(b"hello"))
        .await
        .expect("the message should be sent");

    let received = socket.receive().await.expect("the echo should arrive");
    assert_eq!(b"hello".as_slice(), received.data.as_ref());

    server.await.unwrap();
}
