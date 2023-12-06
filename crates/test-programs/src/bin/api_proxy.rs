pub mod bindings {
    use super::T;

    wit_bindgen::generate!({
        path: "../wasi-http/wit",
        world: "wasi:http/proxy",
        exports: {
            "wasi:http/incoming-handler": T,
        },
    });
}

use bindings::wasi::http::types::{IncomingRequest, ResponseOutparam};

struct T;

impl bindings::exports::wasi::http::incoming_handler::Guest for T {
    fn handle(request: IncomingRequest, outparam: ResponseOutparam) {
        assert!(request.scheme().is_some());
        assert!(request.authority().is_some());
        assert!(request.path_with_query().is_some());

        let header = String::from("custom-forbidden-header");
        let req_hdrs = request.headers();

        assert!(
            !req_hdrs.has(&header),
            "forbidden `custom-forbidden-header` found in request"
        );

        assert!(req_hdrs.delete(&header).is_err());
        assert!(req_hdrs.append(&header, &b"no".to_vec()).is_err());

        assert!(
            !req_hdrs.has(&header),
            "append of forbidden header succeeded"
        );

        let hdrs = bindings::wasi::http::types::Headers::new();
        let resp = bindings::wasi::http::types::OutgoingResponse::new(hdrs);
        let body = resp.body().expect("outgoing response");

        bindings::wasi::http::types::ResponseOutparam::set(outparam, Ok(resp));

        let out = body.write().expect("outgoing stream");
        out.blocking_write_and_flush(b"hello, world!")
            .expect("writing response");

        drop(out);
        bindings::wasi::http::types::OutgoingBody::finish(body, None)
            .expect("outgoing-body.finish");
    }
}

// Technically this should not be here for a proxy, but given the current
// framework for tests it's required since this file is built as a `bin`
fn main() {}
