//! This test assumes that it will be run without ip lookup support enabled
use test_programs::wasi::sockets::{
    ip_name_lookup::{ErrorCode, IpAddress},
    network::Network,
};

fn main() {
    let res = resolve("example.com");
    eprintln!("Result of resolve: {res:?}");
    assert!(matches!(res, Err(ErrorCode::PermanentResolverFailure)));
}

fn resolve(name: &str) -> Result<Vec<IpAddress>, ErrorCode> {
    Network::default().permissive_blocking_resolve_addresses(name)
}
