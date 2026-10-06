mod bindings {
    wit_bindgen::generate!({
        inline: "
            package wasmtime:test;
            world outbound-request-pipe {
                include wasi:cli/imports@0.3.0;
                include wasi:http/service@0.3.0;
                export wasi:cli/run@0.3.0;
            }
        ",
        path: "../wasi-http/src/p3/wit",
        default_bindings_module: "bindings",
        generate_all,
    });
}

use bindings::wasi::http::{
    client,
    types::{ErrorCode, Fields, Method, Request, Response, Scheme},
};
use bindings::wit_future;

struct Component;

bindings::export!(Component);

async fn get_body() -> (
    wit_bindgen::StreamReader<u8>,
    wit_bindgen::FutureReader<Result<Option<Fields>, ErrorCode>>,
) {
    let authority = bindings::wasi::cli::environment::get_environment()
        .into_iter()
        .find_map(|(key, value)| (key == "HTTP_SERVER").then_some(value))
        .unwrap();
    let request = Request::new(Fields::new(), None, wit_future::new(|| Ok(None)).1, None).0;
    request.set_method(&Method::Get).unwrap();
    request.set_scheme(Some(&Scheme::Http)).unwrap();
    request.set_authority(Some(&authority)).unwrap();
    request.set_path_with_query(Some("/a")).unwrap();
    let response = client::send(request).await.unwrap();
    Response::consume_body(response, wit_future::new(|| Ok(())).1)
}

impl bindings::exports::wasi::http::handler::Guest for Component {
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        drop(request);
        let (body, trailers) = get_body().await;
        Ok(Response::new(Fields::new(), Some(body), trailers).0)
    }
}

impl bindings::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        let authority = bindings::wasi::cli::environment::get_environment()
            .into_iter()
            .find_map(|(key, value)| (key == "HTTP_SERVER").then_some(value))
            .unwrap();

        let (body, trailers) = get_body().await;

        let request = Request::new(Fields::new(), Some(body), trailers, None).0;
        request.set_method(&Method::Post).unwrap();
        request.set_scheme(Some(&Scheme::Http)).unwrap();
        request.set_authority(Some(&authority)).unwrap();
        request.set_path_with_query(Some("/b")).unwrap();

        // At this point we have given up ownership of all requests and
        // responses, as well as the request transmission future.
        // The host may continue piping the first response body into the second
        // request body, but that should still not leak resources.
        drop(client::send(request).await.unwrap());

        Ok(())
    }
}

fn main() {}
