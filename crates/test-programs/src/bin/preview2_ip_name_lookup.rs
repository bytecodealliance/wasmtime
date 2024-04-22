use test_programs::wasi::sockets::instance_network::Network;
use test_programs::wasi::sockets::network::{ErrorCode, IpAddress};

fn main() {
    // Valid domains
    resolve("localhost").unwrap();
    resolve("example.com").unwrap();

    // NB: this is an actual real resolution, so it might time out, might cause
    // issues, etc. This result is ignored to prevent flaky failures in CI.
    let _ = resolve("münchen.de");

    // Valid IP addresses
    assert_eq!(resolve_one("0.0.0.0").unwrap(), IpAddress::IPV4_UNSPECIFIED);
    assert_eq!(resolve_one("127.0.0.1").unwrap(), IpAddress::IPV4_LOOPBACK);
    assert_eq!(
        resolve_one("192.0.2.0").unwrap(),
        IpAddress::Ipv4((192, 0, 2, 0))
    );
    assert_eq!(resolve_one("::").unwrap(), IpAddress::IPV6_UNSPECIFIED);
    assert_eq!(resolve_one("::1").unwrap(), IpAddress::IPV6_LOOPBACK);
    assert_eq!(resolve_one("[::]").unwrap(), IpAddress::IPV6_UNSPECIFIED);
    assert_eq!(
        resolve_one("2001:0db8:0:0:0:0:0:0").unwrap(),
        IpAddress::Ipv6((0x2001, 0x0db8, 0, 0, 0, 0, 0, 0))
    );
    assert_eq!(
        resolve_one("dead:beef::").unwrap(),
        IpAddress::Ipv6((0xdead, 0xbeef, 0, 0, 0, 0, 0, 0))
    );
    assert_eq!(
        resolve_one("dead:beef::0").unwrap(),
        IpAddress::Ipv6((0xdead, 0xbeef, 0, 0, 0, 0, 0, 0))
    );
    assert_eq!(
        resolve_one("DEAD:BEEF::0").unwrap(),
        IpAddress::Ipv6((0xdead, 0xbeef, 0, 0, 0, 0, 0, 0))
    );

    // Invalid inputs
    assert_eq!(resolve("").unwrap_err(), ErrorCode::InvalidArgument);
    assert_eq!(resolve(" ").unwrap_err(), ErrorCode::InvalidArgument);
    assert_eq!(resolve("a.b<&>").unwrap_err(), ErrorCode::InvalidArgument);
    assert_eq!(
        resolve("127.0.0.1:80").unwrap_err(),
        ErrorCode::InvalidArgument
    );
    assert_eq!(resolve("[::]:80").unwrap_err(), ErrorCode::InvalidArgument);
    assert_eq!(
        resolve("http://example.com/").unwrap_err(),
        ErrorCode::InvalidArgument
    );
}

fn resolve(name: &str) -> Result<Vec<IpAddress>, ErrorCode> {
    Network::default().permissive_blocking_resolve_addresses(name)
}

fn resolve_one(name: &str) -> Result<IpAddress, ErrorCode> {
    Ok(resolve(name)?.first().unwrap().to_owned())
}
