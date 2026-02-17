use anyhow::{Context as _, Result, anyhow};
use core::future::Future;
use futures::try_join;
use test_programs::p3::wasi::sockets::ip_name_lookup::resolve_addresses;
use test_programs::p3::wasi::sockets::types::{IpAddress, IpSocketAddress, TcpSocket};
use test_programs::p3::wasi::tls::client::Connector;
use test_programs::p3::wit_stream;

struct Component;

test_programs::p3::export!(Component);

const PORT: u16 = 443;

async fn test_tls_sample_application(domain: &str, ip: IpAddress) -> Result<()> {
    let request = format!(
        "GET / HTTP/1.1\r\nHost: {domain}\r\nUser-Agent: wasmtime-wasi-rust\r\nConnection: close\r\n\r\n"
    );

    let sock = TcpSocket::create(ip.family()).unwrap();
    sock.connect(IpSocketAddress::new(ip, PORT))
        .await
        .context("tcp connect failed")?;

    let conn = Connector::new();

    let (sock_rx, sock_rx_fut) = sock.receive();
    let (tls_rx, tls_rx_fut) = conn.receive(sock_rx);

    let (mut data_tx, data_rx) = wit_stream::new();
    let (tls_tx, tls_tx_err_fut) = conn.send(data_rx);
    let sock_tx_fut = sock.send(tls_tx);

    try_join!(
        async {
            Connector::connect(conn, domain.into())
                .await
                .map_err(|err| {
                    anyhow!(err.to_debug_string()).context("failed to establish connection")
                })
        },
        async {
            let buf = data_tx.write_all(request.into()).await;
            assert!(buf.is_empty());
            drop(data_tx);
            Ok(())
        },
        async {
            let response = tls_rx.collect().await;
            let response = String::from_utf8(response)?;
            if response.contains("HTTP/1.1 200 OK") {
                Ok(())
            } else {
                Err(anyhow!("server did not respond with 200 OK: {response}"))
            }
        },
        async { sock_rx_fut.await.context("failed to receive ciphertext") },
        async { sock_tx_fut.await.context("failed to send ciphertext") },
        async {
            tls_rx_fut
                .await
                .map_err(|err| anyhow!(err.to_debug_string()))
                .context("failed to receive plaintext")
        },
        async {
            tls_tx_err_fut
                .await
                .map_err(|err| anyhow!(err.to_debug_string()))
                .context("failed to send plaintext")
        },
    )?;
    Ok(())
}

/// This test sets up a TCP connection using one domain, and then attempts to
/// perform a TLS handshake using another unrelated domain. This should result
/// in a handshake error.
async fn test_tls_invalid_certificate(_domain: &str, ip: IpAddress) -> Result<()> {
    const BAD_DOMAIN: &str = "wrongdomain.localhost";

    let sock = TcpSocket::create(ip.family()).unwrap();
    sock.connect(IpSocketAddress::new(ip, PORT))
        .await
        .context("tcp connect failed")?;

    let conn = Connector::new();

    let (sock_rx, sock_rx_fut) = sock.receive();
    let (tls_rx, tls_rx_fut) = conn.receive(sock_rx);

    let (_, data_rx) = wit_stream::new();
    let (tls_tx, tls_tx_err_fut) = conn.send(data_rx);
    let sock_tx_fut = sock.send(tls_tx);
    let res = try_join!(
        async {
            Connector::connect(conn, BAD_DOMAIN.into())
                .await
                .expect("`connect` failed");
            Ok(())
        },
        async {
            let response = tls_rx.collect().await;
            assert_eq!(response, []);
            Ok(())
        },
        async {
            sock_rx_fut.await.expect("failed to receive ciphertext");
            Ok(())
        },
        async {
            sock_tx_fut.await.expect("failed to send ciphertext");
            Ok(())
        },
        async { tls_rx_fut.await },
        async { tls_tx_err_fut.await },
    );
    match res {
        Err(e) => {
            let debug_string = e.to_debug_string();
            // We're expecting an error regarding certificates in some form or
            // another. When we add more TLS backends this naive check will
            // likely need to be revisited/expanded:
            if debug_string.contains("certificate") || debug_string.contains("HandshakeFailure") {
                return Ok(());
            }
            Err(anyhow!(debug_string))
        }
        Ok(_) => panic!("expecting server name mismatch"),
    }
}

async fn try_live_endpoints<'a, Fut>(test: impl Fn(&'a str, IpAddress) -> Fut)
where
    Fut: Future<Output = Result<()>> + 'a,
{
    // since this is testing remote endpoints to ensure system cert store works
    // the test uses a couple different endpoints to reduce the number of flakes
    const DOMAINS: &[&str] = &[
        "example.com",
        "api.github.com",
        "docs.wasmtime.dev",
        "bytecodealliance.org",
        "www.rust-lang.org",
    ];

    for &domain in DOMAINS {
        let result = (|| async {
            let ip = resolve_addresses(domain.into())
                .await?
                .first()
                .map(|a| a.to_owned())
                .ok_or_else(|| anyhow!("DNS lookup failed."))?;
            test(domain, ip).await
        })();

        match result.await {
            Ok(()) => return,
            Err(e) => {
                eprintln!("test for {domain} failed: {e:#}");
            }
        }
    }

    panic!("all tests failed");
}

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        println!("sample app");
        try_live_endpoints(test_tls_sample_application).await;
        println!("invalid cert");
        try_live_endpoints(test_tls_invalid_certificate).await;
        Ok(())
    }
}

fn main() {}
