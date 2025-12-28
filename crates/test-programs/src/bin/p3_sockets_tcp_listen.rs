use test_programs::p3::wasi::sockets::types::{
    IpAddress, IpAddressFamily, IpSocketAddress, TcpSocket,
};
use test_programs::sockets::supports_ipv6;

struct Component;

test_programs::p3::export!(Component);

/// Listen should perform implicit bind.
fn test_tcp_listen_without_bind(family: IpAddressFamily) {
    let sock = TcpSocket::create(family).unwrap();

    assert!(matches!(sock.get_local_address(), Err(_)));
    assert!(matches!(sock.listen(), Ok(_)));
    assert!(matches!(sock.get_local_address(), Ok(_)));
}

/// Listen should work in combination with an explicit bind.
fn test_tcp_listen_with_bind(family: IpAddressFamily) {
    let bind_addr = IpSocketAddress::new(IpAddress::new_unspecified(family), 0);
    let sock = TcpSocket::create(family).unwrap();

    sock.bind(bind_addr).unwrap();
    let local_addr = sock.get_local_address().unwrap();

    assert!(matches!(sock.listen(), Ok(_)));
    assert_eq!(sock.get_local_address(), Ok(local_addr));
}

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        test_tcp_listen_without_bind(IpAddressFamily::Ipv4);
        test_tcp_listen_with_bind(IpAddressFamily::Ipv4);

        if supports_ipv6() {
            test_tcp_listen_without_bind(IpAddressFamily::Ipv6);
            test_tcp_listen_with_bind(IpAddressFamily::Ipv6);
        }

        Ok(())
    }
}

fn main() {}
