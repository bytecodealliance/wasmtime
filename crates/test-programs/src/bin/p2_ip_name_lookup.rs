use test_programs::wasi::sockets::instance_network::Network;
use test_programs::wasi::sockets::network::{ErrorCode, IpAddress};

fn main() {
    // Valid domains
    assert_resolves("localhost");

    resolve_at_least_one_of(&[
        "example.com",
        "api.github.com",
        "docs.wasmtime.dev",
        "bytecodealliance.org",
        "www.rust-lang.org",
    ]);

    // NB: this is an actual real resolution, so it might time out, might cause
    // issues, etc. This result is ignored to prevent flaky failures in CI.
    let _ = resolve("münchen.de");

    // Valid IP addresses
    assert_resolves_to("0.0.0.0", IpAddress::IPV4_UNSPECIFIED);
    assert_resolves_to("127.0.0.1", IpAddress::IPV4_LOOPBACK);
    assert_resolves_to("192.0.2.0", IpAddress::Ipv4((192, 0, 2, 0)));
    assert_resolves_to("::", IpAddress::IPV6_UNSPECIFIED);
    assert_resolves_to("::1", IpAddress::IPV6_LOOPBACK);
    assert_resolves_to("[::]", IpAddress::IPV6_UNSPECIFIED);
    assert_resolves_to(
        "2001:0db8:0:0:0:0:0:0",
        IpAddress::Ipv6((0x2001, 0x0db8, 0, 0, 0, 0, 0, 0)),
    );
    assert_resolves_to(
        "dead:beef::",
        IpAddress::Ipv6((0xdead, 0xbeef, 0, 0, 0, 0, 0, 0)),
    );
    assert_resolves_to(
        "dead:beef::0",
        IpAddress::Ipv6((0xdead, 0xbeef, 0, 0, 0, 0, 0, 0)),
    );
    assert_resolves_to(
        "DEAD:BEEF::0",
        IpAddress::Ipv6((0xdead, 0xbeef, 0, 0, 0, 0, 0, 0)),
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

/// Attempts to resolve at least one of `domains`. Allows failure so long as one
/// succeeds. Intended to help make this test less flaky while still also
/// testing live services.
fn resolve_at_least_one_of(domains: &[&str]) {
    let mut timeouts = 0;
    for domain in domains {
        match resolve(domain) {
            Ok(_) => return,
            Err(e) => {
                eprintln!("failed to resolve `{domain}`: {e}");
                if let ErrorCode::Timeout = e {
                    timeouts += 1;
                }
            }
        }
    }

    // If everything timed out just assume this is a bad CI weather day.
    if timeouts == domains.len() {
        return;
    }
    panic!("should have been able to resolve at least one domain");
}

/// Asserts that `name` resolves successfully, tolerating timeouts.
///
/// All resolutions, even of IP address literals, seem to occasionally time out
/// on CI, so just ignore timeouts here.
fn assert_resolves(name: &str) -> Option<Vec<IpAddress>> {
    match resolve(name) {
        Ok(addresses) => Some(addresses),
        Err(ErrorCode::Timeout) => {
            eprintln!("resolution of `{name}` timed out, skipping");
            None
        }
        Err(e) => panic!("failed to resolve `{name}`: {e}"),
    }
}

/// Same as `assert_resolves`, additionally asserting that `name` resolved to
/// `expected` if the resolution didn't time out.
fn assert_resolves_to(name: &str, expected: IpAddress) {
    if let Some(addresses) = assert_resolves(name) {
        assert_eq!(addresses.first(), Some(&expected), "resolution of `{name}`");
    }
}

fn resolve(name: &str) -> Result<Vec<IpAddress>, ErrorCode> {
    Network::default().permissive_blocking_resolve_addresses(name)
}
