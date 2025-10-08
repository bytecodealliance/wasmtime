use test_programs::proxy;
use test_programs::wasi::http::types::{
    Fields, IncomingRequest, OutgoingResponse, ResponseOutparam, Scheme,
};

struct T;

proxy::export!(T);

impl proxy::exports::wasi::http::incoming_handler::Guest for T {
    fn handle(request: IncomingRequest, outparam: ResponseOutparam) {
        let authority = request.authority();
        let scheme = request.scheme();

        assert_eq!(authority.as_deref(), Some("localhost"));
        assert!(
            matches!(scheme, Some(Scheme::Http)),
            "bad scheme: {scheme:?}",
        );

        let resp = OutgoingResponse::new(Fields::new());
        ResponseOutparam::set(outparam, Ok(resp));
    }
}

fn main() {}
