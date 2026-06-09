use test_programs::p3::wasi::sockets::types::{
    ErrorCode, IpAddress, IpAddressFamily, IpSocketAddress, Ipv4SocketAddress, Ipv6SocketAddress,
    UdpSocket,
};

struct Component;

test_programs::p3::export!(Component);

// Send without prior `bind` or `connect` performs an implicit bind.
async fn test_udp_send_without_bind_or_connect(family: IpAddressFamily) {
    let message = b"Hello, world!";
    let remote_addr = IpSocketAddress::new(IpAddress::new_loopback(family), 42);

    let sock = UdpSocket::create(family).unwrap();

    assert!(matches!(sock.get_local_address(), Err(_)));

    assert!(matches!(
        sock.send(message.to_vec(), Some(remote_addr)).await,
        Ok(_)
    ));

    assert!(matches!(sock.get_local_address(), Ok(_)));
    assert!(matches!(sock.get_remote_address(), Err(_)));
}

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        test_udp_send_without_bind_or_connect(IpAddressFamily::Ipv4).await;
        test_udp_send_without_bind_or_connect(IpAddressFamily::Ipv6).await;

        test_wrong_address_family(IpAddressFamily::Ipv4).await;
        test_wrong_address_family(IpAddressFamily::Ipv6).await;

        Ok(())
    }
}
async fn test_wrong_address_family(family: IpAddressFamily) {
    let sock = UdpSocket::create(family).unwrap();

    let addr = match family {
        IpAddressFamily::Ipv4 => IpSocketAddress::Ipv6(Ipv6SocketAddress {
            port: 0,
            address: (0, 0, 0, 0, 0, 0, 0, 1),
            flow_info: 0,
            scope_id: 0,
        }),
        IpAddressFamily::Ipv6 => IpSocketAddress::Ipv4(Ipv4SocketAddress {
            port: 0,
            address: (127, 0, 0, 1),
        }),
    };

    let result = sock.send(vec![0; 1], Some(addr)).await;
    assert!(
        matches!(
            result,
            Err(ErrorCode::NotSupported
                | ErrorCode::InvalidArgument
                | ErrorCode::RemoteUnreachable
                | ErrorCode::Other(_))
        ),
        "bad error {result:?}"
    );
}

fn main() {}
