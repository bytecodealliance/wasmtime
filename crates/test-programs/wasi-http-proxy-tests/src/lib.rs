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
    fn handle(request: IncomingRequest, _response_out: ResponseOutparam) {

        let method = bindings::wasi::http::types::incoming_request_method(request);

        println!("handling method: {method:?}!");
    }
}
