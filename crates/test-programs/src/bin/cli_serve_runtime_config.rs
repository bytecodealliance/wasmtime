use test_programs::proxy;
use test_programs::wasi::{
    config::runtime,
    http::types::{Fields, IncomingRequest, OutgoingBody, OutgoingResponse, ResponseOutparam},
};

struct T;

proxy::export!(T);

impl proxy::exports::wasi::http::incoming_handler::Guest for T {
    fn handle(_: IncomingRequest, outparam: ResponseOutparam) {
        let fields = Fields::new();
        let resp = OutgoingResponse::new(fields);
        let body = resp.body().expect("outgoing response");

        ResponseOutparam::set(outparam, Ok(resp));

        let out = body.write().expect("outgoing stream");
        let config = runtime::get("hello").unwrap().unwrap();
        out.blocking_write_and_flush(config.as_bytes())
            .expect("writing response");

        drop(out);
        OutgoingBody::finish(body, None).expect("outgoing-body.finish");
    }
}

fn main() {}
