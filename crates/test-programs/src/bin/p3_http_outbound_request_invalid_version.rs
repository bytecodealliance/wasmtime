use test_programs::p3::wasi::http::types::{ErrorCode, Method, Scheme};

struct Component;

test_programs::p3::export!(Component);

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        let addr = test_programs::p3::wasi::cli::environment::get_environment()
            .into_iter()
            .find_map(|(k, v)| k.eq("HTTP_SERVER").then_some(v))
            .unwrap();
        let res = test_programs::p3::http::request(
            Method::Connect,
            Scheme::Http,
            &addr,
            "/",
            None,
            Some(&[]),
            None,
            None,
            None,
        )
        .await;

        assert!(matches!(
            res.unwrap_err()
                .downcast::<ErrorCode>()
                .expect("expected a wasi-http ErrorCode"),
            ErrorCode::HttpProtocolError,
        ));
        Ok(())
    }
}

fn main() {}
