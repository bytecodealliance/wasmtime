use std::io::Write;
use test_programs::proxy;
use test_programs::wasi::http::types::{
    Fields, IncomingRequest, OutgoingResponse, ResponseOutparam,
};

struct T;

proxy::export!(T);

impl proxy::exports::wasi::http::incoming_handler::Guest for T {
    fn handle(_request: IncomingRequest, outparam: ResponseOutparam) {
        print!("this is half a print ");
        std::io::stdout().flush().unwrap();
        println!("to stdout");
        println!(); // empty line
        println!("after empty");

        eprint!("this is half a print ");
        std::io::stderr().flush().unwrap();
        eprintln!("to stderr");
        eprintln!(); // empty line
        eprintln!("after empty");

        let resp = OutgoingResponse::new(Fields::new());
        ResponseOutparam::set(outparam, Ok(resp));
    }
}

fn main() {}
