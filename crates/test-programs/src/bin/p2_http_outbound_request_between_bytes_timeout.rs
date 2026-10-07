//! `between-bytes-timeout` bounds the wait for each subsequent chunk of the body.
//!
//! The server sends the response headers plus one chunk, then stalls well past the
//! timeout, so reading the second chunk has to fail rather than wait for it.
//! `http.wit` documents these timeouts as transport-layer timeouts, so the expected
//! failure is a read timeout — and `io.wit` warns that the debug string of an error
//! "should not be consumed mechanically", so this only asserts the error's kind.
use anyhow::{Result, anyhow};
use test_programs::wasi::http::{outgoing_handler, types as http_types};
use test_programs::wasi::io::streams;

fn main() -> Result<()> {
    let addr = std::env::var("HTTP_SERVER").unwrap();

    let headers =
        http_types::Headers::from_list(&[("User-agent".to_string(), b"WASI-HTTP/0.0.1".to_vec())])?;
    let request = http_types::OutgoingRequest::new(headers);
    request
        .set_method(&http_types::Method::Get)
        .map_err(|()| anyhow!("set_method"))?;
    request
        .set_scheme(Some(&http_types::Scheme::Http))
        .map_err(|()| anyhow!("set_scheme"))?;
    request
        .set_authority(Some(&addr))
        .map_err(|()| anyhow!("set_authority"))?;
    request
        .set_path_with_query(Some("/between-bytes-timeout"))
        .map_err(|()| anyhow!("set_path"))?;

    let outgoing_body = request.body().map_err(|_| anyhow!("request.body"))?;

    let options = http_types::RequestOptions::new();
    options
        .set_between_bytes_timeout(Some(500_000_000)) // 500ms
        .map_err(|()| anyhow!("set_between_bytes_timeout"))?;

    let future_response = outgoing_handler::handle(request, Some(options))?;
    http_types::OutgoingBody::finish(outgoing_body, None)?;

    let incoming_response = match future_response.get() {
        Some(r) => r.map_err(|()| anyhow!("taken"))??,
        None => {
            future_response.subscribe().block();
            future_response
                .get()
                .expect("available")
                .map_err(|()| anyhow!("taken"))??
        }
    };

    let incoming_body = incoming_response
        .consume()
        .map_err(|()| anyhow!("consume"))?;
    let stream = incoming_body.stream().map_err(|()| anyhow!("stream"))?;
    let pollable = stream.subscribe();

    // The first chunk has already been sent by the server, so it must arrive.
    pollable.block();
    let first = stream
        .read(1024)
        .map_err(|e| anyhow!("reading the first chunk failed: {e:?}"))?;
    assert!(
        !first.is_empty(),
        "the first chunk should have arrived before the stall"
    );

    // The second chunk is stalled well past the timeout, so this read must fail.
    pollable.block();
    let err = stream
        .read(1024)
        .expect_err("reading the stalled second chunk should have timed out");
    assert!(
        matches!(err, streams::StreamError::LastOperationFailed(_)),
        "expected the read to fail with a last-operation failure: {err:?}"
    );

    Ok(())
}
