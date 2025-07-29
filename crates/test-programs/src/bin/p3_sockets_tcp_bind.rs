use futures::join;
use test_programs::p3::sockets::attempt_random_port;
use test_programs::p3::wasi::sockets::types::{
    ErrorCode, IpAddress, IpAddressFamily, IpSocketAddress, TcpSocket,
};
use test_programs::p3::wit_stream;
use wit_bindgen::yield_blocking;

struct Component;

test_programs::p3::export!(Component);

/// Bind a socket and let the system determine a port.
fn test_tcp_bind_ephemeral_port(ip: IpAddress) {
    let bind_addr = IpSocketAddress::new(ip, 0);

    let sock = TcpSocket::new(ip.family());
    sock.bind(bind_addr).unwrap();

    let bound_addr = sock.local_address().unwrap();

    assert_eq!(bind_addr.ip(), bound_addr.ip());
    assert_ne!(bind_addr.port(), bound_addr.port());
}

/// Bind a socket on a specified port.
fn test_tcp_bind_specific_port(ip: IpAddress) {
    let sock = TcpSocket::new(ip.family());

    let bind_addr = attempt_random_port(ip, |bind_addr| sock.bind(bind_addr)).unwrap();

    let bound_addr = sock.local_address().unwrap();

    assert_eq!(bind_addr.ip(), bound_addr.ip());
    assert_eq!(bind_addr.port(), bound_addr.port());
}

/// Two sockets may not be actively bound to the same address at the same time.
fn test_tcp_bind_addrinuse(ip: IpAddress) {
    let bind_addr = IpSocketAddress::new(ip, 0);

    let sock1 = TcpSocket::new(ip.family());
    sock1.bind(bind_addr).unwrap();
    sock1.listen().unwrap();

    let bound_addr = sock1.local_address().unwrap();

    let sock2 = TcpSocket::new(ip.family());
    assert_eq!(sock2.bind(bound_addr), Err(ErrorCode::AddressInUse));
}

// The WASI runtime should set SO_REUSEADDR for us
async fn test_tcp_bind_reuseaddr(ip: IpAddress) {
    let client = TcpSocket::new(ip.family());

    let bind_addr = {
        let listener1 = TcpSocket::new(ip.family());

        let bind_addr = attempt_random_port(ip, |bind_addr| listener1.bind(bind_addr)).unwrap();

        let mut accept = listener1.listen().unwrap();

        let connect_addr =
            IpSocketAddress::new(IpAddress::new_loopback(ip.family()), bind_addr.port());
        join!(
            async {
                client.connect(connect_addr).await.unwrap();
            },
            async {
                let sock = accept.next().await.unwrap();
                let (mut data_tx, data_rx) = wit_stream::new();
                join!(
                    async {
                        sock.send(data_rx).await.unwrap();
                    },
                    async {
                        let remaining = data_tx.write_all(vec![0; 10]).await;
                        assert!(remaining.is_empty());
                        drop(data_tx);
                    }
                );
            },
        );

        bind_addr
    };

    // If SO_REUSEADDR was configured correctly, the following lines
    // shouldn't be affected by the TIME_WAIT state of the just closed
    // `listener1` socket.
    //
    // Note though that the way things are modeled in Wasmtime right now is that
    // the TCP socket is kept alive by a spawned task created in `listen`
    // meaning that to fully close the socket it requires the spawned task to
    // shut down. That may require yielding to the host or similar so try a few
    // times to let the host get around to closing the task while testing each
    // time to see if we can reuse the address. This loop is bounded because it
    // should complete "quickly".
    for _ in 0..10 {
        let listener2 = TcpSocket::new(ip.family());
        if listener2.bind(bind_addr).is_ok() {
            listener2.listen().unwrap();
            return;
        }
        yield_blocking();
    }

    panic!("looks like REUSEADDR isn't in use?");
}

// Try binding to an address that is not configured on the system.
fn test_tcp_bind_addrnotavail(ip: IpAddress) {
    let bind_addr = IpSocketAddress::new(ip, 0);

    let sock = TcpSocket::new(ip.family());

    assert_eq!(sock.bind(bind_addr), Err(ErrorCode::AddressNotBindable));
}

/// Bind should validate the address family.
fn test_tcp_bind_wrong_family(family: IpAddressFamily) {
    let wrong_ip = match family {
        IpAddressFamily::Ipv4 => IpAddress::IPV6_LOOPBACK,
        IpAddressFamily::Ipv6 => IpAddress::IPV4_LOOPBACK,
    };

    let sock = TcpSocket::new(family);
    let result = sock.bind(IpSocketAddress::new(wrong_ip, 0));

    assert!(matches!(result, Err(ErrorCode::InvalidArgument)));
}

/// Bind only works on unicast addresses.
fn test_tcp_bind_non_unicast() {
    let ipv4_broadcast = IpSocketAddress::new(IpAddress::IPV4_BROADCAST, 0);
    let ipv4_multicast = IpSocketAddress::new(IpAddress::Ipv4((224, 254, 0, 0)), 0);
    let ipv6_multicast = IpSocketAddress::new(IpAddress::Ipv6((0xff00, 0, 0, 0, 0, 0, 0, 0)), 0);

    let sock_v4 = TcpSocket::new(IpAddressFamily::Ipv4);
    let sock_v6 = TcpSocket::new(IpAddressFamily::Ipv6);

    assert!(matches!(
        sock_v4.bind(ipv4_broadcast),
        Err(ErrorCode::InvalidArgument)
    ));
    assert!(matches!(
        sock_v4.bind(ipv4_multicast),
        Err(ErrorCode::InvalidArgument)
    ));
    assert!(matches!(
        sock_v6.bind(ipv6_multicast),
        Err(ErrorCode::InvalidArgument)
    ));
}

fn test_tcp_bind_dual_stack() {
    let sock = TcpSocket::new(IpAddressFamily::Ipv6);
    let addr = IpSocketAddress::new(IpAddress::IPV4_MAPPED_LOOPBACK, 0);

    // Binding an IPv4-mapped-IPv6 address on a ipv6-only socket should fail:
    assert!(matches!(sock.bind(addr), Err(ErrorCode::InvalidArgument)));
}

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        const RESERVED_IPV4_ADDRESS: IpAddress = IpAddress::Ipv4((192, 0, 2, 0)); // Reserved for documentation and examples.
        const RESERVED_IPV6_ADDRESS: IpAddress =
            IpAddress::Ipv6((0x2001, 0x0db8, 0, 0, 0, 0, 0, 0)); // Reserved for documentation and examples.

        test_tcp_bind_ephemeral_port(IpAddress::IPV4_LOOPBACK);
        test_tcp_bind_ephemeral_port(IpAddress::IPV6_LOOPBACK);
        test_tcp_bind_ephemeral_port(IpAddress::IPV4_UNSPECIFIED);
        test_tcp_bind_ephemeral_port(IpAddress::IPV6_UNSPECIFIED);

        test_tcp_bind_specific_port(IpAddress::IPV4_LOOPBACK);
        test_tcp_bind_specific_port(IpAddress::IPV6_LOOPBACK);
        test_tcp_bind_specific_port(IpAddress::IPV4_UNSPECIFIED);
        test_tcp_bind_specific_port(IpAddress::IPV6_UNSPECIFIED);

        test_tcp_bind_reuseaddr(IpAddress::IPV4_LOOPBACK).await;
        test_tcp_bind_reuseaddr(IpAddress::IPV6_LOOPBACK).await;

        test_tcp_bind_addrinuse(IpAddress::IPV4_LOOPBACK);
        test_tcp_bind_addrinuse(IpAddress::IPV6_LOOPBACK);
        test_tcp_bind_addrinuse(IpAddress::IPV4_UNSPECIFIED);
        test_tcp_bind_addrinuse(IpAddress::IPV6_UNSPECIFIED);

        test_tcp_bind_addrnotavail(RESERVED_IPV4_ADDRESS);
        test_tcp_bind_addrnotavail(RESERVED_IPV6_ADDRESS);

        test_tcp_bind_wrong_family(IpAddressFamily::Ipv4);
        test_tcp_bind_wrong_family(IpAddressFamily::Ipv6);

        test_tcp_bind_non_unicast();

        test_tcp_bind_dual_stack();

        Ok(())
    }
}

fn main() {}
