use anyhow::{anyhow, Context, Result};
use std::fmt;
use wasi_http_tests::*;

struct Response {
    status: wasi::http::types::StatusCode,
    headers: Vec<(String, Vec<u8>)>,
    body: Vec<u8>,
}
impl fmt::Debug for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = f.debug_struct("Response");
        out.field("status", &self.status)
            .field("headers", &self.headers);
        if let Ok(body) = std::str::from_utf8(&self.body) {
            out.field("body", &body);
        } else {
            out.field("body", &self.body);
        }
        out.finish()
    }
}

impl Response {
    fn header(&self, name: &str) -> Option<&Vec<u8>> {
        self.headers
            .iter()
            .find_map(|(k, v)| if k == name { Some(v) } else { None })
    }
}

fn request(
    method: wasi::http::types::Method,
    scheme: wasi::http::types::Scheme,
    authority: &str,
    path_with_query: &str,
    body: &[u8],
) -> Result<Response> {
    let headers = wasi::http::types::new_fields(&[
        ("User-agent".to_string(), "WASI-HTTP/0.0.1".to_string()),
        ("Content-type".to_string(), "application/json".to_string()),
    ]);

    let request = wasi::http::types::new_outgoing_request(
        &method,
        Some(&path_with_query),
        Some(&scheme),
        Some(&authority),
        headers,
    );

    let request_body = wasi::http::types::outgoing_request_write(request)
        .map_err(|_| anyhow!("outgoing request write failed"))?;

    let mut body_cursor = 0;
    while body_cursor < body.len() {
        let written = wasi::io::streams::write(request_body, &body[body_cursor..])
            .context("writing request body")?;
        body_cursor += written as usize;
    }

    let future_response = wasi::http::outgoing_handler::handle(request, None);

    let incoming_response = wasi::http::types::future_incoming_response_get(future_response)
        .ok_or_else(|| anyhow!("incoming response is available immediately"))?
        // TODO: maybe anything that appears in the Result<_, E> position should impl
        // Error? anyway, just use its Debug here:
        .map_err(|e| anyhow!("{e:?}"))?;

    // TODO: The current implementation requires this drop after the request is sent.
    // The ownership semantics are unclear in wasi-http we should clarify exactly what is
    // supposed to happen here.
    wasi::io::streams::drop_output_stream(request_body);

    // TODO: we could create a pollable from the future_response and poll on it here to test that
    // its available immediately

    wasi::http::types::drop_outgoing_request(request);

    wasi::http::types::drop_future_incoming_response(future_response);

    let status = wasi::http::types::incoming_response_status(incoming_response);

    let headers_handle = wasi::http::types::incoming_response_headers(incoming_response);
    let headers = wasi::http::types::fields_entries(headers_handle);
    wasi::http::types::drop_fields(headers_handle);

    let body_stream = wasi::http::types::incoming_response_consume(incoming_response)
        .map_err(|()| anyhow!("incoming response has no body stream"))?;

    let mut body = Vec::new();
    let mut eof = false;
    while !eof {
        let (mut body_chunk, stream_ended) = wasi::io::streams::read(body_stream, u64::MAX)?;
        eof = stream_ended;
        body.append(&mut body_chunk);
    }
    wasi::io::streams::drop_input_stream(body_stream);
    wasi::http::types::drop_incoming_response(incoming_response);

    Ok(Response {
        status,
        headers,
        body,
    })
}

fn main() -> Result<()> {
    let r1 = request(
        wasi::http::types::Method::Get,
        wasi::http::types::Scheme::Http,
        "localhost:3000",
        "/get?some=arg?goes=here",
        &[],
    )
    .context("localhost:3000 /get")?;

    println!("localhost:3000 /get: {r1:?}");
    assert_eq!(r1.status, 200);
    let method = r1.header("x-wasmtime-test-method").unwrap();
    assert_eq!(std::str::from_utf8(method).unwrap(), "GET");
    let uri = r1.header("x-wasmtime-test-uri").unwrap();
    assert_eq!(
        std::str::from_utf8(uri).unwrap(),
        "http://localhost:3000/get?some=arg?goes=here"
    );
    assert_eq!(r1.body, b"");

    let r2 = request(
        wasi::http::types::Method::Post,
        wasi::http::types::Scheme::Http,
        "localhost:3000",
        "/post",
        b"{\"foo\": \"bar\"}",
    )
    .context("localhost:3000 /post")?;

    println!("localhost:3000 /post: {r2:?}");
    assert_eq!(r2.status, 200);
    let method = r2.header("x-wasmtime-test-method").unwrap();
    assert_eq!(std::str::from_utf8(method).unwrap(), "POST");
    assert_eq!(r2.body, b"{\"foo\": \"bar\"}");

    let r3 = request(
        wasi::http::types::Method::Put,
        wasi::http::types::Scheme::Http,
        "localhost:3000",
        "/put",
        &[],
    )
    .context("localhost:3000 /put")?;

    println!("localhost:3000 /put: {r3:?}");
    assert_eq!(r3.status, 200);
    let method = r3.header("x-wasmtime-test-method").unwrap();
    assert_eq!(std::str::from_utf8(method).unwrap(), "PUT");
    assert_eq!(r3.body, b"");

    let r4 = request(
        wasi::http::types::Method::Other("OTHER".to_owned()),
        wasi::http::types::Scheme::Http,
        "localhost:3000",
        "/",
        &[],
    );

    let error = r4.unwrap_err();
    assert_eq!(
        error.to_string(),
        "Error::UnexpectedError(\"unknown method OTHER\")"
    );

    let r5 = request(
        wasi::http::types::Method::Get,
        wasi::http::types::Scheme::Other("WS".to_owned()),
        "localhost:3000",
        "/",
        &[],
    );

    let error = r5.unwrap_err();
    assert_eq!(
        error.to_string(),
        "Error::UnexpectedError(\"unsupported scheme WS\")"
    );

    Ok(())
}
