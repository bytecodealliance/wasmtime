use anyhow::{anyhow, Context, Result};
use core::str;
use test_programs::wasi::sockets::network::{IpSocketAddress, Network};
use test_programs::wasi::sockets::tcp::{ShutdownType, TcpSocket};
use test_programs::wasi::tls::types::ClientHandshake;

fn test_tls_sample_application(domain: &str) -> Result<()> {
    const PORT: u16 = 443;

    let request =
        format!("GET / HTTP/1.1\r\nHost: {domain}\r\nUser-Agent: wasmtime-wasi-rust\r\n\r\n");

    let net = Network::default();

    let Some(ip) = net
        .permissive_blocking_resolve_addresses(domain)
        .unwrap()
        .first()
        .map(|a| a.to_owned())
    else {
        return Err(anyhow::anyhow!("DNS lookup failed."));
    };

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
    client_connection
        .blocking_close_output(&tls_output)
        .context("closing tls connection failed")?;
    socket.shutdown(ShutdownType::Send)?;
    let response = tls_input
        .blocking_read_to_end()
        .context("reading http response failed")?;

    if String::from_utf8(response)?.contains("HTTP/1.1 200 OK") {
        Ok(())
    } else {
        Err(anyhow!("server did not respond with 200 OK"))
    }
}

fn main() {
    // since this is testing remote endpoints to ensure system cert store works
    // the test uses a couple different endpoints to reduce the number of flakes
    const DOMAINS: &'static [&'static str] = &["example.com", "api.github.com"];

    for &domain in DOMAINS {
        match test_tls_sample_application(domain) {
            Ok(()) => return,
            Err(e) => {
                eprintln!("Failed to make TLS request to {domain}: {e:#}");
            }
        }
    }

    panic!("All TLS requests failed.");
}
