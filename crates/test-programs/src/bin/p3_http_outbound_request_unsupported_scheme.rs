use test_programs::p3::wasi::http::types::{ErrorCode, Method, Scheme};

struct Component;

test_programs::p3::export!(Component);

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        let (transmit, _response) = test_programs::p3::http::request_with_transmit_result(
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
        .await
        .expect("failed to construct request");

        assert!(matches!(
            transmit
                .expect_err("expected request transmission to fail")
                .downcast::<ErrorCode>()
                .expect("expected a wasi-http ErrorCode"),
            ErrorCode::HttpProtocolError,
        ));
        Ok(())
    }
}

fn main() {}
