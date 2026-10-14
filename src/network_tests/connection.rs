use super::route::ProxyRoute;
use std::{
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::TcpStream,
};

trait IoStream: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> IoStream for T {}
type Stream = Box<dyn IoStream>;

pub(super) fn tls_config() -> Arc<rustls::ClientConfig> {
    static TLS: OnceLock<Arc<rustls::ClientConfig>> = OnceLock::new();
    TLS.get_or_init(|| {
        let roots =
            rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        Arc::new(
            rustls::ClientConfig::builder_with_provider(Arc::new(
                rustls::crypto::ring::default_provider(),
            ))
            .with_safe_default_protocol_versions()
            .expect("supported TLS versions")
            .with_root_certificates(roots)
            .with_no_client_auth(),
        )
    })
    .clone()
}

async fn connect(host: &str, port: u16, proxy: bool) -> Result<Stream, String> {
    let addresses: Vec<_> = tokio::net::lookup_host((host, port))
        .await
        .map_err(|_| {
            if proxy {
                "代理域名解析失败"
            } else {
                "域名解析失败"
            }
        })?
        .collect();
    if addresses.is_empty() {
        return Err("域名解析未返回可用地址".into());
    }
    let stream = tokio::time::timeout(
        Duration::from_secs(5),
        TcpStream::connect(addresses.as_slice()),
    )
    .await
    .map_err(|_| "连接超时")?
    .map_err(|_| {
        if proxy {
            "无法连接系统代理"
        } else {
            "无法建立 TCP 连接"
        }
    })?;
    let _ = stream.set_nodelay(true);
    Ok(Box::new(stream))
}

async fn secure(
    stream: Stream,
    host: &str,
    config: Arc<rustls::ClientConfig>,
) -> Result<Stream, String> {
    let name =
        rustls::pki_types::ServerName::try_from(host.to_owned()).map_err(|_| "TLS 主机名无效")?;
    let stream = tokio_rustls::TlsConnector::from(config)
        .connect(name, stream)
        .await
        .map_err(|_| "TLS 握手失败，请检查证书或网络")?;
    Ok(Box::new(stream))
}

// Read exactly through the terminator: do not consume tunnel data or a response body.
async fn response_status(stream: &mut Stream) -> Result<u16, String> {
    let mut headers = Vec::new();
    while !headers.ends_with(b"\r\n\r\n") {
        if headers.len() >= 8192 {
            return Err("HTTP 响应头过大".into());
        }
        headers.push(
            stream
                .read_u8()
                .await
                .map_err(|_| "未收到完整 HTTP 响应头")?,
        );
    }
    let line = headers
        .split(|byte| *byte == b'\n')
        .next()
        .ok_or("HTTP 响应格式异常")?;
    let line = std::str::from_utf8(line).map_err(|_| "HTTP 响应格式异常")?;
    let mut parts = line.split_whitespace();
    if !matches!(parts.next(), Some("HTTP/1.0" | "HTTP/1.1")) {
        return Err("HTTP 响应格式异常".into());
    }
    let status = parts
        .next()
        .and_then(|value| value.parse::<u16>().ok())
        .filter(|status| (100..600).contains(status))
        .ok_or("HTTP 响应状态无效")?;
    Ok(status)
}

fn authorization(route: Option<&ProxyRoute>) -> String {
    route
        .and_then(|route| route.authorization.as_deref())
        .map_or_else(String::new, |value| {
            format!("Proxy-Authorization: {value}\r\n")
        })
}

pub(super) async fn measure(
    url: &reqwest::Url,
    route: Option<&ProxyRoute>,
    config: Arc<rustls::ClientConfig>,
) -> Result<u64, String> {
    let host = url
        .host_str()
        .ok_or("网址缺少主机名")?
        .trim_matches(['[', ']']);
    let port = url.port_or_known_default().ok_or("网址端口无效")?;
    let authority = if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    };
    let started = Instant::now();
    let mut stream = if let Some(route) = route {
        let stream = connect(&route.host, route.port, true).await?;
        if route.secure {
            secure(stream, &route.host, config.clone()).await?
        } else {
            stream
        }
    } else {
        connect(host, port, false).await?
    };
    if url.scheme() == "https" {
        if route.is_some() {
            let request = format!(
                "CONNECT {authority} HTTP/1.1\r\nHost: {authority}\r\n{}\r\n",
                authorization(route)
            );
            stream
                .write_all(request.as_bytes())
                .await
                .map_err(|_| "无法发送代理连接请求")?;
            let status = response_status(&mut stream).await?;
            if status != 200 {
                return Err(format!("代理 CONNECT 返回 HTTP {status}"));
            }
        }
        // TLS completion is the endpoint. No HTTPS application request is sent.
        let _tls = secure(stream, host, config).await?;
    } else {
        let request_target = if route.is_some() {
            url.to_string()
        } else {
            let mut path = url.path().to_owned();
            if let Some(query) = url.query() {
                path.push('?');
                path.push_str(query);
            }
            path
        };
        let request = format!(
            "HEAD {request_target} HTTP/1.1\r\nHost: {authority}\r\nConnection: close\r\n{}\r\n",
            authorization(route)
        );
        stream
            .write_all(request.as_bytes())
            .await
            .map_err(|_| "无法发送 HEAD 请求")?;
        // Any valid HTTP response proves a connection, including 4xx/5xx responses.
        response_status(&mut stream).await?;
    }
    Ok((started.elapsed().as_millis() as u64).max(1))
}
