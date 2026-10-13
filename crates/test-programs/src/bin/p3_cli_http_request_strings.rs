use test_programs::p3::wasi::http::types::{Fields, Method, Request, Scheme};
use test_programs::p3::wit_future;

struct Component;

test_programs::p3::export!(Component);

/// Usage: `<field>=<len>...` where field is one of `method`, `path`, `scheme`,
/// or `authority`.
///
/// Sets each field, in order, on a single request to a valid string of exactly
/// `len` bytes, printing `ok` or `error received` for each.
impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        let (_trailers_tx, trailers_rx) = wit_future::new(|| Ok(None));
        let (req, _transmit) = Request::new(Fields::new(), None, trailers_rx, None);
        for arg in std::env::args().skip(1) {
            let (field, len) = arg.split_once('=').expect("expected <field>=<len>");
            let len: usize = len.parse().expect("len must be a number");
            let result = match field {
                "method" => req.set_method(&Method::Other("X".repeat(len))),
                "path" => req.set_path_with_query(Some(&format!("/{}", "a".repeat(len - 1)))),
                "scheme" => req.set_scheme(Some(&Scheme::Other("x".repeat(len)))),
                "authority" => req.set_authority(Some(&"a".repeat(len))),
                other => panic!("unknown field {other:?}"),
            };
            match result {
                Ok(()) => println!("ok"),
                Err(()) => println!("error received"),
            }
        }
        Ok(())
    }
}

fn main() {}
