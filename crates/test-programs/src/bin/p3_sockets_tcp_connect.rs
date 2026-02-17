use futures::join;
use test_programs::p3::wasi::sockets::types::{
    ErrorCode, IpAddress, IpAddressFamily, IpSocketAddress, TcpSocket,
};
use test_programs::sockets::supports_ipv6;

struct Component;

test_programs::p3::export!(Component);

const SOME_PORT: u16 = 47; // If the tests pass, this will never actually be connected to.

/// `0.0.0.0` / `::` is not a valid remote address in WASI.
async fn test_tcp_connect_unspec(family: IpAddressFamily) {
    let addr = IpSocketAddress::new(IpAddress::new_unspecified(family), SOME_PORT);
    let sock = TcpSocket::create(family).unwrap();

    assert_eq!(sock.connect(addr).await, Err(ErrorCode::InvalidArgument));
}

/// 0 is not a valid remote port.
async fn test_tcp_connect_port_0(family: IpAddressFamily) {
    let addr = IpSocketAddress::new(IpAddress::new_loopback(family), 0);
    let sock = TcpSocket::create(family).unwrap();

    assert_eq!(sock.connect(addr).await, Err(ErrorCode::InvalidArgument));
}

/// Connect should validate the address family.
async fn test_tcp_connect_wrong_family(family: IpAddressFamily) {
    let wrong_ip = match family {
        IpAddressFamily::Ipv4 => IpAddress::IPV6_LOOPBACK,
        IpAddressFamily::Ipv6 => IpAddress::IPV4_LOOPBACK,
    };
    let remote_addr = IpSocketAddress::new(wrong_ip, SOME_PORT);

    let sock = TcpSocket::create(family).unwrap();

    assert_eq!(
        sock.connect(remote_addr).await,
        Err(ErrorCode::InvalidArgument)
    );
}

/// Can only connect to unicast addresses.
async fn test_tcp_connect_non_unicast() {
    let ipv4_broadcast = IpSocketAddress::new(IpAddress::IPV4_BROADCAST, SOME_PORT);
    let ipv4_multicast = IpSocketAddress::new(IpAddress::Ipv4((224, 254, 0, 0)), SOME_PORT);
    let ipv6_multicast =
        IpSocketAddress::new(IpAddress::Ipv6((0xff00, 0, 0, 0, 0, 0, 0, 0)), SOME_PORT);

    let sock_v4 = TcpSocket::create(IpAddressFamily::Ipv4).unwrap();
    let sock_v6 = TcpSocket::create(IpAddressFamily::Ipv6).unwrap();

    assert_eq!(
        sock_v4.connect(ipv4_broadcast).await,
        Err(ErrorCode::InvalidArgument)
    );
    assert_eq!(
        sock_v4.connect(ipv4_multicast).await,
        Err(ErrorCode::InvalidArgument)
    );
    assert_eq!(
        sock_v6.connect(ipv6_multicast).await,
        Err(ErrorCode::InvalidArgument)
    );
}

async fn test_tcp_connect_dual_stack() {
    // Set-up:
    let v4_listener = TcpSocket::create(IpAddressFamily::Ipv4).unwrap();
    v4_listener
        .bind(IpSocketAddress::new(IpAddress::IPV4_LOOPBACK, 0))
        .unwrap();
    v4_listener.listen().unwrap();

    let v4_listener_addr = v4_listener.get_local_address().unwrap();
    let v6_listener_addr =
        IpSocketAddress::new(IpAddress::IPV4_MAPPED_LOOPBACK, v4_listener_addr.port());

    let v6_client = TcpSocket::create(IpAddressFamily::Ipv6).unwrap();

    // Tests:

    // Connecting to an IPv4 address on an IPv6 socket should fail:
    assert_eq!(
        v6_client.connect(v4_listener_addr).await,
        Err(ErrorCode::InvalidArgument)
    );
    // Connecting to an IPv4-mapped-IPv6 address on an IPv6 socket should fail:
    assert_eq!(
        v6_client.connect(v6_listener_addr).await,
        Err(ErrorCode::InvalidArgument)
    );
}

/// Client sockets can be explicitly bound.
async fn test_tcp_connect_explicit_bind(family: IpAddressFamily) {
    let ip = IpAddress::new_loopback(family);

    let (listener, mut accept) = {
        let bind_address = IpSocketAddress::new(ip, 0);
        let listener = TcpSocket::create(family).unwrap();
        listener.bind(bind_address).unwrap();
        let accept = listener.listen().unwrap();
        (listener, accept)
    };

    let listener_address = listener.get_local_address().unwrap();

    // Connect should work:
    join!(
        async {
            let client = TcpSocket::create(family).unwrap();
            client
                .bind(IpSocketAddress::new(IpAddress::new_unspecified(family), 0))
                .unwrap();
            println!("local address: {:?}", client.get_local_address().unwrap());
            client.connect(listener_address).await.unwrap();
            println!("local address: {:?}", client.get_local_address().unwrap());
        },
        async {
            accept.next().await.unwrap();
        }
    );
}

/// Connecting a TCP socket should update the local address to reflect the best
/// network path.
async fn test_tcp_connect_local_address_change(family: IpAddressFamily) {
    let ip_unspec = IpAddress::new_unspecified(family);
    let ip_loopback = IpAddress::new_loopback(family);

    let (listener, mut accept) = {
        let bind_address = IpSocketAddress::new(ip_loopback, 0);
        let listener = TcpSocket::create(family).unwrap();
        listener.bind(bind_address).unwrap();
        let accept = listener.listen().unwrap();
        (listener, accept)
    };

    join!(
        async {
            let listener_address = listener.get_local_address().unwrap();
            let client = TcpSocket::create(family).unwrap();
            client.bind(IpSocketAddress::new(ip_unspec, 0)).unwrap();

            let before = client.get_local_address().unwrap();
            client.connect(listener_address).await.unwrap();
            let after = client.get_local_address().unwrap();

            println!("local address changed from {before:?} to {after:?}");

            // Note: these assertions are based on observed behavior on Linux,
            // MacOS and Windows, but there is nothing in their official
            // documentation to corroborate this.
            assert_eq!(before.ip(), ip_unspec);
            assert_eq!(after.ip(), ip_loopback);
            assert_eq!(before.port(), after.port());
        },
        async {
            accept.next().await.unwrap();
        }
    );
}

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        test_tcp_connect_unspec(IpAddressFamily::Ipv4).await;
        test_tcp_connect_port_0(IpAddressFamily::Ipv4).await;
        test_tcp_connect_wrong_family(IpAddressFamily::Ipv4).await;
        test_tcp_connect_explicit_bind(IpAddressFamily::Ipv4).await;
        test_tcp_connect_local_address_change(IpAddressFamily::Ipv4).await;

        if supports_ipv6() {
            test_tcp_connect_unspec(IpAddressFamily::Ipv6).await;
            test_tcp_connect_port_0(IpAddressFamily::Ipv6).await;
            test_tcp_connect_wrong_family(IpAddressFamily::Ipv6).await;
            test_tcp_connect_non_unicast().await;
            test_tcp_connect_dual_stack().await;
            test_tcp_connect_explicit_bind(IpAddressFamily::Ipv6).await;
            test_tcp_connect_local_address_change(IpAddressFamily::Ipv6).await;
        }
        Ok(())
    }
}

fn main() {}
