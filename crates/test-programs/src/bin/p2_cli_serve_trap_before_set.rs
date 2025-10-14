use test_programs::proxy;
use test_programs::wasi::http::types::{IncomingRequest, ResponseOutparam};

struct T;

proxy::export!(T);

impl proxy::exports::wasi::http::incoming_handler::Guest for T {
    fn handle(_request: IncomingRequest, _outparam: ResponseOutparam) {
        #[cfg(target_arch = "wasm32")]
        core::arch::wasm32::unreachable();
    }
}

fn main() {}
