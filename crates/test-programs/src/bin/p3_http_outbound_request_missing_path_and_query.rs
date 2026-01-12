use test_programs::p3::wasi::http::handler::handle;
use test_programs::p3::wasi::http::types::{Fields, Method, Request, Scheme};
use test_programs::p3::wit_future;

struct Component;

test_programs::p3::export!(Component);

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        let fields = Fields::new();
        let (_, rx) = wit_future::new(|| Ok(None));
        let (req, _) = Request::new(fields, None, rx, None);
        req.set_method(&Method::Get).unwrap();
        req.set_scheme(Some(&Scheme::Https)).unwrap();
        req.set_authority(Some("example.com")).unwrap();

        // Don't set path/query
        // req.set_path_with_query(Some("/")).unwrap();

        let res = handle(req).await;
        assert!(res.is_err());
        Ok(())
    }
}

fn main() {}
