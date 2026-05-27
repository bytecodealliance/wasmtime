/// Test program: verifies that `tx_result` (the future returned by
/// `response.new`) resolves only after the response body has been fully handed
/// to the HTTP layer.
///
/// The handler writes a 64 KiB body as 64 separate 1 KiB chunks and exits via
/// `wasi:cli/exit` after `tx_result` resolves.  Each chunk is written with an
/// explicit `.await`, which yields the CM scheduler between chunks.  The
/// host-side mpsc channel feeding hyper has capacity 1, so after the first
/// chunk is queued a second chunk cannot be sent until hyper drains the
/// channel (a yield the exit task can preempt).
///
/// With incorrect `tx_result` semantics (resolves at request-resource cleanup
/// time, before the body has been handed off to hyper), the exit fires on the
/// first such yield after at most one chunk has been queued, truncating the
/// body and consistently failing the length check in the test.
///
/// With correct semantics (resolves after hyper has consumed all body frames),
/// the exit cannot fire until the store drain is complete, so the full 64 KiB
/// reaches the client.
///
/// Used by `p3_cli_serve_txresult_body_integrity` in `tests/all/cli_tests.rs`.
use test_programs::p3::wasi::cli::exit::exit;
use test_programs::p3::wasi::http::types::{ErrorCode, Fields, Request, Response};
use test_programs::p3::{service, wit_future, wit_stream};

/// Number of 1 KiB chunks that make up the response body.
const CHUNK_COUNT: usize = 64;
/// Size of each chunk in bytes.
const CHUNK_SIZE: usize = 1024;

struct T;

service::export!(T);

impl service::exports::wasi::http::handler::Guest for T {
    async fn handle(_request: Request) -> Result<Response, ErrorCode> {
        let (mut body_tx, body_rx) = wit_stream::new();
        let (trailers_tx, trailers_rx) = wit_future::new(|| Ok(None));
        drop(trailers_tx);

        let (response, tx_result) = Response::new(Fields::new(), Some(body_rx), trailers_rx);

        // Write the body in small, separate chunks.  Each `.await` yields the
        // CM scheduler, giving the exit task a chance to run.  With incorrect
        // tx_result timing the exit fires at the first such yield — after at
        // most one chunk has reached the host-side channel — truncating the
        // body.  With correct timing it cannot fire until all chunks have been
        // consumed by hyper.
        wit_bindgen::spawn(async move {
            for _ in 0..CHUNK_COUNT {
                body_tx.write_all(vec![b'x'; CHUNK_SIZE]).await;
            }
        });

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
