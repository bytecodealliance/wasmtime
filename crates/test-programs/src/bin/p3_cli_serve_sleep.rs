use test_programs::p3::service;
use test_programs::p3::wasi::clocks::monotonic_clock;
use test_programs::p3::wasi::http::types::{ErrorCode, Request, Response};

struct T;

service::export!(T);

impl service::exports::wasi::http::handler::Guest for T {
    async fn handle(_request: Request) -> Result<Response, ErrorCode> {
        monotonic_clock::wait_for(u64::MAX).await;
        unreachable!()
    }
}

fn main() {
    unreachable!()
}
