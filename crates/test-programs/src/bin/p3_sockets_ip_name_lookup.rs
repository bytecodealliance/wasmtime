use test_programs::p3::wasi::sockets::ip_name_lookup::{ErrorCode, resolve_addresses};
use test_programs::p3::wasi::sockets::types::IpAddress;

struct Component;

test_programs::p3::export!(Component);

async fn resolve_one(name: &str) -> Result<IpAddress, ErrorCode> {
    Ok(resolve_addresses(name.into())
        .await?
        .first()
        .unwrap()
        .to_owned())
}

/// Attempts to resolve at least one of `domains`. Allows failure so long as one
/// succeeds. Intended to help make this test less flaky while still also
/// testing live services.
async fn resolve_at_least_one_of(domains: &[&str]) {
    for domain in domains {
        match resolve_one(domain).await {
            Ok(_) => return,
            Err(e) => eprintln!("failed to resolve `{domain}`: {e}"),
        }
    }

    panic!("should have been able to resolve at least one domain");
}

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        // Valid domains
        resolve_one("localhost").await.unwrap();

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
        assert_eq!(
            resolve_one("0.0.0.0").await.unwrap(),
            IpAddress::IPV4_UNSPECIFIED
        );
        assert_eq!(
            resolve_one("127.0.0.1").await.unwrap(),
            IpAddress::IPV4_LOOPBACK
        );
        assert_eq!(
            resolve_one("192.0.2.0").await.unwrap(),
            IpAddress::Ipv4((192, 0, 2, 0))
        );
        assert_eq!(
            resolve_one("::").await.unwrap(),
            IpAddress::IPV6_UNSPECIFIED
        );
        assert_eq!(resolve_one("::1").await.unwrap(), IpAddress::IPV6_LOOPBACK);
        assert_eq!(
            resolve_one("[::]").await.unwrap(),
            IpAddress::IPV6_UNSPECIFIED
        );
        assert_eq!(
            resolve_one("2001:0db8:0:0:0:0:0:0").await.unwrap(),
            IpAddress::Ipv6((0x2001, 0x0db8, 0, 0, 0, 0, 0, 0))
        );
        assert_eq!(
            resolve_one("dead:beef::").await.unwrap(),
            IpAddress::Ipv6((0xdead, 0xbeef, 0, 0, 0, 0, 0, 0))
        );
        assert_eq!(
            resolve_one("dead:beef::0").await.unwrap(),
            IpAddress::Ipv6((0xdead, 0xbeef, 0, 0, 0, 0, 0, 0))
        );
        assert_eq!(
            resolve_one("DEAD:BEEF::0").await.unwrap(),
            IpAddress::Ipv6((0xdead, 0xbeef, 0, 0, 0, 0, 0, 0))
        );

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
