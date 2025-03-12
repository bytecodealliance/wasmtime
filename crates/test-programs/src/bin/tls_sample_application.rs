use anyhow::{Context, Result};
use core::str;
use test_programs::wasi::sockets::network::{IpSocketAddress, Network};
use test_programs::wasi::sockets::tcp::{ShutdownType, TcpSocket};
use test_programs::wasi::tls::types::ClientHandshake;

fn make_tls_request(domain: &str) -> Result<String> {
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
        .unwrap();

    let (client_connection, tls_input, tls_output) =
        ClientHandshake::new(domain, tcp_input, tcp_output)
            .blocking_finish()
            .unwrap();

    tls_output.blocking_write_util(request.as_bytes()).unwrap();
    client_connection
        .blocking_close_output(&tls_output)
        .unwrap();
    socket.shutdown(ShutdownType::Send)?;
    let response = tls_input.blocking_read_to_end().unwrap();
    String::from_utf8(response).context("error converting response")
}

fn test_tls_sample_application() {
    // since this is testing remote endpoint to ensure system cert store works
    // the test uses a couple different endpoints to reduce the number of flakes
    const DOMAIN1: &'static str = "example.com";
    const DOMAIN2: &'static str = "api.github.com";

    let response1 = make_tls_request(DOMAIN1).unwrap();
    let response2 = make_tls_request(DOMAIN2).unwrap();

    assert!(
        response1.contains("HTTP/1.1 200 OK") || response2.contains("HTTP/1.1 200 OK"),
        "Neither response contains 200 OK"
    );
}

fn main() {
    test_tls_sample_application();
}
