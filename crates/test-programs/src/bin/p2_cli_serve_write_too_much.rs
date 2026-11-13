//! Writes more bytes to stdout than the check-write budget allows.

use test_programs::proxy;
use test_programs::wasi::cli::stdout::get_stdout;
use test_programs::wasi::http::types::{
    Fields, IncomingRequest, OutgoingResponse, ResponseOutparam,
};

struct T;

proxy::export!(T);

impl proxy::exports::wasi::http::incoming_handler::Guest for T {
    fn handle(_request: IncomingRequest, outparam: ResponseOutparam) {
        let out = get_stdout();
        let budget = out.check_write().unwrap();
        let data = vec![0; usize::try_from(budget).unwrap() + 1];
        let _ = out.write(&data);
        let resp = OutgoingResponse::new(Fields::new());
        ResponseOutparam::set(outparam, Ok(resp));
    }
}

fn main() {}
