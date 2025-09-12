use anyhow::{Context as _, Result, anyhow};
use core::fmt;
use futures::join;

use crate::p3::wasi::http::{handler, types};
use crate::p3::{wit_future, wit_stream};

pub struct Response {
    pub status: types::StatusCode,
    pub headers: Vec<(String, Vec<u8>)>,
    pub body: Vec<u8>,
    pub trailers: Option<Vec<(String, Vec<u8>)>>,
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
        out.field("trailers", &self.trailers);
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
    method: types::Method,
    scheme: types::Scheme,
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
    let headers = types::Headers::from_list(
        &[
            &[
                ("User-agent".to_string(), header_val("WASI-HTTP/0.0.1")),
                ("Content-type".to_string(), header_val("application/json")),
            ],
            additional_headers.unwrap_or(&[]),
        ]
        .concat(),
    )?;

    let options = types::RequestOptions::new();
    options
        .set_connect_timeout(connect_timeout)
        .map_err(|_err| anyhow!("failed to set connect_timeout"))?;
    options
        .set_first_byte_timeout(first_by_timeout)
        .map_err(|_err| anyhow!("failed to set first_byte_timeout"))?;
    options
        .set_between_bytes_timeout(between_bytes_timeout)
        .map_err(|_err| anyhow!("failed to set between_bytes_timeout"))?;

    let (mut contents_tx, contents_rx) = wit_stream::new();
    let (trailers_tx, trailers_rx) = wit_future::new(|| Ok(None));
    let (request, transmit) =
        types::Request::new(headers, Some(contents_rx), trailers_rx, Some(options));

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

    let (transmit, handle) = join!(
        async { transmit.await.context("failed to transmit request") },
        async {
            let response = handler::handle(request).await?;
            let status = response.get_status_code();
            let headers = response.get_headers().copy_all();
            let (_, result_rx) = wit_future::new(|| Ok(()));
            let (body_rx, trailers_rx) = types::Response::consume_body(response, result_rx);
            let ((), rx) = join!(
                async {
                    if let Some(buf) = body {
                        let remaining = contents_tx.write_all(buf.into()).await;
                        assert!(remaining.is_empty());
                    }
                    drop(contents_tx);
                    // This can fail in HTTP/1.1, since the connection might already be closed
                    _ = trailers_tx.write(Ok(None)).await;
                },
                async {
                    let body = body_rx.collect().await;
                    let trailers = trailers_rx.await.context("failed to read body")?;
                    let trailers = trailers.map(|trailers| trailers.copy_all());
                    anyhow::Ok(Response {
                        status,
                        headers,
                        body,
                        trailers,
                    })
                }
            );
            rx
        },
    );
    let response = handle?;
    transmit?;
    Ok(response)
}
