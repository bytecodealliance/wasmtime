use test_programs::proxy;
use test_programs::wasi::http::types::{
    Fields, IncomingRequest, OutgoingBody, OutgoingResponse, ResponseOutparam,
};

struct T;

proxy::export!(T);

impl proxy::exports::wasi::http::incoming_handler::Guest for T {
    fn handle(request: IncomingRequest, outparam: ResponseOutparam) {
        let fields = Fields::new();
        let resp = OutgoingResponse::new(fields);
        let body = resp.body().expect("outgoing response");

        ResponseOutparam::set(outparam, Ok(resp));

        let out = body.write().expect("outgoing stream");
        out.blocking_write_and_flush(b"Hello, WASI!")
            .expect("writing response");

        if request.path_with_query().as_deref() == Some("/trap") {
            unreachable!();
        }

        drop(out);
        OutgoingBody::finish(body, None).expect("outgoing-body.finish");
    }
}

fn main() {}
