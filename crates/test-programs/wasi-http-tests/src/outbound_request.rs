use anyhow::{anyhow, Context, Result};
use std::fmt;

use crate::wasi::http::{outgoing_handler, types as http_types};
use crate::wasi::io::streams;

struct Response {
    status: http_types::StatusCode,
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
    method: http_types::Method,
    scheme: http_types::Scheme,
    authority: &str,
    path_with_query: &str,
    body: Option<&[u8]>,
) -> Result<Response> {
    let headers = crate::wasi::http::types::new_fields(&[
        ("User-agent".to_string(), "WASI-HTTP/0.0.1".to_string()),
        ("Content-type".to_string(), "application/json".to_string()),
    ]);

    let request = http_types::new_outgoing_request(
        &method,
        Some(&path_with_query),
        Some(&scheme),
        Some(&authority),
        headers,
    );

    let request_body = http_types::outgoing_request_write(request)
        .map_err(|_| anyhow!("outgoing request write failed"))?;

    if let Some(body) = body {
        let mut body_cursor = 0;
        while body_cursor < body.len() {
            let (written, _) = streams::write(request_body, &body[body_cursor..])
                .context("writing request body")?;
            body_cursor += written as usize;
        }
    }

    let future_response = outgoing_handler::handle(request, None);

    let incoming_response = http_types::future_incoming_response_get(future_response)
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

    http_types::drop_outgoing_request(request);

    http_types::drop_future_incoming_response(future_response);

    let status = http_types::incoming_response_status(incoming_response);

    let headers_handle = http_types::incoming_response_headers(incoming_response);
    let headers = http_types::fields_entries(headers_handle);
    http_types::drop_fields(headers_handle);

    let body_stream = http_types::incoming_response_consume(incoming_response)
        .map_err(|()| anyhow!("incoming response has no body stream"))?;

    let mut body = Vec::new();
    let mut eof = streams::StreamStatus::Open;
    while eof != streams::StreamStatus::Ended {
        let (mut body_chunk, stream_status) = streams::read(body_stream, u64::MAX)?;
        eof = if body_chunk.len() == 0 {
            streams::StreamStatus::Ended
        } else {
            stream_status
        };
        body.append(&mut body_chunk);
    }

    streams::drop_input_stream(body_stream);
    http_types::drop_incoming_response(incoming_response);

    Ok(Response {
        status,
        headers,
        body,
    })
}

pub fn main() -> Result<()> {
    let r1 = request(
        http_types::Method::Get,
        http_types::Scheme::Http,
        "localhost:3000",
        "/get?some=arg&goes=here",
        None,
    )
    .context("localhost:3000 /get")?;

    println!("localhost:3000 /get: {r1:?}");
    assert_eq!(r1.status, 200);
    let method = r1.header("x-wasmtime-test-method").unwrap();
    assert_eq!(std::str::from_utf8(method).unwrap(), "GET");
    let uri = r1.header("x-wasmtime-test-uri").unwrap();
    assert_eq!(
        std::str::from_utf8(uri).unwrap(),
        "http://localhost:3000/get?some=arg&goes=here"
    );
    assert_eq!(r1.body, b"");

    let r2 = request(
        http_types::Method::Post,
        http_types::Scheme::Http,
        "localhost:3000",
        "/post",
        Some(b"{\"foo\": \"bar\"}"),
    )
    .context("localhost:3000 /post")?;

    println!("localhost:3000 /post: {r2:?}");
    assert_eq!(r2.status, 200);
    let method = r2.header("x-wasmtime-test-method").unwrap();
    assert_eq!(std::str::from_utf8(method).unwrap(), "POST");
    assert_eq!(r2.body, b"{\"foo\": \"bar\"}");

    let r3 = request(
        http_types::Method::Put,
        http_types::Scheme::Http,
        "localhost:3000",
        "/put",
        Some(&[]),
    )
    .context("localhost:3000 /put")?;

    println!("localhost:3000 /put: {r3:?}");
    assert_eq!(r3.status, 200);
    let method = r3.header("x-wasmtime-test-method").unwrap();
    assert_eq!(std::str::from_utf8(method).unwrap(), "PUT");
    assert_eq!(r3.body, b"");

    let r4 = request(
        http_types::Method::Other("OTHER".to_owned()),
        http_types::Scheme::Http,
        "localhost:3000",
        "/",
        None,
    );

    let error = r4.unwrap_err();
    assert_eq!(
        error.to_string(),
        "Error::UnexpectedError(\"unknown method OTHER\")"
    );

    let r5 = request(
        http_types::Method::Get,
        http_types::Scheme::Other("WS".to_owned()),
        "localhost:3000",
        "/",
        None,
    );

    let error = r5.unwrap_err();
    assert_eq!(
        error.to_string(),
        "Error::UnexpectedError(\"unsupported scheme WS\")"
    );

    Ok(())
}
