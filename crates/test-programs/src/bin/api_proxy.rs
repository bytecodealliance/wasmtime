use test_programs::wasi::http::types::{
    Headers, IncomingRequest, OutgoingBody, OutgoingResponse, ResponseOutparam,
};

struct T;

test_programs::proxy::export!(T);

impl test_programs::proxy::exports::wasi::http::incoming_handler::Guest for T {
    fn handle(request: IncomingRequest, outparam: ResponseOutparam) {
        assert!(request.scheme().is_some());
        assert!(request.authority().is_some());
        assert!(request.path_with_query().is_some());

        test_filesystem();

        let header = String::from("custom-forbidden-header");
        let req_hdrs = request.headers();

        assert!(
            !req_hdrs.has(&header),
            "forbidden `custom-forbidden-header` found in request"
        );

        assert!(req_hdrs.delete(&header).is_err());
        assert!(req_hdrs.append(&header, b"no".as_ref()).is_err());

        assert!(
            !req_hdrs.has(&header),
            "append of forbidden header succeeded"
        );

        assert!(
            !req_hdrs.has("host"),
            "forbidden host header present in incoming request"
        );

        let hdrs = Headers::new();
        let resp = OutgoingResponse::new(hdrs);
        let body = resp.body().expect("outgoing response");

        ResponseOutparam::set(outparam, Ok(resp));

        let out = body.write().expect("outgoing stream");
        out.blocking_write_and_flush(b"hello, world!")
            .expect("writing response");

        drop(out);
        OutgoingBody::finish(body, None).expect("outgoing-body.finish");
    }
}

// Technically this should not be here for a proxy, but given the current
// framework for tests it's required since this file is built as a `bin`
fn main() {}

fn test_filesystem() {
    assert!(std::fs::File::open(".").is_err());
}
