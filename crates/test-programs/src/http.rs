use crate::wasi::http::{outgoing_handler, types as http_types};
use crate::wasi::io::poll;
use crate::wasi::io::streams;
use anyhow::{anyhow, Result};
use std::fmt;

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
    additional_headers: Option<&[(String, Vec<u8>)]>,
) -> Result<Response> {
    fn header_val(v: &str) -> Vec<u8> {
        v.to_string().into_bytes()
    }
    let headers = http_types::Headers::new(
        &[
            &[
                ("User-agent".to_string(), header_val("WASI-HTTP/0.0.1")),
                ("Content-type".to_string(), header_val("application/json")),
            ],
            additional_headers.unwrap_or(&[]),
        ]
        .concat(),
    );

    let request = http_types::OutgoingRequest::new(
        &method,
        Some(path_with_query),
        Some(&scheme),
        Some(authority),
        &headers,
    );

    let outgoing_body = request
        .write()
        .map_err(|_| anyhow!("outgoing request write failed"))?;

    if let Some(mut buf) = body {
        let request_body = outgoing_body
            .write()
            .map_err(|_| anyhow!("outgoing request write failed"))?;

        let pollable = request_body.subscribe();
        while !buf.is_empty() {
            poll::poll_list(&[&pollable]);

            let permit = match request_body.check_write() {
                Ok(n) => n,
                Err(_) => anyhow::bail!("output stream error"),
            };

            let len = buf.len().min(permit as usize);
            let (chunk, rest) = buf.split_at(len);
            buf = rest;

            match request_body.write(chunk) {
                Err(_) => anyhow::bail!("output stream error"),
                _ => {}
            }
        }

        match request_body.flush() {
            Err(_) => anyhow::bail!("output stream error"),
            _ => {}
        }

        poll::poll_list(&[&pollable]);

        match request_body.check_write() {
            Ok(_) => {}
            Err(_) => anyhow::bail!("output stream error"),
        };
    }

    let future_response = outgoing_handler::handle(request, None)?;

    http_types::OutgoingBody::finish(outgoing_body, None);

    let incoming_response = match future_response.get() {
        Some(result) => result.map_err(|_| anyhow!("incoming response errored"))?,
        None => {
            let pollable = future_response.subscribe();
            let _ = poll::poll_list(&[&pollable]);
            future_response
                .get()
                .expect("incoming response available")
                .map_err(|_| anyhow!("incoming response errored"))?
        }
    }
    // TODO: maybe anything that appears in the Result<_, E> position should impl
    // Error? anyway, just use its Debug here:
    .map_err(|e| anyhow!("{e:?}"))?;

    drop(future_response);

    let status = incoming_response.status();

    let headers_handle = incoming_response.headers();
    let headers = headers_handle.entries();
    drop(headers_handle);

    let incoming_body = incoming_response
        .consume()
        .map_err(|()| anyhow!("incoming response has no body stream"))?;

    drop(incoming_response);

    let input_stream = incoming_body.stream().unwrap();
    let input_stream_pollable = input_stream.subscribe();

    let mut body = Vec::new();
    loop {
        poll::poll_list(&[&input_stream_pollable]);

        let mut body_chunk = match input_stream.read(1024 * 1024) {
            Ok(c) => c,
            Err(streams::StreamError::Closed) => break,
            Err(e) => Err(anyhow!("input_stream read failed: {e:?}"))?,
        };

        if !body_chunk.is_empty() {
            body.append(&mut body_chunk);
        }
    }

    Ok(Response {
        status,
        headers,
        body,
    })
}
