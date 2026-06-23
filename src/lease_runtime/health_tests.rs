use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::test]
async fn probe_health_detects_http_to_https_redirects() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.expect("bind");
    let port = listener.local_addr().expect("addr").port();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept");
        let mut buffer = [0_u8; 1024];
        let _ = socket.read(&mut buffer).await.expect("read request");
        socket
            .write_all(
                b"HTTP/1.1 308 Permanent Redirect\r\nLocation: https://127.0.0.1/health\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            )
            .await
            .expect("write response");
    });

    let outcome = probe_health_with_protocol(port, "/health", "http")
        .await
        .expect("probe");
    assert_eq!(outcome, HealthProbeOutcome::RedirectedToHttps);

    server.await.expect("server");
}
