use anyhow::Context;
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use test_programs::wasi::http::types::{Method, Scheme};

fn main() {
    // This address inside the TEST-NET-3 address block is expected to time out.
    let addr = SocketAddr::from(([203, 0, 113, 12], 80)).to_string();
    let timeout = Duration::from_millis(200);
    let start = Instant::now();
    let connect_timeout : Option<u64> = Some(timeout.as_nanos() as u64);
    let res = test_programs::http::request(
        Method::Get,
        Scheme::Http,
        &addr,
        "/get?some=arg&goes=here",
        None,
        None,
        connect_timeout,
        None,
        None,
    )
    .context("/get");

    assert!(res.is_err());

    let actual = start.elapsed();
    let tolerance = Duration::from_millis(100);
    let expected: Duration = timeout + tolerance;
    assert!(actual < expected);
}
