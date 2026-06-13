use test_programs::p3::wasi::sockets::types::{
    ErrorCode, IpAddress, IpAddressFamily, IpSocketAddress, UdpSocket,
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

async fn test_unspecified_remote_addr(family: IpAddressFamily) {
    let sock = UdpSocket::create(family).unwrap();
    let unspec = IpSocketAddress::new(IpAddress::new_unspecified(family), 1234);
    let result = sock.send(vec![0; 1], Some(unspec)).await;

    assert!(matches!(result, Err(ErrorCode::InvalidArgument)));
}

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        test_udp_send_without_bind_or_connect(IpAddressFamily::Ipv4).await;
        test_udp_send_without_bind_or_connect(IpAddressFamily::Ipv6).await;

        test_wrong_address_family(IpAddressFamily::Ipv4).await;
        test_wrong_address_family(IpAddressFamily::Ipv6).await;

        test_unspecified_remote_addr(IpAddressFamily::Ipv4).await;
        test_unspecified_remote_addr(IpAddressFamily::Ipv6).await;

        Ok(())
    }
}
async fn test_wrong_address_family(family: IpAddressFamily) {
    let wrong_family = match family {
        IpAddressFamily::Ipv4 => IpAddressFamily::Ipv6,
        IpAddressFamily::Ipv6 => IpAddressFamily::Ipv4,
    };
    let addr = IpSocketAddress::new(IpAddress::new_loopback(wrong_family), 1234);

    let sock = UdpSocket::create(family).unwrap();
    let result = sock.send(vec![0; 1], Some(addr)).await;
    assert!(
        matches!(result, Err(ErrorCode::InvalidArgument)),
        "bad error {result:?}"
    );
}

fn main() {}
