//! Guest for benchmarking individual `wasi:http` APIs.
//!
//! Usage: `p3_http_api_bench <api> <iters>`, where `<api>` is one of
//! `request.new`, `response.consume-body` or `client.send`.
//!
//! Each call will perform setup steps and then time itself calling the given
//! WASI api multiple times in a loop. The total time in nanoseconds is printed
//! to stdout.

use test_programs::p3::{
    wasi::cli::environment,
    wasi::clocks::monotonic_clock,
    wasi::http::{
        client,
        types::{ErrorCode, Fields, Request, Response, Scheme, Trailers},
    },
    wit_future, wit_stream,
};
use wit_bindgen::{FutureWriter, StreamWriter};

struct Component;

test_programs::p3::export!(Component);

/// Arguments are created, and results dropped, in chunks of this many calls
/// to bound usage of memory and other resources.
const CHUNK: u64 = 10_000;

type BodyWriters = (
    StreamWriter<u8>,
    FutureWriter<Result<Option<Trailers>, ErrorCode>>,
);

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        let args = environment::get_arguments();
        let [_, api, iters] = &args[..] else {
            panic!("usage: p3_http_api_bench <api> <iters>");
        };
        let iters: u64 = iters.parse().unwrap();

        let mut elapsed = 0;
        let mut remaining = iters;
        while remaining > 0 {
            let n = remaining.min(CHUNK);
            remaining -= n;
            let n = usize::try_from(n).unwrap();
            elapsed += match api.as_str() {
                "request.new" => request_new(n),
                "response.consume-body" => response_consume_body(n).await,
                "client.send" => client_send(n).await,
                api => panic!("unknown API `{api}`"),
            };
        }
        println!("{elapsed}");
        Ok(())
    }
}

/// Time `n` calls to `request.new`, passing a body stream as a proxy would.
fn request_new(n: usize) -> u64 {
    // Vec to hold the write ends of futures/streams created until after timing
    // is done. This prevents accidentally including the sending of the default
    // values in the timing.
    let mut writers = Vec::with_capacity(n);
    let args = (0..n)
        .map(|_| {
            let (body_tx, body_rx) = wit_stream::new();
            let (trailers_tx, trailers_rx) = wit_future::new(|| Ok(None));
            writers.push((body_tx, trailers_tx));
            (Fields::new(), body_rx, trailers_rx)
        })
        .collect::<Vec<_>>();
    let mut results = Vec::with_capacity(n);

    let start = monotonic_clock::now();
    for (headers, body, trailers) in args {
        results.push(Request::new(headers, Some(body), trailers, None));
    }
    let elapsed = monotonic_clock::now() - start;

    drop(results);
    drop(writers);
    elapsed
}

/// Time `n` calls to `response.consume-body` on responses from
/// `client.send`, whose bodies are created by the host. This takes the same
/// path in the host as consuming the body of a proxy's incoming request.
async fn response_consume_body(n: usize) -> u64 {
    let (requests, body_writers) = new_outgoing_requests(n);
    // Vec to hold writers so sending their default value isn't timed.
    let mut writers = Vec::with_capacity(n);
    let mut args = Vec::with_capacity(n);
    for request in requests {
        let response = client::send(request).await.unwrap();
        let (result_tx, result_rx) = wit_future::new(|| Ok(()));
        writers.push(result_tx);
        args.push((response, result_rx));
    }
    let mut results = Vec::with_capacity(n);

    let start = monotonic_clock::now();
    for (response, fut) in args {
        results.push(Response::consume_body(response, fut));
    }
    let elapsed = monotonic_clock::now() - start;

    drop(results);
    drop(writers);
    drop(body_writers);
    elapsed
}

/// Time `n` sequential calls to `client.send`.
async fn client_send(n: usize) -> u64 {
    let (args, writers) = new_outgoing_requests(n);
    let mut results = Vec::with_capacity(n);

    let start = monotonic_clock::now();
    for request in args {
        results.push(client::send(request).await.unwrap());
    }
    let elapsed = monotonic_clock::now() - start;

    drop(results);
    drop(writers);
    elapsed
}

/// Create a request with a body stream, as a proxy would, returning it along
/// with the writable ends of its body and trailers.
fn new_request() -> (Request, BodyWriters) {
    let (body_tx, body_rx) = wit_stream::new();
    let (trailers_tx, trailers_rx) = wit_future::new(|| Ok(None));
    let request = Request::new(Fields::new(), Some(body_rx), trailers_rx, None).0;
    (request, (body_tx, trailers_tx))
}

fn new_outgoing_requests(n: usize) -> (Vec<Request>, Vec<BodyWriters>) {
    (0..n)
        .map(|_| {
            let (request, writers) = new_request();
            request.set_path_with_query(Some("/")).unwrap();
            request.set_scheme(Some(&Scheme::Http)).unwrap();
            request
                .set_authority(Some("p3-test-immediate-response"))
                .unwrap();
            (request, writers)
        })
        .unzip()
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
