use anyhow::{anyhow, Result};
use wasi_http_tests::*;

fn request(
    method: types::MethodParam<'_>,
    scheme: types::SchemeParam<'_>,
    authority: &str,
    path: &str,
    query: &str,
    body: &[u8],
) -> Result<()> {
    let headers = types::new_fields(&[
        ("User-agent", "WASI-HTTP/0.0.1"),
        ("Content-type", "application/json"),
    ]);

    let request =
        types::new_outgoing_request(method, path, query, Some(scheme), authority, headers);

    let request_stream = types::outgoing_request_write(request)
        .map_err(|_| anyhow!("outgoing request write failed"))?;

    let mut body_cursor = 0;
    while body_cursor < body.len() {
        let written = streams::write(request_stream, &body[body_cursor..])?;
        body_cursor += written as usize;
    }

    default_outgoing_http::handle(request, None);

    todo!()
}

fn main() {}
