use test_programs::p3::wasi as wasip3;

struct Component;

test_programs::p3::export!(Component);

fn test_wasip3() {
    use wasip3::sockets::types::{
        ErrorCode, IpAddressFamily, IpSocketAddress, Ipv4SocketAddress, TcpSocket,
    };

    // explicit bind disallowed
    let a = TcpSocket::create(IpAddressFamily::Ipv4).unwrap();
    let err = a
        .bind(IpSocketAddress::Ipv4(Ipv4SocketAddress {
            port: 0,
            address: (127, 0, 0, 1),
        }))
        .unwrap_err();
    assert!(matches!(err, ErrorCode::AccessDenied), "bad error {err:?}");

    // implicit bind disallowed
    let a = TcpSocket::create(IpAddressFamily::Ipv4).unwrap();
    let err = a.listen().unwrap_err();
    assert!(matches!(err, ErrorCode::AccessDenied), "bad error {err:?}");
}

fn test_wasip2() {
    use test_programs::wasi::sockets::network::{
        ErrorCode, IpAddressFamily, IpSocketAddress, Ipv4SocketAddress, Network,
    };
    use test_programs::wasi::sockets::tcp::TcpSocket;

    // explicit bind disallowed
    let sock = TcpSocket::new(IpAddressFamily::Ipv4).unwrap();
    let net = Network::default();
    let err = sock
        .blocking_bind(
            &net,
            IpSocketAddress::Ipv4(Ipv4SocketAddress {
                port: 0,
                address: (127, 0, 0, 1),
            }),
        )
        .unwrap_err();
    assert!(matches!(err, ErrorCode::AccessDenied), "bad error {err:?}");

    // implicit bind disallowed via the p2 state machine
    let sock = TcpSocket::new(IpAddressFamily::Ipv4).unwrap();
    assert!(matches!(
        sock.blocking_listen(),
        Err(ErrorCode::InvalidState)
    ));
}

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        test_wasip2();
        test_wasip3();
        Ok(())
    }
}

fn main() {
    unreachable!();
}
