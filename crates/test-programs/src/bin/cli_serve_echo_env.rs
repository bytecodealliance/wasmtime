use test_programs::proxy;
use test_programs::wasi::http::types::{
    Fields, IncomingRequest, OutgoingResponse, ResponseOutparam,
};

struct T;

proxy::export!(T);

impl proxy::exports::wasi::http::incoming_handler::Guest for T {
    fn handle(request: IncomingRequest, outparam: ResponseOutparam) {
        let headers = request.headers();
        let header_key = "env".to_string();
        let env_var = headers.get(&header_key);
        assert!(env_var.len() == 1, "should have exactly one `env` header");
        let key = std::str::from_utf8(&env_var[0]).unwrap();
        let fields = Fields::new();
        if let Ok(val) = std::env::var(key) {
            fields.set(&header_key, &[val.into_bytes()]).unwrap();
        }
        let resp = OutgoingResponse::new(fields);
        ResponseOutparam::set(outparam, Ok(resp));
    }
}

fn main() {}
