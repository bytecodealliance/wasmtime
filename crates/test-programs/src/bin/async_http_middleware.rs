mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "wasi:http/proxy",
        async: {
            imports: [
                "wasi:http/types@0.3.0-draft#[static]body.finish",
                "wasi:http/handler@0.3.0-draft#handle",
            ],
            exports: [
                "wasi:http/handler@0.3.0-draft#handle",
            ]
        }
    });

    use super::Component;
    export!(Component);
}

use {
    bindings::{
        exports::wasi::http::handler::Guest as Handler,
        wasi::http::{
            handler,
            types::{Body, ErrorCode, Headers, Request, Response},
        },
        wit_future, wit_stream,
    },
    flate2::{
        write::{DeflateDecoder, DeflateEncoder},
        Compression,
    },
    futures::{SinkExt, StreamExt},
    std::{io::Write, mem},
    wit_bindgen_rt::async_support,
};

struct Component;

impl Handler for Component {
    /// Forward the specified request to the imported `wasi:http/handler`, transparently decoding the request body
    /// if it is `deflate`d and then encoding the response body if the client has provided an `accept-encoding:
    /// deflate` header.
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        // First, extract the parts of the request and check for (and remove) headers pertaining to body encodings.
        let method = request.method();
        let scheme = request.scheme();
        let path_with_query = request.path_with_query();
        let authority = request.authority();
        let mut accept_deflated = false;
        let mut content_deflated = false;
        let (headers, body) = Request::into_parts(request);
        let mut headers = headers.entries();
        headers.retain(|(k, v)| match (k.as_str(), v.as_slice()) {
            ("accept-encoding", b"deflate") => {
                accept_deflated = true;
                false
            }
            ("content-encoding", b"deflate") => {
                content_deflated = true;
                false
            }
            _ => true,
        });

        let body = if content_deflated {
            // Next, spawn a task to pipe and decode the original request body and trailers into a new request
            // we'll create below.  This will run concurrently with any code in the imported `wasi:http/handler`.
            let (trailers_tx, trailers_rx) = wit_future::new();
            let (mut pipe_tx, pipe_rx) = wit_stream::new();

            async_support::spawn(async move {
                {
                    let mut body_rx = body.stream().unwrap();

                    let mut decoder = DeflateDecoder::new(Vec::new());

                    while let Some(chunk) = body_rx.next().await {
                        decoder.write_all(&chunk).unwrap();
                        pipe_tx.send(mem::take(decoder.get_mut())).await.unwrap();
                    }

                    pipe_tx.send(decoder.finish().unwrap()).await.unwrap();

                    drop(pipe_tx);
                }

                if let Some(trailers) = Body::finish(body).await.unwrap() {
                    trailers_tx.write(trailers).await;
                }
            });

            Body::new(pipe_rx, Some(trailers_rx))
        } else {
            body
        };

        // While the above task (if any) is running, synthesize a request from the parts collected above and pass
        // it to the imported `wasi:http/handler`.
        let my_request = Request::new(Headers::from_list(&headers).unwrap(), body, None);
        my_request.set_method(&method).unwrap();
        my_request.set_scheme(scheme.as_ref()).unwrap();
        my_request
            .set_path_with_query(path_with_query.as_deref())
            .unwrap();
        my_request.set_authority(authority.as_deref()).unwrap();

        let response = handler::handle(my_request).await?;

        // Now that we have the response, extract the parts, adding an extra header if we'll be encoding the body.
        let status_code = response.status_code();
        let (headers, body) = Response::into_parts(response);
        let mut headers = headers.entries();
        if accept_deflated {
            headers.push(("content-encoding".into(), b"deflate".into()));
        }

        let body = if accept_deflated {
            // Spawn another task; this one is to pipe and encode the original response body and trailers into a
            // new response we'll create below.  This will run concurrently with the caller's code (i.e. it won't
            // necessarily complete before we return a value).
            let (trailers_tx, trailers_rx) = wit_future::new();
            let (mut pipe_tx, pipe_rx) = wit_stream::new();

            async_support::spawn(async move {
                {
                    let mut body_rx = body.stream().unwrap();

                    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::fast());

                    while let Some(chunk) = body_rx.next().await {
                        encoder.write_all(&chunk).unwrap();
                        pipe_tx.send(mem::take(encoder.get_mut())).await.unwrap();
                    }

                    pipe_tx.send(encoder.finish().unwrap()).await.unwrap();

                    drop(pipe_tx);
                }

                if let Some(trailers) = Body::finish(body).await.unwrap() {
                    trailers_tx.write(trailers).await;
                }
            });

            Body::new(pipe_rx, Some(trailers_rx))
        } else {
            body
        };

        // While the above tasks (if any) are running, synthesize a response from the parts collected above and
        // return it.
        let my_response = Response::new(Headers::from_list(&headers).unwrap(), body);
        my_response.set_status_code(status_code).unwrap();

        Ok(my_response)
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
