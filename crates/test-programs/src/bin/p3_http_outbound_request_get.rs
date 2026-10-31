use anyhow::Context;
use test_programs::p3::wasi::http::types::{Method, Scheme};

struct Component;

test_programs::p3::export!(Component);

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        let addr = test_programs::p3::wasi::cli::environment::get_environment()
            .into_iter()
            .find_map(|(k, v)| k.eq("HTTP_SERVER").then_some(v))
            .unwrap();
        let res = test_programs::p3::http::request(
            Method::Get,
            Scheme::Http,
            &addr,
            "/get?some=arg&goes=here",
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .context("/get")
        .unwrap();

        println!("{addr} /get: {res:?}");
        assert_eq!(res.status, 200);
        let method = res.header("x-wasmtime-test-method").unwrap();
        assert_eq!(std::str::from_utf8(method).unwrap(), "GET");
        // HTTP/1.1 sends path and query only, while HTTP/2 clients may send
        // the absolute form, so accept either.
        let uri = res.header("x-wasmtime-test-uri").unwrap();
        let uri = std::str::from_utf8(uri).unwrap();
        assert!(
            uri.ends_with("/get?some=arg&goes=here"),
            "unexpected request URI: {uri}"
        );
        assert_eq!(res.body, b"");
        Ok(())
    }
}

fn main() {}
