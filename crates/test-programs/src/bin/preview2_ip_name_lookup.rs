use test_programs::wasi::sockets::network::{ErrorCode, IpAddress};
use test_programs::wasi::sockets::*;

fn main() {
    assert!(matches!(resolve("example.com"), Ok(_)));
    assert!(matches!(resolve("github.com"), Ok(_)));
    assert!(matches!(resolve("a.b<&>"), Err(ErrorCode::InvalidArgument)));
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
