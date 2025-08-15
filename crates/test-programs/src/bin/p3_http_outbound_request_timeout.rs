use anyhow::Context;
use std::net::SocketAddr;
use std::time::Duration;
use test_programs::p3::wasi::http::types::{ErrorCode, Method, Scheme};

struct Component;

test_programs::p3::export!(Component);

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        // This address inside the TEST-NET-3 address block is expected to time out.
        let addr = SocketAddr::from(([203, 0, 113, 12], 80)).to_string();
        let timeout = Duration::from_millis(200);
        let connect_timeout: Option<u64> = Some(timeout.as_nanos() as u64);
        let res = test_programs::p3::http::request(
            Method::Get,
            Scheme::Http,
            &addr,
            "/get?some=arg&goes=here",
            None,
            None,
            connect_timeout,
            None,
            None,
        )
        .await
        .context("/get");

        assert!(res.is_err());
        let err = res.unwrap_err();
        assert!(
            matches!(
                err.downcast_ref::<ErrorCode>(),
                Some(ErrorCode::ConnectionTimeout | ErrorCode::ConnectionRefused)
            ),
            "expected connection timeout: {err:?}"
        );
        Ok(())
    }
}

fn main() {}
