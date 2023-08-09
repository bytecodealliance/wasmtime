pub mod bindings {
    wit_bindgen::generate!({
        path: "../../wasi/wit",
        world: "wasi:preview/command-extended",
        macro_call_prefix: "::wasi_http_tests::bindings::",
        macro_export,
    });
}

use anyhow::{anyhow, Context, Result};
use std::fmt;

use bindings::wasi::http::{outgoing_handler, types as http_types};
use bindings::wasi::io::streams;
use bindings::wasi::poll::poll;

pub struct Response {
    pub status: http_types::StatusCode,
    pub headers: Vec<(String, Vec<u8>)>,
    pub body: Vec<u8>,
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
    pub fn header(&self, name: &str) -> Option<&Vec<u8>> {
        self.headers
            .iter()
            .find_map(|(k, v)| if k == name { Some(v) } else { None })
    }
}

pub fn request(
    method: http_types::Method,
    scheme: http_types::Scheme,
    authority: &str,
    path_with_query: &str,
    body: Option<&[u8]>,
) -> Result<Response> {
    let headers = http_types::new_fields(&[
        ("User-agent".to_string(), "WASI-HTTP/0.0.1".to_string()),
        ("Content-type".to_string(), "application/json".to_string()),
    ]);

    let request = http_types::new_outgoing_request(
        &method,
        Some(path_with_query),
        Some(&scheme),
        Some(authority),
        headers,
    );

    let request_body = http_types::outgoing_request_write(request)
        .map_err(|_| anyhow!("outgoing request write failed"))?;

    if let Some(body) = body {
        let output_stream_pollable = streams::subscribe_to_output_stream(request_body);

        let mut body_cursor = 0;
        while body_cursor < body.len() {
            let (written, _) = streams::write(request_body, &body[body_cursor..])
                .context("writing request body")?;
            body_cursor += written as usize;
        }

        // TODO: enable when working as expected
        // let _ = poll::poll_oneoff(&[output_stream_pollable]);

        poll::drop_pollable(output_stream_pollable);
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
    let input_stream_pollable = streams::subscribe_to_input_stream(body_stream);

    let mut body = Vec::new();
    let mut eof = streams::StreamStatus::Open;
    while eof != streams::StreamStatus::Ended {
        let (mut body_chunk, stream_status) = streams::read(body_stream, u64::MAX)?;
        eof = if body_chunk.is_empty() {
            streams::StreamStatus::Ended
        } else {
            stream_status
        };
        body.append(&mut body_chunk);
    }

    // TODO: enable when working as expected
    // let _ = poll::poll_oneoff(&[input_stream_pollable]);

    poll::drop_pollable(input_stream_pollable);
    streams::drop_input_stream(body_stream);
    http_types::drop_incoming_response(incoming_response);

    Ok(Response {
        status,
        headers,
        body,
    })
}
