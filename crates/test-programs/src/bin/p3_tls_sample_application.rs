use anyhow::{Context as _, Result, anyhow, bail};
use core::future::{Future as _, poll_fn};
use core::pin::pin;
use core::str;
use core::task::{Poll, ready};
use futures::try_join;
use test_programs::p3::wasi::sockets::ip_name_lookup::resolve_addresses;
use test_programs::p3::wasi::sockets::types::{IpAddress, IpSocketAddress, TcpSocket};
use test_programs::p3::wasi::tls;
use test_programs::p3::wasi::tls::client::Hello;
use test_programs::p3::wit_stream;
use wit_bindgen::StreamResult;

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

    let (sock_rx, sock_rx_fut) = sock.receive();
    let hello = Hello::new();
    hello
        .set_server_name(domain)
        .map_err(|()| anyhow!("failed to set SNI"))?;
    let (sock_tx, conn) = tls::client::connect(hello, sock_rx);
    let sock_tx_fut = sock.send(sock_tx);

    let mut conn = pin!(conn.into_future());
    let mut sock_rx_fut = pin!(sock_rx_fut.into_future());
    let mut sock_tx_fut = pin!(sock_tx_fut);
    let conn = poll_fn(|cx| match conn.as_mut().poll(cx) {
        Poll::Ready(Ok(conn)) => Poll::Ready(Ok(conn)),
        Poll::Ready(Err(())) => Poll::Ready(Err(anyhow!("tls handshake failed"))),
        Poll::Pending => match sock_tx_fut.as_mut().poll(cx) {
            Poll::Ready(Ok(())) => Poll::Ready(Err(anyhow!("Tx stream closed unexpectedly"))),
            Poll::Ready(Err(err)) => {
                Poll::Ready(Err(anyhow!("Tx stream closed with error: {err:?}")))
            }
            Poll::Pending => match ready!(sock_rx_fut.as_mut().poll(cx)) {
                Ok(_) => Poll::Ready(Err(anyhow!("Rx stream closed unexpectedly"))),
                Err(err) => Poll::Ready(Err(anyhow!("Rx stream closed with error: {err:?}"))),
            },
        },
    })
    .await?;

    let (mut req_tx, req_rx) = wit_stream::new();
    let (mut res_rx, result_fut) = tls::client::Handshake::finish(conn, req_rx);

    let res = Vec::with_capacity(8192);
    try_join!(
        async {
            let buf = req_tx.write_all(request.into()).await;
            assert_eq!(buf, []);
            drop(req_tx);
            Ok(())
        },
        async {
            let (result, buf) = res_rx.read(res).await;
            match result {
                StreamResult::Complete(..) => {
                    drop(res_rx);
                    let res = String::from_utf8(buf)?;
                    if res.contains("HTTP/1.1 200 OK") {
                        Ok(())
                    } else {
                        bail!("server did not respond with 200 OK: {res}")
                    }
                }
                StreamResult::Dropped => bail!("read dropped"),
                StreamResult::Cancelled => bail!("read cancelled"),
            }
        },
        async { result_fut.await.map_err(|()| anyhow!("TLS session failed")) },
        async { sock_rx_fut.await.context("TCP receipt failed") },
        async { sock_tx_fut.await.context("TCP transmit failed") },
    )?;
    Ok(())
}

/// This test sets up a TCP connection using one domain, and then attempts to
/// perform a TLS handshake using another unrelated domain. This should result
/// in a handshake error.
async fn test_tls_invalid_certificate(_domain: &str, ip: IpAddress) -> Result<()> {
    const BAD_DOMAIN: &'static str = "wrongdomain.localhost";

    let sock = TcpSocket::create(ip.family()).unwrap();
    sock.connect(IpSocketAddress::new(ip, PORT))
        .await
        .context("tcp connect failed")?;

    let (sock_rx, sock_rx_fut) = sock.receive();
    let hello = Hello::new();
    hello
        .set_server_name(BAD_DOMAIN)
        .map_err(|()| anyhow!("failed to set SNI"))?;
    let (sock_tx, conn) = tls::client::connect(hello, sock_rx);
    let sock_tx_fut = sock.send(sock_tx);

    try_join!(
        async {
            match conn.await {
                Err(()) => Ok(()),
                Ok(_) => panic!("expecting server name mismatch"),
            }
        },
        async { sock_rx_fut.await.context("TCP receipt failed") },
        async { sock_tx_fut.await.context("TCP transmit failed") },
    )?;
    Ok(())
}

async fn try_live_endpoints<'a, Fut>(test: impl Fn(&'a str, IpAddress) -> Fut)
where
    Fut: Future<Output = Result<()>> + 'a,
{
    // since this is testing remote endpoints to ensure system cert store works
    // the test uses a couple different endpoints to reduce the number of flakes
    const DOMAINS: &'static [&'static str] = &[
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
            test(&domain, ip).await
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
