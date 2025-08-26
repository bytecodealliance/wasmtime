use anyhow::Context;
use std::io::{self, Read};
use test_programs::p3::wasi::http::types::{Method, Scheme};

struct Component;

test_programs::p3::export!(Component);

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        // Make sure the final body is larger than 1024*1024, but we cannot allocate
        // so much memory directly in the wasm program, so we use the `repeat`
        // method to increase the body size.
        const LEN: usize = 1024;
        const REPEAT: usize = 1025;
        let mut buffer = [0; LEN];
        let addr = test_programs::p3::wasi::cli::environment::get_environment()
            .into_iter()
            .find_map(|(k, v)| k.eq("HTTP_SERVER").then_some(v))
            .unwrap();
        io::repeat(0b001).read_exact(&mut buffer).unwrap();
        let res = test_programs::p3::http::request(
            Method::Post,
            Scheme::Http,
            &addr,
            "/post",
            Some(&buffer.repeat(REPEAT)),
            None,
            None,
            None,
            None,
        )
        .await
        .context("/post large")
        .unwrap();

        println!("/post large: {}", res.status);
        assert_eq!(res.status, 200);
        let method = res.header("x-wasmtime-test-method").unwrap();
        assert_eq!(std::str::from_utf8(method).unwrap(), "POST");
        assert_eq!(res.body.len(), LEN * REPEAT);
        Ok(())
    }
}

fn main() {}
