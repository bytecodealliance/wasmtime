use test_programs::sockets::supports_ipv6;
use test_programs::wasi::sockets::network::{ErrorCode, IpAddressFamily};
use test_programs::wasi::sockets::tcp::TcpSocket;

/// Socket must be explicitly bound before listening.
fn test_tcp_listen_without_bind(family: IpAddressFamily) {
    let sock = TcpSocket::new(family).unwrap();

    assert!(matches!(
        sock.blocking_listen(),
        Err(ErrorCode::InvalidState)
    ));
}

fn main() {
    test_tcp_listen_without_bind(IpAddressFamily::Ipv4);

    if supports_ipv6() {
        test_tcp_listen_without_bind(IpAddressFamily::Ipv6);
    }
}
