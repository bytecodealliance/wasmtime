use test_programs::p3::sockets::attempt_random_port;
use test_programs::p3::wasi::sockets::types::{
    ErrorCode, IpAddress, IpAddressFamily, IpSocketAddress, UdpSocket,
};

struct Component;

test_programs::p3::export!(Component);

/// Bind a socket and let the system determine a port.
fn test_udp_bind_ephemeral_port(ip: IpAddress) {
    let bind_addr = IpSocketAddress::new(ip, 0);

    let sock = UdpSocket::new(ip.family());
    sock.bind(bind_addr).unwrap();

    let bound_addr = sock.local_address().unwrap();

    assert_eq!(bind_addr.ip(), bound_addr.ip());
    assert_ne!(bind_addr.port(), bound_addr.port());
}

/// Bind a socket on a specified port.
fn test_udp_bind_specific_port(ip: IpAddress) {
    let sock = UdpSocket::new(ip.family());

    let bind_addr = attempt_random_port(ip, |bind_addr| sock.bind(bind_addr)).unwrap();

    let bound_addr = sock.local_address().unwrap();

    assert_eq!(bind_addr.ip(), bound_addr.ip());
    assert_eq!(bind_addr.port(), bound_addr.port());
}

/// Two sockets may not be actively bound to the same address at the same time.
fn test_udp_bind_addrinuse(ip: IpAddress) {
    let bind_addr = IpSocketAddress::new(ip, 0);

    let sock1 = UdpSocket::new(ip.family());
    sock1.bind(bind_addr).unwrap();

    let bound_addr = sock1.local_address().unwrap();

    let sock2 = UdpSocket::new(ip.family());
    assert!(matches!(
        sock2.bind(bound_addr),
        Err(ErrorCode::AddressInUse)
    ));
}

// Try binding to an address that is not configured on the system.
fn test_udp_bind_addrnotavail(ip: IpAddress) {
    let bind_addr = IpSocketAddress::new(ip, 0);

    let sock = UdpSocket::new(ip.family());

    assert!(matches!(
        sock.bind(bind_addr),
        Err(ErrorCode::AddressNotBindable)
    ));
}

/// Bind should validate the address family.
fn test_udp_bind_wrong_family(family: IpAddressFamily) {
    let wrong_ip = match family {
        IpAddressFamily::Ipv4 => IpAddress::IPV6_LOOPBACK,
        IpAddressFamily::Ipv6 => IpAddress::IPV4_LOOPBACK,
    };

    let sock = UdpSocket::new(family);
    let result = sock.bind(IpSocketAddress::new(wrong_ip, 0));

    assert!(matches!(result, Err(ErrorCode::InvalidArgument)));
}

fn test_udp_bind_dual_stack() {
    let sock = UdpSocket::new(IpAddressFamily::Ipv6);
    let addr = IpSocketAddress::new(IpAddress::IPV4_MAPPED_LOOPBACK, 0);

    // Binding an IPv4-mapped-IPv6 address on a ipv6-only socket should fail:
    assert!(matches!(sock.bind(addr), Err(ErrorCode::InvalidArgument)));
}

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        const RESERVED_IPV4_ADDRESS: IpAddress = IpAddress::Ipv4((192, 0, 2, 0)); // Reserved for documentation and examples.
        const RESERVED_IPV6_ADDRESS: IpAddress =
            IpAddress::Ipv6((0x2001, 0x0db8, 0, 0, 0, 0, 0, 0)); // Reserved for documentation and examples.

        test_udp_bind_ephemeral_port(IpAddress::IPV4_LOOPBACK);
        test_udp_bind_ephemeral_port(IpAddress::IPV6_LOOPBACK);
        test_udp_bind_ephemeral_port(IpAddress::IPV4_UNSPECIFIED);
        test_udp_bind_ephemeral_port(IpAddress::IPV6_UNSPECIFIED);

        test_udp_bind_specific_port(IpAddress::IPV4_LOOPBACK);
        test_udp_bind_specific_port(IpAddress::IPV6_LOOPBACK);
        test_udp_bind_specific_port(IpAddress::IPV4_UNSPECIFIED);
        test_udp_bind_specific_port(IpAddress::IPV6_UNSPECIFIED);

        test_udp_bind_addrinuse(IpAddress::IPV4_LOOPBACK);
        test_udp_bind_addrinuse(IpAddress::IPV6_LOOPBACK);
        test_udp_bind_addrinuse(IpAddress::IPV4_UNSPECIFIED);
        test_udp_bind_addrinuse(IpAddress::IPV6_UNSPECIFIED);

        test_udp_bind_addrnotavail(RESERVED_IPV4_ADDRESS);
        test_udp_bind_addrnotavail(RESERVED_IPV6_ADDRESS);

        test_udp_bind_wrong_family(IpAddressFamily::Ipv4);
        test_udp_bind_wrong_family(IpAddressFamily::Ipv6);

        test_udp_bind_dual_stack();
        Ok(())
    }
}

fn main() {}
