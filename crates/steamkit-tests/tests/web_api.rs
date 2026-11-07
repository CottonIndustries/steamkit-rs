//! `WebAPI` tests.
//!
//! The configuration's base address points at a minimal HTTP server on the
//! loopback interface, which records each request and answers it.

use std::net::SocketAddr;
use std::str::FromStr;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;

use steamkit::error::SteamError;
use steamkit::steam::steam_client::configuration::i_steam_configuration_builder::ISteamConfigurationBuilder;
use steamkit::steam::steam_client::configuration::steam_configuration::SteamConfiguration;
use steamkit::steam::webapi::steam_configuration_web_api_extensions::get_async_web_api_interface;
use steamkit::steam::webapi::web_api::{web_api_args, AsyncInterface, WebApiValue};
use steamkit_protos::steam::CMsgClientHello;

/// Parses a value from its textual form.
///
/// The type is inferred from the call site.
fn parse<T>(text: &str) -> T
where
    T: FromStr,
    T::Err: std::fmt::Display,
{
    match T::from_str(text) {
        Ok(value) => value,
        Err(error) => panic!("{text} is not a valid value: {error}"),
    }
}

/// Finds the first occurrence of `needle` in `haystack`.
fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }

    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Splits a query string into its key value pairs.
///
/// The names and values are percent decoded, like `HttpUtility.ParseQueryString`
/// does in SteamKit2's tests.
fn parse_query(query: &str) -> Vec<(String, String)> {
    let url = match reqwest::Url::parse(&format!("http://localhost/?{query}")) {
        Ok(url) => url,
        Err(error) => panic!("{query} is not a valid query string: {error}"),
    };

    url.query_pairs()
        .map(|(name, value)| (name.into_owned(), value.into_owned()))
        .collect()
}

/// Looks a value up in the pairs of a parsed query string.
fn lookup<'a>(values: &'a [(String, String)], name: &str) -> Option<&'a str> {
    values
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.as_str())
}

/// A request that the fake Web API server received.
#[derive(Clone, Debug)]
struct CapturedRequest {
    /// The HTTP method of the request.
    method: String,
    /// The path of the request, without the query string.
    path: String,
    /// The query string of the request.
    query: String,
    /// The body of the request.
    body: String,
}

/// A minimal HTTP server that records the requests it receives.
struct FakeWebApi {
    address: SocketAddr,
    requests: mpsc::UnboundedReceiver<CapturedRequest>,
}

impl FakeWebApi {
    /// Starts the server, which answers every request with the given status
    /// line and an empty body.
    async fn start(status_line: &'static str) -> FakeWebApi {
        let listener = match TcpListener::bind("127.0.0.1:0").await {
            Ok(listener) => listener,
            Err(error) => panic!("the fake Web API server must bind, got: {error}"),
        };

        let address = match listener.local_addr() {
            Ok(address) => address,
            Err(error) => panic!("the fake Web API server must have an address, got: {error}"),
        };

        let (sender, requests) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            loop {
                let accepted = listener.accept().await;

                let Ok((mut socket, _)) = accepted else {
                    break;
                };

                let sender = sender.clone();

                tokio::spawn(async move {
                    let request = read_request(&mut socket).await;
                    let _ = sender.send(request);

                    let response =
                        format!("{status_line}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");

                    let _ = socket.write_all(response.as_bytes()).await;
                    let _ = socket.shutdown().await;
                });
            }
        });

        FakeWebApi { address, requests }
    }

    /// The base address the server listens on.
    fn base_address(&self) -> String {
        format!("http://{}/", self.address)
    }

    /// Waits for the next request the server received.
    async fn next_request(&mut self) -> CapturedRequest {
        match self.requests.recv().await {
            Some(request) => request,
            None => panic!("the fake Web API server must record the request"),
        }
    }

    /// Returns the next request the server received, if one arrived already.
    fn try_next_request(&mut self) -> Option<CapturedRequest> {
        self.requests.try_recv().ok()
    }
}

/// Reads a single HTTP request from the given socket.
async fn read_request(socket: &mut TcpStream) -> CapturedRequest {
    let mut buffer: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 1024];

    let head_end = loop {
        if let Some(position) = find_subslice(&buffer, b"\r\n\r\n") {
            break position + 4;
        }

        let count = match socket.read(&mut chunk).await {
            Ok(count) => count,
            Err(_) => break buffer.len(),
        };

        if count == 0 {
            break buffer.len();
        }

        buffer.extend_from_slice(&chunk[..count]);
    };

    let head = String::from_utf8_lossy(&buffer[..head_end]).to_string();

    let mut lines = head.lines();
    let request_line = lines.next().unwrap_or_default().to_string();

    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let target = parts.next().unwrap_or_default().to_string();

    let mut content_length = 0usize;

    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            if name.trim().eq_ignore_ascii_case("content-length") {
                content_length = value.trim().parse().unwrap_or(0);
            }
        }
    }

    let mut body = buffer[head_end..].to_vec();

    while body.len() < content_length {
        let count = match socket.read(&mut chunk).await {
            Ok(count) => count,
            Err(_) => break,
        };

        if count == 0 {
            break;
        }

        body.extend_from_slice(&chunk[..count]);
    }

    let (path, query) = match target.split_once('?') {
        Some((path, query)) => (path.to_string(), query.to_string()),
        None => (target, String::new()),
    };

    CapturedRequest {
        method,
        path,
        query,
        body: String::from_utf8_lossy(&body).to_string(),
    }
}

/// Creates a configuration that talks to the fake Web API server.
fn configuration_for(server: &FakeWebApi) -> SteamConfiguration {
    SteamConfiguration::create(|builder| {
        builder.with_web_api_base_address(parse(&server.base_address()));
    })
}

/// Creates an interface that talks to the fake Web API server.
fn interface_for(server: &FakeWebApi) -> AsyncInterface {
    let config = configuration_for(server);
    get_async_web_api_interface(&config, "IFooService")
}



#[tokio::test]
async fn steam_config_web_api_interface() {
    let mut server = FakeWebApi::start("HTTP/1.1 200 OK").await;

    let config = SteamConfiguration::create(|builder| {
        builder
            .with_web_api_base_address(parse(&server.base_address()))
            .with_web_api_key("hello world".to_string());
    });

    let iface = get_async_web_api_interface(&config, "TestInterface");
    iface
        .call("PerformFooOperation", 1, &[])
        .await
        .expect("the request must succeed");

    let request = server.next_request().await;

    assert!(request.path.starts_with("/TestInterface/PerformFooOperation/v1/"));
    assert!(
        request.query.contains("key=hello%20world"),
        "the configured API key must be sent, got: {}",
        request.query
    );
}

#[tokio::test]
async fn throws_web_api_request_exception_if_request_unsuccessful() {
    let mut server = FakeWebApi::start("HTTP/1.1 503 Service Unavailable").await;
    let iface = interface_for(&server);

    // A failed request comes back as an error.
        match iface.call("PerformFooOperation", 1, &[]).await {
            Ok(_) => panic!("the request must not succeed"),
            Err(SteamError::WebApi { status: 503, .. }) => {}
            Err(error) => panic!("the request must fail with a Web API error, got: {error}"),
        }

    let _ = server.next_request().await;
}

#[tokio::test]
async fn throws_on_incorrect_format_in_args_provided() {
    let mut server = FakeWebApi::start("HTTP/1.1 200 OK").await;
    let iface = interface_for(&server);

    let args = web_api_args([
        ("f", "foo".into()),
        ("b", "bar".into()),
        ("format", "json".into()),
    ]);

    match iface.call("GetFoo", 1, &args).await {
        Ok(_) => panic!("the call must not succeed"),
        Err(SteamError::InvalidArgument(_)) => {}
        Err(error) => panic!("the call must fail with an invalid argument, got: {error}"),
    }
}

#[tokio::test]
async fn doesnt_throw_when_correct_format_in_args_provided() {
    let mut server = FakeWebApi::start("HTTP/1.1 200 OK").await;
    let iface = interface_for(&server);

    let args = web_api_args([
        ("f", "foo".into()),
        ("b", "bar".into()),
        ("format", "vdf".into()),
    ]);

    if let Err(error) = iface.call("GetFoo", 1, &args).await {
        panic!("the call must succeed, got: {error}");
    }

    let _ = server.next_request().await;
}

#[tokio::test]
async fn doesnt_throw_when_key_in_args_provided() {
    let mut server = FakeWebApi::start("HTTP/1.1 200 OK").await;

    let config = SteamConfiguration::create(|builder| {
        builder
            .with_web_api_base_address(parse(&server.base_address()))
            .with_web_api_key("test1".to_string());
    });

    let iface = get_async_web_api_interface(&config, "IFooService");

    let args = web_api_args([
        ("f", "foo".into()),
        ("b", "bar".into()),
        ("key", "test2".into()),
    ]);

    if let Err(error) = iface.call("GetFoo", 1, &args).await {
        panic!("the call must succeed, got: {error}");
    }

    // The call doesn't change the caller's arguments.
    let key = args
        .iter()
        .find(|argument| argument.name == "key")
        .expect("the arguments must still hold the key");

    assert_eq!(WebApiValue::Text("test2".to_string()), key.value);

    let _ = server.next_request().await;
}

#[tokio::test]
async fn doesnt_throw_on_arguments_reuse() {
    let mut server = FakeWebApi::start("HTTP/1.1 200 OK").await;
    let iface = interface_for(&server);

    let args = web_api_args([("f", "foo".into()), ("b", "bar".into())]);

    if let Err(error) = iface.call("GetFoo", 1, &args).await {
        panic!("the first call must succeed, got: {error}");
    }

    if let Err(error) = iface.call("GetFoo", 1, &args).await {
        panic!("the second call must succeed, got: {error}");
    }

    let _ = server.next_request().await;
    let _ = server.next_request().await;
}

#[tokio::test]
async fn uses_args_as_query_string_params() {
    let mut server = FakeWebApi::start("HTTP/1.1 200 OK").await;
    let iface = interface_for(&server);

    let args = web_api_args([("f", "foo".into()), ("b", "bar".into())]);

    if let Err(error) = iface.call("PerformFooOperation", 2, &args).await {
        panic!("the call must succeed, got: {error}");
    }

    let request = server.next_request().await;

    assert_eq!("GET", request.method);
    assert_eq!("/IFooService/PerformFooOperation/v2/", request.path);

    let values = parse_query(&request.query);

    assert_eq!(3usize, values.len());
    assert_eq!(Some("foo"), lookup(&values, "f"));
    assert_eq!(Some("bar"), lookup(&values, "b"));
    assert_eq!(Some("vdf"), lookup(&values, "format"));
}

#[tokio::test]
async fn supports_null_args_dictionary() {
    let mut server = FakeWebApi::start("HTTP/1.1 200 OK").await;
    let iface = interface_for(&server);

    // No arguments at all.
    if let Err(error) = iface.call("PerformFooOperation", 2, &[]).await {
        panic!("the call must succeed, got: {error}");
    }

    let request = server.next_request().await;

    assert_eq!("GET", request.method);
    assert_eq!("/IFooService/PerformFooOperation/v2/", request.path);

    let values = parse_query(&request.query);

    assert_eq!(1usize, values.len());
    assert_eq!(Some("vdf"), lookup(&values, "format"));
}

#[tokio::test]
async fn uses_single_parameter_arguments_dictionary() {
    let mut server = FakeWebApi::start("HTTP/1.1 200 OK").await;
    let iface = interface_for(&server);

    // The HTTP method is passed to the call directly, not as an argument.
    let args = web_api_args([("f", "foo".into()), ("b", "bar".into())]);

    if let Err(error) = iface
        .call_with_method(parse("PUT"), "PerformFooOperation", 2, &args)
        .await
    {
        panic!("the call must succeed, got: {error}");
    }

    let request = server.next_request().await;

    assert_eq!("/IFooService/PerformFooOperation/v2/", request.path);
    assert_eq!("PUT", request.method);

    let form_data = parse_query(&request.body);

    assert_eq!(3usize, form_data.len());
    assert_eq!(Some("foo"), lookup(&form_data, "f"));
    assert_eq!(Some("bar"), lookup(&form_data, "b"));
    assert_eq!(Some("vdf"), lookup(&form_data, "format"));
}

#[tokio::test]
async fn includes_api_key_in_params() {
    let mut server = FakeWebApi::start("HTTP/1.1 200 OK").await;

    let config = SteamConfiguration::create(|builder| {
        builder
            .with_web_api_base_address(parse(&server.base_address()))
            .with_web_api_key("MySecretApiKey".to_string());
    });

    let iface = get_async_web_api_interface(&config, "IFooService");

    let args = web_api_args([("f", "foo".into()), ("b", "bar".into())]);

    if let Err(error) = iface.call("PerformFooOperation", 2, &args).await {
        panic!("the call must succeed, got: {error}");
    }

    let request = server.next_request().await;

    assert_eq!("GET", request.method);
    assert_eq!("/IFooService/PerformFooOperation/v2/", request.path);

    let values = parse_query(&request.query);

    assert_eq!(4usize, values.len());
    assert_eq!(Some("MySecretApiKey"), lookup(&values, "key"));
    assert_eq!(Some("foo"), lookup(&values, "f"));
    assert_eq!(Some("bar"), lookup(&values, "b"));
    assert_eq!(Some("vdf"), lookup(&values, "format"));
}

#[tokio::test]
async fn flattens_list_args_into_indexed_params() {
    let mut server = FakeWebApi::start("HTTP/1.1 200 OK").await;
    let iface = interface_for(&server);

    let args = web_api_args([
        (
            "ids",
            WebApiValue::List(vec!["10".into(), "20".into()]),
        ),
        ("f", "foo".into()),
    ]);

    if let Err(error) = iface.call("PerformFooOperation", 1, &args).await {
        panic!("the call must succeed, got: {error}");
    }

    let request = server.next_request().await;
    let values = parse_query(&request.query);

    assert_eq!(4usize, values.len());
    assert_eq!(Some("10"), lookup(&values, "ids[0]"));
    assert_eq!(Some("20"), lookup(&values, "ids[1]"));
    assert_eq!(Some("foo"), lookup(&values, "f"));
    assert_eq!(Some("vdf"), lookup(&values, "format"));
}

#[tokio::test]
async fn uses_api_key_from_args() {
    let mut server = FakeWebApi::start("HTTP/1.1 200 OK").await;
    let iface = interface_for(&server);

    let args = web_api_args([
        ("key", "MySecretApiKey".into()),
        ("f", "foo".into()),
        ("b", "bar".into()),
    ]);

    if let Err(error) = iface.call("PerformFooOperation", 2, &args).await {
        panic!("the call must succeed, got: {error}");
    }

    let request = server.next_request().await;

    assert_eq!("GET", request.method);
    assert_eq!("/IFooService/PerformFooOperation/v2/", request.path);

    let values = parse_query(&request.query);

    assert_eq!(4usize, values.len());
    assert_eq!(Some("MySecretApiKey"), lookup(&values, "key"));
    assert_eq!(Some("foo"), lookup(&values, "f"));
    assert_eq!(Some("bar"), lookup(&values, "b"));
    assert_eq!(Some("vdf"), lookup(&values, "format"));
}

#[tokio::test]
async fn formats_protobuf_requests() {
    let mut server = FakeWebApi::start("HTTP/1.1 200 OK").await;
    let iface = interface_for(&server);

    let args = web_api_args([("format", "protobuf_raw".into())]);

    // 'protobuf_raw' is the accepted value, so this must not throw.
    if let Err(error) = iface
        .call_protobuf::<CMsgClientHello>(parse("GET"), "PerformFooOperation", 2, &args)
        .await
    {
        panic!("the call must succeed, got: {error}");
    }

    let request = server.next_request().await;
    let values = parse_query(&request.query);

    assert_eq!(Some("protobuf_raw"), lookup(&values, "format"));
}

#[tokio::test]
async fn rejects_vdf_format_when_calling_protobuf() {
    let mut server = FakeWebApi::start("HTTP/1.1 200 OK").await;
    let iface = interface_for(&server);

    let args = web_api_args([("format", "vdf".into())]);

    match iface
        .call_protobuf::<CMsgClientHello>(parse("GET"), "PerformFooOperation", 2, &args)
        .await
    {
        Ok(_) => panic!("the call must not succeed"),
        Err(SteamError::InvalidArgument(_)) => {}
        Err(error) => panic!("the call must fail with an invalid argument, got: {error}"),
    }

    assert!(server.try_next_request().is_none());
}

#[tokio::test]
async fn rejects_protobuf_format_when_calling_vdf() {
    let mut server = FakeWebApi::start("HTTP/1.1 200 OK").await;
    let iface = interface_for(&server);

    let args = web_api_args([("format", "protobuf_raw".into())]);

    match iface.call("PerformFooOperation", 2, &args).await {
        Ok(_) => panic!("the call must not succeed"),
        Err(SteamError::InvalidArgument(_)) => {}
        Err(error) => panic!("the call must fail with an invalid argument, got: {error}"),
    }

    assert!(server.try_next_request().is_none());
}

#[tokio::test]
async fn does_not_produce_empty_query_parameter() {
    for api_key in ["", "MySecretApiKey"] {
        let mut server = FakeWebApi::start("HTTP/1.1 200 OK").await;

        let config = SteamConfiguration::create(|builder| {
            builder
                .with_web_api_base_address(parse(&server.base_address()))
                .with_web_api_key(api_key.to_string());
        });

        let iface = get_async_web_api_interface(&config, "IFooService");

        let args = web_api_args([("format", "vdf".into()), ("f", "foo".into())]);

        if let Err(error) = iface.call("PerformFooOperation", 2, &args).await {
            panic!("the call must succeed, got: {error}");
        }

        let request = server.next_request().await;
        let query = format!("?{}", request.query);

        assert!(!query.contains("?&"), "the query must not hold an empty parameter, got: {query}");
        assert!(!query.contains("&&"), "the query must not hold an empty parameter, got: {query}");

        let values = parse_query(&request.query);

        assert_eq!(Some("foo"), lookup(&values, "f"));
    }
}
