use test_programs::wasi::sockets::network::{ErrorCode, IpAddress};
use test_programs::wasi::sockets::*;

fn main() {
    // Valid domains
    resolve("localhost").unwrap();
    resolve("example.com").unwrap();
    resolve("münchen.de").unwrap();

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
    let network = instance_network::instance_network();

    match network.blocking_resolve_addresses(name) {
        // The following error codes signal that the input passed validation
        // and a lookup was actually attempted, but failed. Ignore these to
        // make the CI tests less flaky:
        Err(
            ErrorCode::NameUnresolvable
            | ErrorCode::TemporaryResolverFailure
            | ErrorCode::PermanentResolverFailure,
        ) => Ok(vec![]),
        r => r,
    }
}

fn resolve_one(name: &str) -> Result<IpAddress, ErrorCode> {
    Ok(resolve(name)?.first().unwrap().to_owned())
}
