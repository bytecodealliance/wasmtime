use futures::join;
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

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        // Valid domains
        let (res0, res1) = join!(
            resolve_addresses("localhost".into()),
            resolve_addresses("example.com".into())
        );
        if res0.is_err() && res1.is_err() {
            panic!("should have been able to resolve at least one domain");
        }

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
        assert_eq!(
            resolve_addresses("".into()).await.unwrap_err(),
            ErrorCode::InvalidArgument
        );
        assert_eq!(
            resolve_addresses(" ".into()).await.unwrap_err(),
            ErrorCode::InvalidArgument
        );
        assert_eq!(
            resolve_addresses("a.b<&>".into()).await.unwrap_err(),
            ErrorCode::InvalidArgument
        );
        assert_eq!(
            resolve_addresses("127.0.0.1:80".into()).await.unwrap_err(),
            ErrorCode::InvalidArgument
        );
        assert_eq!(
            resolve_addresses("[::]:80".into()).await.unwrap_err(),
            ErrorCode::InvalidArgument
        );
        assert_eq!(
            resolve_addresses("http://example.com/".into())
                .await
                .unwrap_err(),
            ErrorCode::InvalidArgument
        );
        Ok(())
    }
}

fn main() {}
