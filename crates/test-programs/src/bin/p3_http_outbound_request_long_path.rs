use test_programs::p3::wasi::http::types::{Method, Scheme};

struct Component;

test_programs::p3::export!(Component);

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        let addr = test_programs::p3::wasi::cli::environment::get_environment()
            .into_iter()
            .find_map(|(k, v)| k.eq("HTTP_SERVER").then_some(v))
            .unwrap();
        // A path of at least 65535 bytes before the `?`, which exceeds the maximum
        // length supported by `http::uri::PathAndQuery`.
        let path = format!("/{}?some=arg", "é".repeat(u16::MAX as usize + 10));
        let res = test_programs::p3::http::request(
            Method::Get,
            Scheme::Http,
            &addr,
            &path,
            None,
            None,
            None,
            None,
            None,
        )
        .await;
        println!("{addr} long path: {res:?}");
        Ok(())
    }
}

fn main() {}
