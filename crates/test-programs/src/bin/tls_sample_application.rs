use anyhow::{Context, Result, anyhow};
use core::str;
use test_programs::wasi::sockets::network::{IpAddress, IpSocketAddress, Network};
use test_programs::wasi::sockets::tcp::{ShutdownType, TcpSocket};
use test_programs::wasi::tls::types::ClientHandshake;

const PORT: u16 = 443;

fn test_tls_sample_application(domain: &str, ip: IpAddress) -> Result<()> {
    let request = format!(
        "GET / HTTP/1.1\r\nHost: {domain}\r\nUser-Agent: wasmtime-wasi-rust\r\nConnection: close\r\n\r\n"
    );

    let net = Network::default();

    let socket = TcpSocket::new(ip.family()).unwrap();
    let (tcp_input, tcp_output) = socket
        .blocking_connect(&net, IpSocketAddress::new(ip, PORT))
        .context("tcp connect failed")?;

    let (client_connection, tls_input, tls_output) =
        ClientHandshake::new(domain, tcp_input, tcp_output)
            .blocking_finish()
            .context("tls handshake failed")?;

    tls_output
        .blocking_write_util(request.as_bytes())
        .context("writing http request failed")?;
    let response = tls_input
        .blocking_read_to_end()
        .context("reading http response failed")?;
    client_connection
        .blocking_close_output(&tls_output)
        .context("closing tls connection failed")?;
    socket.shutdown(ShutdownType::Both)?;

    if String::from_utf8(response)?.contains("HTTP/1.1 200 OK") {
        Ok(())
    } else {
        Err(anyhow!("server did not respond with 200 OK"))
    }
}

/// This test sets up a TCP connection using one domain, and then attempts to
/// perform a TLS handshake using another unrelated domain. This should result
/// in a handshake error.
fn test_tls_invalid_certificate(_domain: &str, ip: IpAddress) -> Result<()> {
    const BAD_DOMAIN: &'static str = "wrongdomain.localhost";

    let net = Network::default();

    let socket = TcpSocket::new(ip.family()).unwrap();
    let (tcp_input, tcp_output) = socket
        .blocking_connect(&net, IpSocketAddress::new(ip, PORT))
        .context("tcp connect failed")?;

    match ClientHandshake::new(BAD_DOMAIN, tcp_input, tcp_output).blocking_finish() {
        Err(e) => {
            let debug_string = e.to_debug_string();
            // We're expecting an error regarding certificates in some form or
            // another. When we add more TLS backends this naive check will
            // likely need to be revisited/expanded:
            if debug_string.contains("certificate") || debug_string.contains("HandshakeFailure") {
                return Ok(());
            }
            Err(e.into())
        }
        Ok(_) => panic!("expecting server name mismatch"),
    }
}

fn try_live_endpoints(test: impl Fn(&str, IpAddress) -> Result<()>) {
    // since this is testing remote endpoints to ensure system cert store works
    // the test uses a couple different endpoints to reduce the number of flakes
    const DOMAINS: &'static [&'static str] = &[
        "example.com",
        "api.github.com",
        "docs.wasmtime.dev",
        "bytecodealliance.org",
        "www.rust-lang.org",
    ];

    let net = Network::default();

    for &domain in DOMAINS {
        let result = (|| {
            let ip = net
                .permissive_blocking_resolve_addresses(domain)?
                .first()
                .map(|a| a.to_owned())
                .ok_or_else(|| anyhow!("DNS lookup failed."))?;
            test(&domain, ip)
        })();

        match result {
            Ok(()) => return,
            Err(e) => {
                eprintln!("test for {domain} failed: {e:#}");
            }
        }
    }

    panic!("all tests failed");
}

fn main() {
    println!("sample app");
    try_live_endpoints(test_tls_sample_application);
    println!("invalid cert");
    try_live_endpoints(test_tls_invalid_certificate);
}
