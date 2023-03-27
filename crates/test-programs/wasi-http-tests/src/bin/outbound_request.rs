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

fn request(
    method: types::MethodParam<'_>,
    scheme: types::SchemeParam<'_>,
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
        types::new_outgoing_request(method, path, query, Some(scheme), authority, headers);

    let request_body = types::outgoing_request_write(request)
        .map_err(|_| anyhow!("outgoing request write failed"))?;

    let mut body_cursor = 0;
    while body_cursor < body.len() {
        let written =
            streams::write(request_body, &body[body_cursor..]).context("writing request body")?;
        body_cursor += written as usize;
    }

    streams::drop_output_stream(request_body);

    let future_response = default_outgoing_http::handle(request, None);
    // TODO: we could create a pollable from the future_response and poll on it here to test that
    // its available immediately

    types::drop_outgoing_request(request);

    let incoming_response = types::future_incoming_response_get(future_response)
        .ok_or_else(|| anyhow!("incoming response is available immediately"))?
        // TODO: maybe anything that appears in the Result<_, E> position should impl
        // Error? anyway, just use its Debug here:
        .map_err(|e| anyhow!("incoming response error: {e:?}"))?;

    types::drop_future_incoming_response(future_response);

    let status = types::incoming_response_status(incoming_response);

    let headers_handle = types::incoming_response_headers(incoming_response);
    let headers = types::fields_entries(headers_handle);
    types::drop_fields(headers_handle);

    let body_stream = types::incoming_response_consume(incoming_response)
        .map_err(|()| anyhow!("incoming response has no body stream"))?;
    types::drop_incoming_response(incoming_response);

    let mut body = Vec::new();
    let mut eof = false;
    while !eof {
        let (mut body_chunk, stream_ended) = streams::read(body_stream, u64::MAX)?;
        eof = stream_ended;
        body.append(&mut body_chunk);
    }
    streams::drop_input_stream(body_stream);

    Ok(Response {
        status,
        headers,
        body,
    })
}

fn main() -> Result<()> {
    let r1 = request(
        types::MethodParam::Get,
        types::SchemeParam::Https,
        "postman-echo.com",
        "/get",
        "?some=arg?goes=here",
        &[],
    )
    .context("postman-echo /get")?;

    println!("postman-echo /get: {r1:?}");
    assert_eq!(r1.status, 200);

    let r2 = request(
        types::MethodParam::Post,
        types::SchemeParam::Https,
        "postman-echo.com",
        "/post",
        "",
        b"{\"foo\": \"bar\"}",
    )
    .context("postman-echo /post")?;

    println!("postman-echo /post: {r2:?}");
    assert_eq!(r2.status, 200);

    let r3 = request(
        types::MethodParam::Put,
        types::SchemeParam::Https,
        "postman-echo.com",
        "/put",
        "",
        &[],
    )
    .context("postman-echo /put")?;

    println!("postman-echo /put: {r3:?}");
    assert_eq!(r3.status, 200);
    Ok(())
}
