//! System proxy discovery for the WebSocket transport, based on the
//! conventional proxy environment variables.
//!
//! Only `http://` proxies are supported. The tunnel is opened with a plain
//! `CONNECT` request whose response is read byte-by-byte, so no TLS bytes are
//! consumed before the caller starts its own handshake. The Windows registry
//! and the macOS `SystemConfiguration` cases are deliberately out of scope.

use std::time::Duration;

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::error::SteamError;

/// The port used when the proxy URL does not carry one.
const DEFAULT_PROXY_PORT: u16 = 8080;

/// The timeout applied to both the proxy dial and the `CONNECT` handshake.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// The largest `CONNECT` response header block we are willing to buffer.
const MAX_HEADER_BYTES: usize = 16 * 1024;

/// The proxy environment variables, in priority order.
const PROXY_ENV_VARS: [&str; 4] = ["https_proxy", "HTTPS_PROXY", "all_proxy", "ALL_PROXY"];

/// The proxy exclusion environment variables, in priority order.
const NO_PROXY_ENV_VARS: [&str; 2] = ["no_proxy", "NO_PROXY"];

/// An HTTP proxy resolved from the environment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProxyConfig {
    /// The proxy host, without IPv6 brackets.
    pub host: String,
    /// The proxy port.
    pub port: u16,
    /// The `Proxy-Authorization` header value, if the URL carried credentials.
    pub authorization: Option<String>,
}

/// Resolves the proxy to use for a connection to `host`:`port`, or `None` when
/// no proxy is configured or the target is excluded by `no_proxy`.
pub(crate) fn proxy_for(host: &str, port: u16) -> Option<ProxyConfig> {
    let url = proxy_url()?;

    if no_proxy_list().is_some_and(|list| no_proxy_matches(&list, host, port)) {
        return None;
    }

    parse_http_proxy(&url)
}

/// Dials the proxy and opens a `CONNECT` tunnel to the target.
///
/// The returned stream is positioned right after the proxy's response headers,
/// ready for the caller to perform its own TLS handshake over it.
pub(crate) async fn connect_through(
    proxy: &ProxyConfig,
    target_host: &str,
    target_port: u16,
) -> Result<tokio::net::TcpStream, SteamError> {
    let dial = tokio::net::TcpStream::connect((proxy.host.as_str(), proxy.port));
    let mut stream = match tokio::time::timeout(CONNECT_TIMEOUT, dial).await {
        Ok(Ok(stream)) => stream,
        Ok(Err(error)) => {
            return Err(SteamError::Connection(format!(
                "proxy dial to {}:{} failed: {error}",
                proxy.host, proxy.port
            )));
        }
        Err(_) => return Err(SteamError::Timeout(CONNECT_TIMEOUT)),
    };

    let handshake = async {
        let request = connect_request(proxy, target_host, target_port);
        stream.write_all(&request).await.map_err(|error| {
            SteamError::Connection(format!("proxy CONNECT write failed: {error}"))
        })?;
        stream
            .flush()
            .await
            .map_err(|error| SteamError::Connection(format!("proxy CONNECT flush failed: {error}")))?;
        read_connect_response(&mut stream).await
    };

    match tokio::time::timeout(CONNECT_TIMEOUT, handshake).await {
        Ok(Ok(())) => Ok(stream),
        Ok(Err(error)) => Err(error),
        Err(_) => Err(SteamError::Timeout(CONNECT_TIMEOUT)),
    }
}

/// Builds the exact bytes of the `CONNECT` request sent to the proxy.
fn connect_request(proxy: &ProxyConfig, target_host: &str, target_port: u16) -> Vec<u8> {
    let mut request = format!(
        "CONNECT {target_host}:{target_port} HTTP/1.1\r\nHost: {target_host}:{target_port}\r\n"
    );

    if let Some(authorization) = &proxy.authorization {
        request.push_str("Proxy-Authorization: ");
        request.push_str(authorization);
        request.push_str("\r\n");
    }

    request.push_str("\r\n");
    request.into_bytes()
}

/// Reads the proxy response headers one byte at a time and validates the status
/// line. Reading a byte at a time guarantees that no payload bytes are stolen
/// from the tunnel.
async fn read_connect_response(stream: &mut tokio::net::TcpStream) -> Result<(), SteamError> {
    let mut response = Vec::with_capacity(256);
    let mut byte = [0u8; 1];

    loop {
        let read = stream
            .read(&mut byte)
            .await
            .map_err(|error| SteamError::Connection(format!("proxy response read failed: {error}")))?;

        if read == 0 {
            return Err(SteamError::Connection(
                "proxy closed the connection during the CONNECT handshake".to_string(),
            ));
        }

        response.push(byte[0]);

        if response.len() > MAX_HEADER_BYTES {
            return Err(SteamError::Connection(format!(
                "proxy CONNECT response headers exceeded {MAX_HEADER_BYTES} bytes"
            )));
        }

        if response.ends_with(b"\r\n\r\n") {
            break;
        }
    }

    validate_status_line(&response)
}

/// Accepts only an `HTTP/1.x 200` status line.
fn validate_status_line(response: &[u8]) -> Result<(), SteamError> {
    let line_end = response
        .windows(2)
        .position(|window| window == b"\r\n")
        .unwrap_or(response.len());
    let status_line = String::from_utf8_lossy(&response[..line_end]);
    let status_line = status_line.trim();

    let mut parts = status_line.split_whitespace();
    let version = parts.next().unwrap_or_default();
    let code = parts.next().unwrap_or_default();

    if version.starts_with("HTTP/1.") && code == "200" {
        Ok(())
    } else {
        Err(SteamError::Connection(format!(
            "proxy CONNECT refused: {status_line}"
        )))
    }
}

/// Retrieves the configured proxy URL from the environment.
fn proxy_url() -> Option<String> {
    proxy_url_from(non_empty_env)
}

fn proxy_url_from<F>(mut lookup: F) -> Option<String>
where
    F: FnMut(&str) -> Option<String>,
{
    PROXY_ENV_VARS
        .iter()
        .find_map(|key| lookup(*key).filter(|value| !value.trim().is_empty()))
}

/// Retrieves the `no_proxy` exclusion list from the environment.
fn no_proxy_list() -> Option<String> {
    NO_PROXY_ENV_VARS
        .iter()
        .find_map(|key| non_empty_env(key))
}

fn non_empty_env(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

/// Whether any entry in the comma-separated `list` excludes `host`:`port`.
fn no_proxy_matches(list: &str, host: &str, port: u16) -> bool {
    let host = host.to_ascii_lowercase();

    list.split(',')
        .any(|entry| no_proxy_entry_matches(entry.trim(), &host, port))
}

fn no_proxy_entry_matches(entry: &str, host: &str, port: u16) -> bool {
    if entry.is_empty() {
        return false;
    }

    if entry == "*" {
        return true;
    }

    let (pattern, entry_port) = split_no_proxy_entry(entry);

    if entry_port.is_some_and(|entry_port| entry_port != port) {
        return false;
    }

    let pattern = pattern
        .strip_prefix("*.")
        .or_else(|| pattern.strip_prefix('.'))
        .unwrap_or(pattern)
        .to_ascii_lowercase();

    if pattern.is_empty() {
        return false;
    }

    host == pattern || host.ends_with(&format!(".{pattern}"))
}

/// Splits an entry into its host part and its optional port, handling
/// `[::1]:8080` style IPv6 entries.
fn split_no_proxy_entry(entry: &str) -> (&str, Option<u16>) {
    if let Some(rest) = entry.strip_prefix('[') {
        if let Some(end) = rest.find(']') {
            let host = &rest[..end];
            let after = &rest[end + 1..];
            let port = after.strip_prefix(':').and_then(|port| port.parse().ok());

            return (host, port);
        }
    }

    if let Some((host, port)) = entry.rsplit_once(':') {
        // A bare IPv6 address also contains ':', so only a single colon can be
        // an actual port separator here.
        if !host.contains(':') {
            if let Ok(port) = port.parse::<u16>() {
                return (host, Some(port));
            }
        }
    }

    (entry, None)
}

/// Parses an `http://` proxy URL. Other schemes, including `https://` and
/// `socks*://`, are rejected rather than guessed at.
fn parse_http_proxy(url: &str) -> Option<ProxyConfig> {
    let url = url.trim();
    let scheme_end = url.find("://")?;
    let scheme = &url[..scheme_end];

    if !scheme.eq_ignore_ascii_case("http") {
        return None;
    }

    let authority = url[scheme_end + 3..]
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default();

    let (userinfo, host_port) = match authority.rsplit_once('@') {
        Some((userinfo, host_port)) => (Some(userinfo), host_port),
        None => (None, authority),
    };

    let (host, port) = split_host_port(host_port)?;

    let authorization = userinfo
        .filter(|userinfo| !userinfo.is_empty())
        .map(|userinfo| format!("Basic {}", BASE64_STANDARD.encode(userinfo)));

    Some(ProxyConfig {
        host,
        port,
        authorization,
    })
}

/// Splits `host:port`, accepting bracketed IPv6 literals and defaulting the
/// port when it is absent.
fn split_host_port(host_port: &str) -> Option<(String, u16)> {
    let (host, port) = if let Some(rest) = host_port.strip_prefix('[') {
        let end = rest.find(']')?;
        let host = &rest[..end];
        let after = &rest[end + 1..];

        let port = match after.strip_prefix(':') {
            Some(port) => port.parse().ok()?,
            None if after.is_empty() => DEFAULT_PROXY_PORT,
            None => return None,
        };

        (host, port)
    } else {
        match host_port.rsplit_once(':') {
            Some((host, port)) if !host.contains(':') => (host, port.parse().ok()?),
            Some(_) => return None,
            None => (host_port, DEFAULT_PROXY_PORT),
        }
    };

    if host.is_empty() {
        return None;
    }

    Some((host.to_string(), port))
}

#[cfg(test)]
mod tests {
    use std::sync::{Mutex, PoisonError};

    use super::*;

    const ENV_KEYS: [&str; 6] = [
        "https_proxy",
        "HTTPS_PROXY",
        "all_proxy",
        "ALL_PROXY",
        "no_proxy",
        "NO_PROXY",
    ];

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_env<F: FnOnce()>(vars: &[(&str, &str)], test: F) {
        let _guard = ENV_LOCK.lock().unwrap_or_else(PoisonError::into_inner);

        for key in ENV_KEYS {
            std::env::remove_var(key);
        }
        for (key, value) in vars {
            std::env::set_var(key, value);
        }

        test();

        for key in ENV_KEYS {
            std::env::remove_var(key);
        }
    }

    #[test]
    fn proxy_url_prefers_lowercase_then_uppercase_and_ignores_empty() {
        let lowercase_wins = proxy_url_from(|key| match key {
            "https_proxy" => Some("http://lower:9090".to_string()),
            "HTTPS_PROXY" => Some("http://upper:8080".to_string()),
            _ => None,
        });
        assert_eq!(lowercase_wins.as_deref(), Some("http://lower:9090"));

        let uppercase_fallback = proxy_url_from(|key| match key {
            "HTTPS_PROXY" => Some("http://upper:8080".to_string()),
            _ => None,
        });
        assert_eq!(uppercase_fallback.as_deref(), Some("http://upper:8080"));

        let all_proxy_fallback = proxy_url_from(|key| match key {
            "all_proxy" => Some("http://all:1080".to_string()),
            _ => None,
        });
        assert_eq!(all_proxy_fallback.as_deref(), Some("http://all:1080"));

        let empty_is_unset = proxy_url_from(|key| match key {
            "https_proxy" => Some("   ".to_string()),
            "HTTPS_PROXY" => Some("http://upper:8080".to_string()),
            _ => None,
        });
        assert_eq!(empty_is_unset.as_deref(), Some("http://upper:8080"));

        assert!(proxy_url_from(|_| None).is_none());
    }

    #[test]
    fn env_proxy_and_no_proxy_are_honoured() {
        with_env(
            &[
                ("https_proxy", "http://env-proxy:8080"),
                ("no_proxy", ".example.com"),
            ],
            || {
                assert!(proxy_for("steam.example.com", 443).is_none());

                let proxy = proxy_for("example.org", 443).expect("proxy should be configured");
                assert_eq!(proxy.host, "env-proxy");
                assert_eq!(proxy.port, 8080);
                assert_eq!(proxy.authorization, None);
            },
        );

        with_env(&[("https_proxy", "")], || {
            assert!(proxy_for("example.org", 443).is_none());
        });
    }

    #[test]
    fn no_proxy_matches_suffixes_wildcards_and_ports() {
        assert!(no_proxy_matches("example.com", "sub.example.com", 443));
        assert!(!no_proxy_matches("example.com", "badexample.com", 443));
        assert!(no_proxy_matches(".internal.local", "host.internal.local", 443));
        assert!(no_proxy_matches("*.internal.local", "host.internal.local", 443));
        assert!(no_proxy_matches("*", "anything.test", 443));
        assert!(no_proxy_matches("example.com:8080", "example.com", 8080));
        assert!(!no_proxy_matches("example.com:8080", "example.com", 443));
        assert!(no_proxy_matches("EXAMPLE.COM", "api.example.com", 443));
        assert!(!no_proxy_matches("", "example.com", 443));
    }

    #[test]
    fn ipv6_proxy_hosts_are_unbracketed() {
        let proxy = parse_http_proxy("http://[::1]:8080").expect("valid proxy");
        assert_eq!(proxy.host, "::1");
        assert_eq!(proxy.port, 8080);

        let proxy = parse_http_proxy("http://[2001:db8::1]").expect("valid proxy");
        assert_eq!(proxy.host, "2001:db8::1");
        assert_eq!(proxy.port, 8080);
    }

    #[test]
    fn default_port_and_rejected_schemes() {
        let proxy = parse_http_proxy("http://proxy.example").expect("valid proxy");
        assert_eq!(proxy.host, "proxy.example");
        assert_eq!(proxy.port, 8080);

        assert!(parse_http_proxy("https://proxy.example:8080").is_none());
        assert!(parse_http_proxy("socks5://proxy.example:1080").is_none());
        assert!(parse_http_proxy("not a url").is_none());
    }

    #[test]
    fn userinfo_becomes_basic_authorization() {
        let proxy = parse_http_proxy("http://user:pass@proxy.example:3128").expect("valid proxy");

        assert_eq!(proxy.host, "proxy.example");
        assert_eq!(proxy.port, 3128);
        assert_eq!(proxy.authorization.as_deref(), Some("Basic dXNlcjpwYXNz"));

        let proxy = parse_http_proxy("http://proxy.example:3128").expect("valid proxy");
        assert_eq!(proxy.authorization, None);
    }

    #[test]
    fn connect_request_wire_format() {
        let proxy = ProxyConfig {
            host: "proxy.example".to_string(),
            port: 8080,
            authorization: None,
        };
        let request = connect_request(&proxy, "steam.example", 443);
        assert_eq!(
            String::from_utf8(request).expect("request is utf-8"),
            "CONNECT steam.example:443 HTTP/1.1\r\nHost: steam.example:443\r\n\r\n"
        );

        let proxy = ProxyConfig {
            host: "proxy.example".to_string(),
            port: 8080,
            authorization: Some("Basic dXNlcjpwYXNz".to_string()),
        };
        let request = connect_request(&proxy, "steam.example", 443);
        assert_eq!(
            String::from_utf8(request).expect("request is utf-8"),
            "CONNECT steam.example:443 HTTP/1.1\r\nHost: steam.example:443\r\nProxy-Authorization: Basic dXNlcjpwYXNz\r\n\r\n"
        );
    }
}
