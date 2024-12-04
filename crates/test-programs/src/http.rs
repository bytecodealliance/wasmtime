use crate::wasi::http::{outgoing_handler, types as http_types};
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
    connect_timeout: Option<u64>,
    first_by_timeout: Option<u64>,
    between_bytes_timeout: Option<u64>,
) -> Result<Response> {
    fn header_val(v: &str) -> Vec<u8> {
        v.to_string().into_bytes()
    }
    let headers = http_types::Headers::from_list(
        &[
            &[
                ("User-agent".to_string(), header_val("WASI-HTTP/0.0.1")),
                ("Content-type".to_string(), header_val("application/json")),
            ],
            additional_headers.unwrap_or(&[]),
        ]
        .concat(),
    )?;

    let request = http_types::OutgoingRequest::new(headers);

    request
        .set_method(&method)
        .map_err(|()| anyhow!("failed to set method"))?;
    request
        .set_scheme(Some(&scheme))
        .map_err(|()| anyhow!("failed to set scheme"))?;
    request
        .set_authority(Some(authority))
        .map_err(|()| anyhow!("failed to set authority"))?;
    request
        .set_path_with_query(Some(&path_with_query))
        .map_err(|()| anyhow!("failed to set path_with_query"))?;

    let outgoing_body = request
        .body()
        .map_err(|_| anyhow!("outgoing request write failed"))?;

    let options = http_types::RequestOptions::new();
    options
        .set_connect_timeout(connect_timeout)
        .map_err(|()| anyhow!("failed to set connect_timeout"))?;
    options
        .set_first_byte_timeout(first_by_timeout)
        .map_err(|()| anyhow!("failed to set first_byte_timeout"))?;
    options
        .set_between_bytes_timeout(between_bytes_timeout)
        .map_err(|()| anyhow!("failed to set between_bytes_timeout"))?;
    let options = Some(options);

    let future_response = outgoing_handler::handle(request, options)?;

    if let Some(mut buf) = body {
        let request_body = outgoing_body
            .write()
            .map_err(|_| anyhow!("outgoing request write failed"))?;

        let pollable = request_body.subscribe();
        while !buf.is_empty() {
            pollable.block();

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

        pollable.block();

        match request_body.check_write() {
            Ok(_) => {}
            Err(_) => anyhow::bail!("output stream error"),
        };
    }
    http_types::OutgoingBody::finish(outgoing_body, None)?;

    let incoming_response = match future_response.get() {
        Some(result) => result.map_err(|()| anyhow!("response already taken"))?,
        None => {
            let pollable = future_response.subscribe();
            pollable.block();
            future_response
                .get()
                .expect("incoming response available")
                .map_err(|()| anyhow!("response already taken"))?
        }
    }?;

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
        input_stream_pollable.block();

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
