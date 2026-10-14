use super::{connection, route::ProxyRoute};
use std::{future::Future, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

fn run<T>(future: impl Future<Output = T>) -> T {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            tokio::time::timeout(Duration::from_secs(3), future)
                .await
                .unwrap()
        })
}

fn certificates() -> (Arc<rustls::ClientConfig>, Arc<rustls::ServerConfig>) {
    let rcgen::CertifiedKey { cert, key_pair } =
        rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let key = rustls::pki_types::PrivatePkcs8KeyDer::from(key_pair.serialize_der());
    let mut roots = rustls::RootCertStore::empty();
    roots.add(cert.der().clone()).unwrap();
    let client = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_root_certificates(roots)
    .with_no_client_auth();
    let server = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_no_client_auth()
    .with_single_cert(vec![cert.der().clone()], key.into())
    .unwrap();
    (Arc::new(client), Arc::new(server))
}

async fn request(stream: &mut (impl AsyncRead + Unpin)) -> String {
    let mut bytes = Vec::new();
    while !bytes.ends_with(b"\r\n\r\n") {
        assert!(bytes.len() < 4096);
        bytes.push(stream.read_u8().await.unwrap());
    }
    String::from_utf8(bytes).unwrap()
}

async fn no_application_request(stream: tokio::net::TcpStream, config: Arc<rustls::ServerConfig>) {
    match tokio_rustls::TlsAcceptor::from(config).accept(stream).await {
        Ok(mut tls) => {
            let mut bytes = [0; 1024];
            if let Ok(Ok(count)) =
                tokio::time::timeout(Duration::from_millis(200), tls.read(&mut bytes)).await
            {
                assert_eq!(count, 0, "HTTPS connection test must not send GET or HEAD");
            }
        }
        Err(error) => {
            // Closing immediately after client handshake can interrupt server ticket writes.
            assert!(matches!(
                error.kind(),
                std::io::ErrorKind::UnexpectedEof
                    | std::io::ErrorKind::BrokenPipe
                    | std::io::ErrorKind::ConnectionReset
            ));
        }
    }
}

#[test]
fn https_stops_at_verified_tls_without_an_http_request() {
    run(async {
        let (client, server) = certificates();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            no_application_request(stream, server).await;
        });
        let url = reqwest::Url::parse(&format!(
            "https://localhost:{}/ignored?path=yes",
            address.port()
        ))
        .unwrap();
        assert!(connection::measure(&url, None, client).await.unwrap() >= 1);
        task.await.unwrap();
    });
}

#[test]
fn https_uses_a_validated_connect_tunnel_before_tls() {
    run(async {
        let (client, server) = certificates();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            assert!(
                request(&mut stream)
                    .await
                    .starts_with("CONNECT localhost:443 HTTP/1.1\r\n")
            );
            stream
                .write_all(b"HTTP/1.1 200 Connection established\r\n")
                .await
                .unwrap();
            tokio::task::yield_now().await;
            stream
                .write_all(b"Proxy-Agent: synthetic-test\r\n\r\n")
                .await
                .unwrap();
            no_application_request(stream, server).await;
        });
        let route = ProxyRoute {
            host: "127.0.0.1".into(),
            port: address.port(),
            secure: false,
            authorization: None,
        };
        let url = reqwest::Url::parse("https://localhost/ignored").unwrap();
        assert!(
            connection::measure(&url, Some(&route), client)
                .await
                .unwrap()
                >= 1
        );
        task.await.unwrap();
    });
}

#[test]
fn http_uses_head_and_never_downloads_the_body() {
    run(async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            assert!(
                request(&mut stream)
                    .await
                    .starts_with("HEAD /custom?check=yes HTTP/1.1\r\n")
            );
            // A 4xx response still proves connectivity; no body is provided.
            stream
                .write_all(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 1000000\r\n\r\n")
                .await
                .unwrap();
        });
        let url = reqwest::Url::parse(&format!("http://{address}/custom?check=yes")).unwrap();
        assert!(
            connection::measure(&url, None, connection::tls_config())
                .await
                .unwrap()
                >= 1
        );
        task.await.unwrap();
    });
}

#[test]
fn untrusted_tls_is_a_failure_without_disabling_verification() {
    run(async {
        let (_, server) = certificates();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let _ = tokio_rustls::TlsAcceptor::from(server).accept(stream).await;
        });
        let url = reqwest::Url::parse(&format!("https://localhost:{}/", address.port())).unwrap();
        assert!(
            connection::measure(&url, None, connection::tls_config())
                .await
                .unwrap_err()
                .contains("TLS 握手失败")
        );
        task.await.unwrap();
    });
}

#[test]
fn rejected_proxy_and_missing_response_never_become_a_delay() {
    run(async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let _ = request(&mut stream).await;
            stream
                .write_all(b"HTTP/1.1 407 Proxy Authentication Required\r\n\r\n")
                .await
                .unwrap();
        });
        let route = ProxyRoute {
            host: "127.0.0.1".into(),
            port: address.port(),
            secure: false,
            authorization: None,
        };
        let url = reqwest::Url::parse("https://localhost/").unwrap();
        assert!(
            connection::measure(&url, Some(&route), connection::tls_config())
                .await
                .unwrap_err()
                .contains("HTTP 407")
        );
        task.await.unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            let (_stream, _) = listener.accept().await.unwrap();
            tokio::time::sleep(Duration::from_millis(200)).await;
        });
        let url = reqwest::Url::parse(&format!("http://{address}/")).unwrap();
        assert!(
            tokio::time::timeout(
                Duration::from_millis(50),
                connection::measure(&url, None, connection::tls_config())
            )
            .await
            .is_err()
        );
        task.await.unwrap();
    });
}
