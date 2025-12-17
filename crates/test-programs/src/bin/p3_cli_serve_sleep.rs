use test_programs::p3::proxy;
use test_programs::p3::wasi::clocks::monotonic_clock;
use test_programs::p3::wasi::http::types::{ErrorCode, Request, Response};

struct T;

proxy::export!(T);

impl proxy::exports::wasi::http::handler::Guest for T {
    async fn handle(_request: Request) -> Result<Response, ErrorCode> {
        monotonic_clock::wait_for(u64::MAX).await;
        unreachable!()
    }
}

fn main() {
    unreachable!()
}
