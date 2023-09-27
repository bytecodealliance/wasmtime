pub mod bindings {
    use super::T;

    wit_bindgen::generate!({
        path: "../../wasi-http/wit",
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
        let method = bindings::wasi::http::types::incoming_request_method(request);

        let hdrs = bindings::wasi::http::types::new_fields(&[]);
        let resp = bindings::wasi::http::types::new_outgoing_response(200, hdrs);
        let body =
            bindings::wasi::http::types::outgoing_response_write(resp).expect("outgoing response");

        bindings::wasi::http::types::set_response_outparam(outparam, Ok(resp));

        let out = bindings::wasi::http::types::outgoing_body_write(body).expect("outgoing stream");
        bindings::wasi::io::streams::blocking_write_and_flush(out, b"hello, world!")
            .expect("writing response");

        println!("handling method: {method:?}!");
    }
}
