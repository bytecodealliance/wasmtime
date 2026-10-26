//! `between-bytes-timeout` bounds the wait for each chunk of the body: the server
//! sends one chunk then stalls, so reading the second one must fail.
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
        let between_bytes = Duration::from_millis(100).as_nanos() as u64;

        let res = test_programs::p3::http::request(
            Method::Get,
            Scheme::Http,
            &addr,
            "/between-bytes-timeout",
            None,
            None,
            None,                // connect_timeout
            None,                // first_byte_timeout
            Some(between_bytes), // between_bytes_timeout
        )
        .await;

        let err = match res {
            Ok(_) => panic!("the stalled second chunk should have timed out"),
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
