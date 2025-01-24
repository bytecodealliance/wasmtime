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
        wasi::http::types::{Body, ErrorCode, Request, Response},
        wit_future, wit_stream,
    },
    futures::{SinkExt, StreamExt},
    wit_bindgen_rt::async_support,
};

struct Component;

impl Handler for Component {
    /// Return a response which echoes the request headers, body, and trailers.
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        let (headers, body) = Request::into_parts(request);

        if false {
            // This is the easy and efficient way to do it...
            Ok(Response::new(headers, body))
        } else {
            // ...but we do it the more difficult, less efficient way here to exercise various component model
            // features (e.g. `future`s, `stream`s, and post-return asynchronous execution):
            let (trailers_tx, trailers_rx) = wit_future::new();
            let (mut pipe_tx, pipe_rx) = wit_stream::new();

            async_support::spawn(async move {
                let mut body_rx = body.stream().unwrap();
                while let Some(chunk) = body_rx.next().await {
                    pipe_tx.send(chunk).await.unwrap();
                }

                drop(pipe_tx);

                if let Some(trailers) = Body::finish(body).await.unwrap() {
                    trailers_tx.write(trailers).await;
                }
            });

            Ok(Response::new(
                headers,
                Body::new(pipe_rx, Some(trailers_rx)),
            ))
        }
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
