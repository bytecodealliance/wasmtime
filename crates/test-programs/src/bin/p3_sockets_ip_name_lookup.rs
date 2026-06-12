use core::pin::pin;
use futures::future::{Either, select};
use test_programs::p3::wasi::clocks::monotonic_clock;
use test_programs::p3::wasi::sockets::ip_name_lookup::{ErrorCode, resolve_addresses};
use test_programs::p3::wasi::sockets::types::IpAddress;

struct Component;

test_programs::p3::export!(Component);

/// Resolves `name`, returning `None` if the resolution didn't complete within
/// enough time.
async fn resolve(name: &str) -> Option<Result<Vec<IpAddress>, ErrorCode>> {
    const TIMEOUT_NS: u64 = 1_000_000_000;
    let resolve = pin!(resolve_addresses(name.into()));
    let timeout = pin!(monotonic_clock::wait_for(TIMEOUT_NS));
    match select(resolve, timeout).await {
        Either::Left((result, _)) => Some(result),
        Either::Right(((), _)) => None,
    }
}

/// Asserts that `name` resolves successfully, tolerating timeouts.
///
/// Timed out resolutions are skipped rather than failing the test. See
/// `resolve` for why timeouts don't fail the test.
async fn assert_resolves(name: &str) -> Option<Vec<IpAddress>> {
    match resolve(name).await {
        Some(Ok(addresses)) => Some(addresses),
        None => {
            eprintln!("resolution of `{name}` timed out, skipping");
            None
        }
        Some(Err(e)) => panic!("failed to resolve `{name}`: {e}"),
    }
}

/// Same as `assert_resolves`, additionally asserting that `name` resolved to
/// `expected` if the resolution didn't time out.
async fn assert_resolves_to(name: &str, expected: IpAddress) {
    if let Some(addresses) = assert_resolves(name).await {
        assert_eq!(addresses.first(), Some(&expected), "resolution of `{name}`");
    }
}

/// Attempts to resolve at least one of `domains`. Allows failure so long as one
/// succeeds. Intended to help make this test less flaky while still also
/// testing live services.
async fn resolve_at_least_one_of(domains: &[&str]) {
    let mut timeouts = 0;
    for domain in domains {
        match resolve(domain).await {
            Some(Ok(_)) => return,
            None => timeouts += 1,
            Some(Err(e)) => eprintln!("failed to resolve `{domain}`: {e}"),
        }
    }

    // Ignore if everything times out.
    if timeouts == domains.len() {
        return;
    }

    panic!("should have been able to resolve at least one domain");
}

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        // Valid domains
        assert_resolves("localhost").await;

        resolve_at_least_one_of(&[
            "example.com",
            "api.github.com",
            "docs.wasmtime.dev",
            "bytecodealliance.org",
            "www.rust-lang.org",
        ])
        .await;

        // NB: this is an actual real resolution, so it might time out, might cause
        // issues, etc. This result is ignored to prevent flaky failures in CI.
        let _ = resolve_addresses("münchen.de".into()).await;

        // Valid IP addresses
        assert_resolves_to("0.0.0.0", IpAddress::IPV4_UNSPECIFIED).await;
        assert_resolves_to("127.0.0.1", IpAddress::IPV4_LOOPBACK).await;
        assert_resolves_to("192.0.2.0", IpAddress::Ipv4((192, 0, 2, 0))).await;
        assert_resolves_to("::", IpAddress::IPV6_UNSPECIFIED).await;
        assert_resolves_to("::1", IpAddress::IPV6_LOOPBACK).await;
        assert_resolves_to("[::]", IpAddress::IPV6_UNSPECIFIED).await;
        assert_resolves_to(
            "2001:0db8:0:0:0:0:0:0",
            IpAddress::Ipv6((0x2001, 0x0db8, 0, 0, 0, 0, 0, 0)),
        )
        .await;
        assert_resolves_to(
            "dead:beef::",
            IpAddress::Ipv6((0xdead, 0xbeef, 0, 0, 0, 0, 0, 0)),
        )
        .await;
        assert_resolves_to(
            "dead:beef::0",
            IpAddress::Ipv6((0xdead, 0xbeef, 0, 0, 0, 0, 0, 0)),
        )
        .await;
        assert_resolves_to(
            "DEAD:BEEF::0",
            IpAddress::Ipv6((0xdead, 0xbeef, 0, 0, 0, 0, 0, 0)),
        )
        .await;

        // Invalid inputs
        assert!(matches!(
            resolve_addresses("".into()).await.unwrap_err(),
            ErrorCode::InvalidArgument
        ));
        assert!(matches!(
            resolve_addresses(" ".into()).await.unwrap_err(),
            ErrorCode::InvalidArgument
        ));
        assert!(matches!(
            resolve_addresses("a.b<&>".into()).await.unwrap_err(),
            ErrorCode::InvalidArgument
        ));
        assert!(matches!(
            resolve_addresses("127.0.0.1:80".into()).await.unwrap_err(),
            ErrorCode::InvalidArgument
        ));
        assert!(matches!(
            resolve_addresses("[::]:80".into()).await.unwrap_err(),
            ErrorCode::InvalidArgument
        ));
        assert!(matches!(
            resolve_addresses("http://example.com/".into())
                .await
                .unwrap_err(),
            ErrorCode::InvalidArgument
        ));
        Ok(())
    }
}

fn main() {}
