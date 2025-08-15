use test_programs::p3::wasi::http::types::{ErrorCode, Method, Scheme};

struct Component;

test_programs::p3::export!(Component);

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        let res = test_programs::p3::http::request(
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
        .await;

        let e = res.unwrap_err();
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
