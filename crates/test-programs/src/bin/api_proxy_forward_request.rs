use test_programs::wasi::http::types::{
    Headers, IncomingRequest, OutgoingBody, OutgoingResponse, ResponseOutparam,
};

struct T;

test_programs::proxy::export!(T);

impl test_programs::proxy::exports::wasi::http::incoming_handler::Guest for T {
    fn handle(request: IncomingRequest, outparam: ResponseOutparam) {
        let res = test_programs::http::request(
            request.method(),
            request.scheme().unwrap(),
            request.authority().unwrap().as_str(),
            request.path_with_query().unwrap().as_str(),
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

        let hdrs = Headers::from_list(&res.headers).unwrap();
        let resp = OutgoingResponse::new(hdrs);
        resp.set_status_code(res.status).expect("status code");
        let body = resp.body().expect("outgoing response");

        ResponseOutparam::set(outparam, Ok(resp));

        let out = body.write().expect("outgoing stream");
        out.blocking_write_and_flush(res.body.as_ref())
            .expect("writing response");

        drop(out);
        OutgoingBody::finish(body, None).expect("outgoing-body.finish");
    }
}

// Technically this should not be here for a proxy, but given the current
// framework for tests it's required since this file is built as a `bin`
fn main() {}
