pub mod bindings {
    wit_bindgen::generate!({
        path: "../../wasi-http/wit",
        world: "wasmtime:wasi/command-extended",
        // macro_call_prefix: "::wasi_http_tests::bindings::",
        // macro_export,
    });
}

use anyhow::{anyhow, Result};
use std::fmt;
use std::sync::OnceLock;

use bindings::wasi::http::{outgoing_handler, types as http_types};
use bindings::wasi::io::poll;
use bindings::wasi::io::streams;

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

pub async fn request(
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
    let headers = http_types::new_fields(
        &[
            &[
                ("User-agent".to_string(), header_val("WASI-HTTP/0.0.1")),
                ("Content-type".to_string(), header_val("application/json")),
            ],
            additional_headers.unwrap_or(&[]),
        ]
        .concat(),
    );

    let request = http_types::new_outgoing_request(
        &method,
        Some(path_with_query),
        Some(&scheme),
        Some(authority),
        headers,
    );

    let outgoing_body = http_types::outgoing_request_write(request)
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

    // TODO: The current implementation requires this drop after the request is sent.
    // The ownership semantics are unclear in wasi-http we should clarify exactly what is
    // supposed to happen here.
    drop(outgoing_body);

    let incoming_response = match http_types::future_incoming_response_get(future_response) {
        Some(result) => result.map_err(|_| anyhow!("incoming response errored"))?,
        None => {
            let pollable = http_types::listen_to_future_incoming_response(future_response);
            let _ = poll::poll_list(&[&pollable]);
            http_types::future_incoming_response_get(future_response)
                .expect("incoming response available")
                .map_err(|_| anyhow!("incoming response errored"))?
        }
    }
    // TODO: maybe anything that appears in the Result<_, E> position should impl
    // Error? anyway, just use its Debug here:
    .map_err(|e| anyhow!("{e:?}"))?;

    http_types::drop_future_incoming_response(future_response);

    let status = http_types::incoming_response_status(incoming_response);

    let headers_handle = http_types::incoming_response_headers(incoming_response);
    let headers = http_types::fields_entries(headers_handle);
    http_types::drop_fields(headers_handle);

    let incoming_body = http_types::incoming_response_consume(incoming_response)
        .map_err(|()| anyhow!("incoming response has no body stream"))?;

    http_types::drop_incoming_response(incoming_response);

    let input_stream = incoming_body.stream().unwrap();
    let input_stream_pollable = input_stream.subscribe();

    let mut body = Vec::new();
    let mut eof = streams::StreamStatus::Open;
    while eof != streams::StreamStatus::Ended {
        poll::poll_list(&[&input_stream_pollable]);

        let (mut body_chunk, stream_status) = input_stream
            .read(1024 * 1024)
            .map_err(|_| anyhow!("input_stream read failed"))?;

        eof = stream_status;

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

static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

pub fn in_tokio<F: std::future::Future>(f: F) -> F::Output {
    match tokio::runtime::Handle::try_current() {
        Ok(h) => {
            let _enter = h.enter();
            h.block_on(f)
        }
        Err(_) => {
            let runtime = RUNTIME.get_or_init(|| {
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap()
            });
            let _enter = runtime.enter();
            runtime.block_on(f)
        }
    }
}
