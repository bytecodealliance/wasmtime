use anyhow::{anyhow, Context, Result};
use std::fmt;
use wasi_http_tests::*;

struct Response {
    status: types::StatusCode,
    headers: Vec<(String, String)>,
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
    fn header(&self, name: &str) -> Option<&String> {
        self.headers
            .iter()
            .find_map(|(k, v)| if k == name { Some(v) } else { None })
    }
}

fn request(
    method: types::Method,
    scheme: types::Scheme,
    authority: &str,
    path: &str,
    query: &str,
    body: &[u8],
) -> Result<Response> {
    let headers = types::new_fields(&[
        ("User-agent", "WASI-HTTP/0.0.1"),
        ("Content-type", "application/json"),
    ]);

    let request =
        types::new_outgoing_request(&method, path, query, Some(&scheme), authority, headers);

    let request_body = types::outgoing_request_write(request)
        .map_err(|_| anyhow!("outgoing request write failed"))?;

    let mut body_cursor = 0;
    while body_cursor < body.len() {
        let written =
            streams::write(request_body, &body[body_cursor..]).context("writing request body")?;
        body_cursor += written as usize;
    }

    let future_response = default_outgoing_http::handle(request, None);

    let incoming_response = types::future_incoming_response_get(future_response)
        .ok_or_else(|| anyhow!("incoming response is available immediately"))?
        // TODO: maybe anything that appears in the Result<_, E> position should impl
        // Error? anyway, just use its Debug here:
        .map_err(|e| anyhow!("{e:?}"))?;

    // TODO: The current implementation requires this drop after the request is sent.
    // The ownership semantics are unclear in wasi-http we should clarify exactly what is
    // supposed to happen here.
    streams::drop_output_stream(request_body);

    // TODO: we could create a pollable from the future_response and poll on it here to test that
    // its available immediately

    types::drop_outgoing_request(request);

    types::drop_future_incoming_response(future_response);

    let status = types::incoming_response_status(incoming_response);

    let headers_handle = types::incoming_response_headers(incoming_response);
    let headers = types::fields_entries(headers_handle);
    types::drop_fields(headers_handle);

    let body_stream = types::incoming_response_consume(incoming_response)
        .map_err(|()| anyhow!("incoming response has no body stream"))?;

    let mut body = Vec::new();
    let mut eof = false;
    while !eof {
        let (mut body_chunk, stream_ended) = streams::read(body_stream, u64::MAX)?;
        eof = stream_ended;
        body.append(&mut body_chunk);
    }
    streams::drop_input_stream(body_stream);
    types::drop_incoming_response(incoming_response);

    Ok(Response {
        status,
        headers,
        body,
    })
}

fn main() -> Result<()> {
    let r1 = request(
        types::Method::Get,
        types::Scheme::Http,
        "localhost:3000",
        "/get",
        "?some=arg?goes=here",
        &[],
    )
    .context("localhost:3000 /get")?;

    println!("localhost:3000 /get: {r1:?}");
    assert_eq!(r1.status, 200);
    let method = r1.header("x-wasmtime-test-method").unwrap();
    assert_eq!(method, "GET");
    let uri = r1.header("x-wasmtime-test-uri").unwrap();
    assert_eq!(uri, "http://localhost:3000/get?some=arg?goes=here");
    assert_eq!(r1.body, b"");

    let r2 = request(
        types::Method::Post,
        types::Scheme::Http,
        "localhost:3000",
        "/post",
        "",
        b"{\"foo\": \"bar\"}",
    )
    .context("localhost:3000 /post")?;

    println!("localhost:3000 /post: {r2:?}");
    assert_eq!(r2.status, 200);
    let method = r2.header("x-wasmtime-test-method").unwrap();
    assert_eq!(method, "POST");
    assert_eq!(r2.body, b"{\"foo\": \"bar\"}");

    let r3 = request(
        types::Method::Put,
        types::Scheme::Http,
        "localhost:3000",
        "/put",
        "",
        &[],
    )
    .context("localhost:3000 /put")?;

    println!("localhost:3000 /put: {r3:?}");
    assert_eq!(r3.status, 200);
    let method = r3.header("x-wasmtime-test-method").unwrap();
    assert_eq!(method, "PUT");
    assert_eq!(r3.body, b"");

    let r4 = request(
        types::Method::Other("OTHER".to_owned()),
        types::Scheme::Http,
        "localhost:3000",
        "/",
        "",
        &[],
    );

    let error = r4.unwrap_err();
    assert_eq!(
        error.to_string(),
        "Error::UnexpectedError(\"unknown method OTHER\")"
    );

    let r5 = request(
        types::Method::Get,
        types::Scheme::Other("WS".to_owned()),
        "localhost:3000",
        "/",
        "",
        &[],
    );

    let error = r5.unwrap_err();
    assert_eq!(
        error.to_string(),
        "Error::UnexpectedError(\"unsupported scheme WS\")"
    );

    Ok(())
}
