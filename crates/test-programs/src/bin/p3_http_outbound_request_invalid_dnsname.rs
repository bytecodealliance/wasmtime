use test_programs::p3::wasi::http::types::{ErrorCode, Method, Scheme};

struct Component;

test_programs::p3::export!(Component);

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        let (transmit, _response) = test_programs::p3::http::request_with_transmit_result(
            Method::Get,
            Scheme::Http,
            "some.invalid.dnsname:3000",
            "/",
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .expect("failed to construct request");

        let e = transmit.expect_err("expected request transmission to fail");
        assert!(
            matches!(
                e.downcast_ref::<ErrorCode>()
                    .expect("expected a wasi-http ErrorCode"),
                ErrorCode::DnsError(_) | ErrorCode::ConnectionRefused,
            ),
            "Unexpected error: {e:#?}"
        );
        Ok(())
    }
}

fn main() {}
