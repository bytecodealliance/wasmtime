use test_programs::p3::wasi::http::types::{ErrorCode, Method, Scheme};

struct Component;

test_programs::p3::export!(Component);

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        let res = test_programs::p3::http::request(
            Method::Get,
            Scheme::Other("WS".to_owned()),
            "localhost:3000",
            "/",
            None,
            None,
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
