use test_programs::proxy;
use test_programs::wasi::http::types::{IncomingRequest, ResponseOutparam};

struct T;

proxy::export!(T);

impl proxy::exports::wasi::http::incoming_handler::Guest for T {
    fn handle(_: IncomingRequest, _outparam: ResponseOutparam) {
        // About a second worth of busy loop. This should be plenty to test
        // 100µs MMU interrupts with.
        for i in 0..200_000_000u64 {
            std::hint::black_box(i);
        }
        // Worker should time out before hitting this.
        unreachable!()
    }
}

fn main() {}
