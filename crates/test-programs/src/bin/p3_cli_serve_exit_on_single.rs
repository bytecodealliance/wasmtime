/// Test program: handler calls `exit(ok)` after transmitting each response.
///
/// Each request is handled by a fresh instance because:
/// 1. `backpressure_inc` is called at the start of `handle`, preventing the CM
///    runtime from dispatching a second request to the same instance.
/// 2. After the response body is transmitted, a spawned task awaits `tx_result`
///    (the future returned by `response.new`) and then calls `exit(ok)`.
///    `tx_result` resolves once hyper has consumed all response body frames, so
///    the guest only exits after the response has been handed off.
///
/// The response body is the instance's random ID (decimal u64), stable for
/// the lifetime of the instance.  The test verifies that N sequential requests
/// each receive a distinct instance ID, and that no "worker error" appears in
/// stderr (i.e. `exit(ok)` is treated as a clean shutdown, not an error).
///
/// Used by `p3_cli_serve_exit_on_single` in `tests/all/cli_tests.rs`.
use std::sync::OnceLock;
use test_programs::p3::wasi::cli::exit::exit;
use test_programs::p3::wasi::http::types::{ErrorCode, Fields, Request, Response};
use test_programs::p3::wasi::random::random::get_random_u64;
use test_programs::p3::{service, wit_future, wit_stream};

/// Stable identifier for this instance, generated once on first request.
static INSTANCE_ID: OnceLock<u64> = OnceLock::new();

fn instance_id() -> u64 {
    *INSTANCE_ID.get_or_init(get_random_u64)
}

struct T;

service::export!(T);

impl service::exports::wasi::http::handler::Guest for T {
    async fn handle(_request: Request) -> Result<Response, ErrorCode> {
        // Prevent the CM runtime from dispatching another request to this
        // instance while the current one is in flight.
        wit_bindgen::backpressure_inc();

        let body = format!("{}", instance_id()).into_bytes();
        let (mut body_tx, body_rx) = wit_stream::new();
        let (trailers_tx, trailers_rx) = wit_future::new(|| Ok(None));
        drop(trailers_tx);

        let (response, tx_result) = Response::new(Fields::new(), Some(body_rx), trailers_rx);

        wit_bindgen::spawn(async move {
            body_tx.write_all(body).await;
        });

        // Exit cleanly after the response body has been handed off to the
        // HTTP layer.  `tx_result` only resolves once hyper has consumed all
        // response body frames, so the store is not torn down prematurely.
        wit_bindgen::spawn(async move {
            let _ = tx_result.await;
            exit(Ok(()));
        });

        Ok(response)
    }
}

fn main() {
    unreachable!()
}
