//! Writes more bytes to a TLS stream than `check-write` allowed.

use anyhow::{Context, Result, anyhow};
use test_programs::wasi::sockets::network::{IpAddress, IpSocketAddress, Network};
use test_programs::wasi::sockets::tcp::TcpSocket;
use test_programs::wasi::tls::types::ClientHandshake;

const PORT: u16 = 443;

fn write_over_permit(domain: &str, ip: IpAddress) -> Result<()> {
    let net = Network::default();

    let socket = TcpSocket::new(ip.family()).unwrap();
    let (tcp_input, tcp_output) = socket
        .blocking_connect(&net, IpSocketAddress::new(ip, PORT))
        .context("tcp connect failed")?;

    let (_connection, _tls_input, tls_output) = ClientHandshake::new(domain, tcp_input, tcp_output)
        .blocking_finish()
        .context("tls handshake failed")?;

    let permit = tls_output
        .check_write()
        .map_err(|e| anyhow!("check_write failed: {e:?}"))?;
    eprintln!("check-write permit: {permit}");

    let data = vec![0u8; usize::try_from(permit).unwrap() + 1];
    // The host is expected to trap here. Reaching the next line means the
    // oversized write was accepted.
    let _ = tls_output.write(&data);
    eprintln!("write returned without trapping");
    Ok(())
}

fn try_live_endpoints(test: impl Fn(&str, IpAddress) -> Result<()>) {
    const DOMAINS: &[&str] = &[
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

    panic!("all endpoints failed");
}

fn main() {
    try_live_endpoints(write_over_permit);
}
