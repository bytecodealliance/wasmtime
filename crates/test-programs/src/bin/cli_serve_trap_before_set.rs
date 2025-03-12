use test_programs::proxy;
use test_programs::wasi::http::types::{IncomingRequest, ResponseOutparam};

struct T;

proxy::export!(T);

impl proxy::exports::wasi::http::incoming_handler::Guest for T {
    #[cfg(target_arch = "wasm32")]
    fn handle(_request: IncomingRequest, _outparam: ResponseOutparam) {
        core::arch::wasm32::unreachable();
    }
}

fn main() {}
