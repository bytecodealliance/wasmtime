//! `first-byte-timeout` bounds the wait for the first byte of the response.
//!
//! The server accepts the connection but delays its response headers well past
//! the timeout, so the request has to fail rather than wait for the headers.
//! `http.wit` documents these timeouts as transport-layer timeouts, separate
//! from any the user may use to bound a request, so the expected error is a
//! read timeout.
use std::time::Duration;

use test_programs::p3::wasi::http::types::{ErrorCode, Method, Scheme};

struct Component;

test_programs::p3::export!(Component);

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        let addr = test_programs::p3::wasi::cli::environment::get_environment()
            .into_iter()
            .find_map(|(k, v)| k.eq("HTTP_SERVER").then_some(v))
            .unwrap();
        let first_byte = Duration::from_millis(500).as_nanos() as u64;

        let res = test_programs::p3::http::request(
            Method::Get,
            Scheme::Http,
            &addr,
            "/first-byte-timeout",
            None,
            None,
            None,             // connect_timeout
            Some(first_byte), // first_byte_timeout
            None,             // between_bytes_timeout
        )
        .await;

        let err = match res {
            Ok(_) => panic!("the delayed response headers should have timed out"),
            Err(err) => err,
        };
        assert!(
            matches!(
                err.downcast_ref::<ErrorCode>(),
                Some(ErrorCode::ConnectionReadTimeout)
            ),
            "expected a connection read timeout: {err:?}"
        );
        Ok(())
    }
}

fn main() {}
