use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProxyProtocol {
    TcpOnly,
    Socks5,
    HttpConnect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyConfig {
    pub host: String,
    pub port: u16,
    pub protocol: ProxyProtocol,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default)]
    pub target_probe_host: Option<String>,
    #[serde(default)]
    pub target_probe_port: Option<u16>,
}

fn default_timeout_ms() -> u64 {
    4000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyProbeReport {
    pub host: String,
    pub port: u16,
    pub protocol: ProxyProtocol,
    pub reachable: bool,
    pub handshake_ok: bool,
    pub auth_status: String,
    pub target_connect_ok: Option<bool>,
    pub tcp_connect_ms: Option<u64>,
    pub total_latency_ms: u64,
    pub error: Option<String>,
    pub disclaimer: String,
    pub checked_at_utc: String,
}

fn proxy_disclaimer() -> String {
    "Real TCP/SOCKS5/HTTP proxy probe verifies endpoint reachability and proxy handshake only; it does not prove Android game traffic is routed through the proxy and is strictly separate from ADB forward/reverse port mappings."
        .to_string()
}

pub async fn probe_proxy(config: &ProxyConfig) -> ProxyProbeReport {
    let start = Instant::now();
    let timeout_dur = Duration::from_millis(config.timeout_ms.clamp(250, 30_000));
    let addr = format!("{}:{}", config.host.trim(), config.port);

    if config.host.trim().is_empty() || config.port == 0 {
        return ProxyProbeReport {
            host: config.host.clone(),
            port: config.port,
            protocol: config.protocol,
            reachable: false,
            handshake_ok: false,
            auth_status: "invalid_config".to_string(),
            target_connect_ok: None,
            tcp_connect_ms: None,
            total_latency_ms: 0,
            error: Some("PROXY_INVALID_CONFIG: Host and non-zero port are required".to_string()),
            disclaimer: proxy_disclaimer(),
            checked_at_utc: chrono::Utc::now().to_rfc3339(),
        };
    }

    let tcp_res = tokio::time::timeout(timeout_dur, TcpStream::connect(&addr)).await;
    let mut stream = match tcp_res {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => {
            return ProxyProbeReport {
                host: config.host.clone(),
                port: config.port,
                protocol: config.protocol,
                reachable: false,
                handshake_ok: false,
                auth_status: "unreachable".to_string(),
                target_connect_ok: None,
                tcp_connect_ms: None,
                total_latency_ms: start.elapsed().as_millis() as u64,
                error: Some(format!("PROXY_TCP_CONNECT_FAILED: {}", e)),
                disclaimer: proxy_disclaimer(),
                checked_at_utc: chrono::Utc::now().to_rfc3339(),
            };
        }
        Err(_) => {
            return ProxyProbeReport {
                host: config.host.clone(),
                port: config.port,
                protocol: config.protocol,
                reachable: false,
                handshake_ok: false,
                auth_status: "timeout".to_string(),
                target_connect_ok: None,
                tcp_connect_ms: None,
                total_latency_ms: start.elapsed().as_millis() as u64,
                error: Some(format!(
                    "PROXY_TIMEOUT: TCP connect to {} timed out after {}ms",
                    addr,
                    timeout_dur.as_millis()
                )),
                disclaimer: proxy_disclaimer(),
                checked_at_utc: chrono::Utc::now().to_rfc3339(),
            };
        }
    };

    let tcp_ms = start.elapsed().as_millis() as u64;

    match config.protocol {
        ProxyProtocol::TcpOnly => ProxyProbeReport {
            host: config.host.clone(),
            port: config.port,
            protocol: ProxyProtocol::TcpOnly,
            reachable: true,
            handshake_ok: true,
            auth_status: "none".to_string(),
            target_connect_ok: None,
            tcp_connect_ms: Some(tcp_ms),
            total_latency_ms: tcp_ms,
            error: None,
            disclaimer: proxy_disclaimer(),
            checked_at_utc: chrono::Utc::now().to_rfc3339(),
        },
        ProxyProtocol::Socks5 => {
            let res =
                tokio::time::timeout(timeout_dur, run_socks5_handshake(&mut stream, config)).await;
            match res {
                Ok(Ok((auth_status, target_ok))) => ProxyProbeReport {
                    host: config.host.clone(),
                    port: config.port,
                    protocol: ProxyProtocol::Socks5,
                    reachable: true,
                    handshake_ok: true,
                    auth_status,
                    target_connect_ok: target_ok,
                    tcp_connect_ms: Some(tcp_ms),
                    total_latency_ms: start.elapsed().as_millis() as u64,
                    error: None,
                    disclaimer: proxy_disclaimer(),
                    checked_at_utc: chrono::Utc::now().to_rfc3339(),
                },
                Ok(Err((auth_status, msg))) => ProxyProbeReport {
                    host: config.host.clone(),
                    port: config.port,
                    protocol: ProxyProtocol::Socks5,
                    reachable: true,
                    handshake_ok: false,
                    auth_status,
                    target_connect_ok: None,
                    tcp_connect_ms: Some(tcp_ms),
                    total_latency_ms: start.elapsed().as_millis() as u64,
                    error: Some(msg),
                    disclaimer: proxy_disclaimer(),
                    checked_at_utc: chrono::Utc::now().to_rfc3339(),
                },
                Err(_) => ProxyProbeReport {
                    host: config.host.clone(),
                    port: config.port,
                    protocol: ProxyProtocol::Socks5,
                    reachable: true,
                    handshake_ok: false,
                    auth_status: "timeout".to_string(),
                    target_connect_ok: None,
                    tcp_connect_ms: Some(tcp_ms),
                    total_latency_ms: start.elapsed().as_millis() as u64,
                    error: Some("PROXY_TIMEOUT: SOCKS5 handshake timed out".to_string()),
                    disclaimer: proxy_disclaimer(),
                    checked_at_utc: chrono::Utc::now().to_rfc3339(),
                },
            }
        }
        ProxyProtocol::HttpConnect => {
            let res =
                tokio::time::timeout(timeout_dur, run_http_connect_probe(&mut stream, config))
                    .await;
            match res {
                Ok(Ok((auth_status, target_ok))) => ProxyProbeReport {
                    host: config.host.clone(),
                    port: config.port,
                    protocol: ProxyProtocol::HttpConnect,
                    reachable: true,
                    handshake_ok: true,
                    auth_status,
                    target_connect_ok: Some(target_ok),
                    tcp_connect_ms: Some(tcp_ms),
                    total_latency_ms: start.elapsed().as_millis() as u64,
                    error: None,
                    disclaimer: proxy_disclaimer(),
                    checked_at_utc: chrono::Utc::now().to_rfc3339(),
                },
                Ok(Err((auth_status, msg))) => ProxyProbeReport {
                    host: config.host.clone(),
                    port: config.port,
                    protocol: ProxyProtocol::HttpConnect,
                    reachable: true,
                    handshake_ok: false,
                    auth_status,
                    target_connect_ok: Some(false),
                    tcp_connect_ms: Some(tcp_ms),
                    total_latency_ms: start.elapsed().as_millis() as u64,
                    error: Some(msg),
                    disclaimer: proxy_disclaimer(),
                    checked_at_utc: chrono::Utc::now().to_rfc3339(),
                },
                Err(_) => ProxyProbeReport {
                    host: config.host.clone(),
                    port: config.port,
                    protocol: ProxyProtocol::HttpConnect,
                    reachable: true,
                    handshake_ok: false,
                    auth_status: "timeout".to_string(),
                    target_connect_ok: None,
                    tcp_connect_ms: Some(tcp_ms),
                    total_latency_ms: start.elapsed().as_millis() as u64,
                    error: Some("PROXY_TIMEOUT: HTTP CONNECT handshake timed out".to_string()),
                    disclaimer: proxy_disclaimer(),
                    checked_at_utc: chrono::Utc::now().to_rfc3339(),
                },
            }
        }
    }
}

async fn run_socks5_handshake(
    stream: &mut TcpStream,
    config: &ProxyConfig,
) -> std::result::Result<(String, Option<bool>), (String, String)> {
    let has_creds = config
        .username
        .as_deref()
        .map(|u| !u.is_empty())
        .unwrap_or(false);

    let greeting: Vec<u8> = if has_creds {
        vec![0x05, 0x02, 0x00, 0x02] // NoAuth or User/Pass
    } else {
        vec![0x05, 0x01, 0x00] // NoAuth
    };
    stream.write_all(&greeting).await.map_err(|e| {
        (
            "io_error".to_string(),
            format!("SOCKS5 greeting write failed: {}", e),
        )
    })?;

    let mut choice = [0u8; 2];
    stream.read_exact(&mut choice).await.map_err(|e| {
        (
            "io_error".to_string(),
            format!("SOCKS5 greeting read failed: {}", e),
        )
    })?;

    if choice[0] != 0x05 {
        return Err((
            "protocol_error".to_string(),
            format!(
                "SOCKS5_PROTOCOL_ERROR: Unexpected SOCKS version 0x{:02x}",
                choice[0]
            ),
        ));
    }

    let auth_status = match choice[1] {
        0x00 => "none".to_string(),
        0x02 => {
            let user = config.username.as_deref().unwrap_or("");
            let pass = config.password.as_deref().unwrap_or("");
            if user.is_empty() || user.len() > 255 || pass.len() > 255 {
                return Err((
                    "auth_required".to_string(),
                    "PROXY_AUTH_REQUIRED: SOCKS5 server requested RFC 1929 credentials".to_string(),
                ));
            }
            let mut pkt = Vec::with_capacity(3 + user.len() + pass.len());
            pkt.push(0x01); // subnegotiation version
            pkt.push(user.len() as u8);
            pkt.extend_from_slice(user.as_bytes());
            pkt.push(pass.len() as u8);
            pkt.extend_from_slice(pass.as_bytes());
            stream.write_all(&pkt).await.map_err(|e| {
                (
                    "io_error".to_string(),
                    format!("SOCKS5 auth write failed: {}", e),
                )
            })?;

            let mut auth_rep = [0u8; 2];
            stream.read_exact(&mut auth_rep).await.map_err(|e| {
                (
                    "io_error".to_string(),
                    format!("SOCKS5 auth read failed: {}", e),
                )
            })?;
            if auth_rep[1] != 0x00 {
                return Err((
                    "auth_failed".to_string(),
                    format!(
                        "PROXY_AUTH_FAILED: SOCKS5 rejected credentials (status=0x{:02x})",
                        auth_rep[1]
                    ),
                ));
            }
            "authenticated".to_string()
        }
        0xff => {
            return Err((
                "auth_required".to_string(),
                "PROXY_AUTH_METHODS_REJECTED: No acceptable SOCKS5 authentication method"
                    .to_string(),
            ));
        }
        other => {
            return Err((
                "unsupported_auth".to_string(),
                format!("SOCKS5_UNSUPPORTED_METHOD: 0x{:02x}", other),
            ));
        }
    };

    if let (Some(target_host), Some(target_port)) = (
        config
            .target_probe_host
            .as_deref()
            .filter(|h| !h.trim().is_empty()),
        config.target_probe_port,
    ) {
        let host_bytes = target_host.trim().as_bytes();
        if host_bytes.len() > 255 {
            return Err((
                auth_status,
                "Target probe host exceeds 255 bytes".to_string(),
            ));
        }
        let mut req = Vec::with_capacity(7 + host_bytes.len());
        req.extend_from_slice(&[0x05, 0x01, 0x00, 0x03, host_bytes.len() as u8]);
        req.extend_from_slice(host_bytes);
        req.extend_from_slice(&target_port.to_be_bytes());
        stream.write_all(&req).await.map_err(|e| {
            (
                auth_status.clone(),
                format!("SOCKS5 CONNECT write failed: {}", e),
            )
        })?;
        let mut hdr = [0u8; 4];
        stream.read_exact(&mut hdr).await.map_err(|e| {
            (
                auth_status.clone(),
                format!("SOCKS5 CONNECT reply failed: {}", e),
            )
        })?;
        if hdr[1] != 0x00 {
            return Err((
                auth_status,
                format!("SOCKS5_CONNECT_REJECTED: reply code 0x{:02x}", hdr[1]),
            ));
        }
        return Ok((auth_status, Some(true)));
    }

    Ok((auth_status, None))
}

async fn run_http_connect_probe(
    stream: &mut TcpStream,
    config: &ProxyConfig,
) -> std::result::Result<(String, bool), (String, String)> {
    let target_host = config
        .target_probe_host
        .as_deref()
        .filter(|h| !h.trim().is_empty())
        .unwrap_or("example.com");
    let target_port = config.target_probe_port.unwrap_or(443);
    let authority = format!("{}:{}", target_host.trim(), target_port);

    let mut req = format!(
        "CONNECT {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: FarmerPro-ProxyProbe/1.0\r\n",
        authority, authority
    );
    let has_creds = config
        .username
        .as_deref()
        .map(|u| !u.is_empty())
        .unwrap_or(false);
    if has_creds {
        let user = config.username.as_deref().unwrap_or("");
        let pass = config.password.as_deref().unwrap_or("");
        let encoded = B64.encode(format!("{}:{}", user, pass));
        req.push_str(&format!("Proxy-Authorization: Basic {}\r\n", encoded));
    }
    req.push_str("\r\n");

    stream.write_all(req.as_bytes()).await.map_err(|e| {
        (
            "io_error".to_string(),
            format!("HTTP CONNECT write failed: {}", e),
        )
    })?;

    let mut buf = [0u8; 1024];
    let n = stream.read(&mut buf).await.map_err(|e| {
        (
            "io_error".to_string(),
            format!("HTTP CONNECT read failed: {}", e),
        )
    })?;
    let resp = String::from_utf8_lossy(&buf[..n]);
    let status_line = resp.lines().next().unwrap_or("").trim().to_string();

    if status_line.contains(" 200 ") || status_line.ends_with(" 200") {
        let auth = if has_creds { "authenticated" } else { "none" };
        Ok((auth.to_string(), true))
    } else if status_line.contains(" 407 ") {
        Err((
            if has_creds {
                "auth_failed"
            } else {
                "auth_required"
            }
            .to_string(),
            format!("PROXY_AUTH_407: {}", status_line),
        ))
    } else {
        Err((
            "rejected".to_string(),
            format!("HTTP_CONNECT_FAILED: '{}'", status_line),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn test_proxy_tcp_probe_success_and_refused() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let _ = listener.accept().await;
        });

        let ok_rep = probe_proxy(&ProxyConfig {
            host: "127.0.0.1".to_string(),
            port,
            protocol: ProxyProtocol::TcpOnly,
            username: None,
            password: None,
            timeout_ms: 1500,
            target_probe_host: None,
            target_probe_port: None,
        })
        .await;
        assert!(ok_rep.reachable);
        assert!(ok_rep.handshake_ok);

        // Closed port must fail closed
        let closed_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let closed_port = closed_listener.local_addr().unwrap().port();
        drop(closed_listener);

        let fail_rep = probe_proxy(&ProxyConfig {
            host: "127.0.0.1".to_string(),
            port: closed_port,
            protocol: ProxyProtocol::TcpOnly,
            username: None,
            password: None,
            timeout_ms: 4000,
            target_probe_host: None,
            target_probe_port: None,
        })
        .await;
        assert!(!fail_rep.reachable);
        let err = fail_rep.error.unwrap();
        assert!(
            err.contains("PROXY_TCP_CONNECT_FAILED") || err.contains("PROXY_TIMEOUT"),
            "Unexpected error: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_proxy_socks5_auth_success_and_wrong_password() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        tokio::spawn(async move {
            for _ in 0..2 {
                if let Ok((mut sock, _)) = listener.accept().await {
                    tokio::spawn(async move {
                        let mut greet = [0u8; 4];
                        let n = sock.read(&mut greet).await.unwrap_or(0);
                        if n < 3 {
                            return;
                        }
                        // Require username/password 0x02
                        let _ = sock.write_all(&[0x05, 0x02]).await;
                        let mut hdr = [0u8; 2];
                        if sock.read_exact(&mut hdr).await.is_err() {
                            return;
                        }
                        let ulen = hdr[1] as usize;
                        let mut ubuf = vec![0u8; ulen];
                        if sock.read_exact(&mut ubuf).await.is_err() {
                            return;
                        }
                        let mut plen = [0u8; 1];
                        if sock.read_exact(&mut plen).await.is_err() {
                            return;
                        }
                        let mut pbuf = vec![0u8; plen[0] as usize];
                        if sock.read_exact(&mut pbuf).await.is_err() {
                            return;
                        }
                        if ubuf == b"fp_user" && pbuf == b"fp_secret" {
                            let _ = sock.write_all(&[0x01, 0x00]).await;
                            let mut conn_hdr = [0u8; 4];
                            if sock.read_exact(&mut conn_hdr).await.is_ok() {
                                if conn_hdr[3] == 0x03 {
                                    let mut dlen = [0u8; 1];
                                    if sock.read_exact(&mut dlen).await.is_ok() {
                                        let mut rest = vec![0u8; dlen[0] as usize + 2];
                                        let _ = sock.read_exact(&mut rest).await;
                                    }
                                }
                                let _ = sock
                                    .write_all(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
                                    .await;
                                let _ = sock.shutdown().await;
                            }
                        } else {
                            let _ = sock.write_all(&[0x01, 0x01]).await;
                            let _ = sock.shutdown().await;
                        }
                    });
                }
            }
        });

        let ok_rep = probe_proxy(&ProxyConfig {
            host: "127.0.0.1".to_string(),
            port,
            protocol: ProxyProtocol::Socks5,
            username: Some("fp_user".to_string()),
            password: Some("fp_secret".to_string()),
            timeout_ms: 2000,
            target_probe_host: Some("api.example.local".to_string()),
            target_probe_port: Some(443),
        })
        .await;
        assert!(ok_rep.reachable && ok_rep.handshake_ok);
        assert_eq!(ok_rep.auth_status, "authenticated");
        assert_eq!(ok_rep.target_connect_ok, Some(true));

        let bad_rep = probe_proxy(&ProxyConfig {
            host: "127.0.0.1".to_string(),
            port,
            protocol: ProxyProtocol::Socks5,
            username: Some("fp_user".to_string()),
            password: Some("wrong_pw".to_string()),
            timeout_ms: 2000,
            target_probe_host: None,
            target_probe_port: None,
        })
        .await;
        assert!(bad_rep.reachable);
        assert!(!bad_rep.handshake_ok);
        assert_eq!(bad_rep.auth_status, "auth_failed");
        assert!(bad_rep.error.unwrap().contains("PROXY_AUTH_FAILED"));
    }

    #[tokio::test]
    async fn test_proxy_http_connect_200_and_407() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        tokio::spawn(async move {
            for _ in 0..2 {
                if let Ok((mut sock, _)) = listener.accept().await {
                    let mut buf = [0u8; 1024];
                    let n = sock.read(&mut buf).await.unwrap_or(0);
                    let req = String::from_utf8_lossy(&buf[..n]);
                    let expected_b64 = B64.encode(b"alice:wonderland");
                    if req.contains(&format!("Proxy-Authorization: Basic {}", expected_b64)) {
                        let _ = sock
                            .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
                            .await;
                    } else {
                        let _ = sock
                            .write_all(b"HTTP/1.1 407 Proxy Authentication Required\r\n\r\n")
                            .await;
                    }
                }
            }
        });

        let ok_rep = probe_proxy(&ProxyConfig {
            host: "127.0.0.1".to_string(),
            port,
            protocol: ProxyProtocol::HttpConnect,
            username: Some("alice".to_string()),
            password: Some("wonderland".to_string()),
            timeout_ms: 2000,
            target_probe_host: Some("game.server.local".to_string()),
            target_probe_port: Some(8443),
        })
        .await;
        assert!(ok_rep.handshake_ok);
        assert_eq!(ok_rep.auth_status, "authenticated");
        assert_eq!(ok_rep.target_connect_ok, Some(true));

        let unauth_rep = probe_proxy(&ProxyConfig {
            host: "127.0.0.1".to_string(),
            port,
            protocol: ProxyProtocol::HttpConnect,
            username: None,
            password: None,
            timeout_ms: 2000,
            target_probe_host: Some("game.server.local".to_string()),
            target_probe_port: Some(8443),
        })
        .await;
        assert!(!unauth_rep.handshake_ok);
        assert_eq!(unauth_rep.auth_status, "auth_required");
        assert!(unauth_rep.error.unwrap().contains("407"));
    }
}
