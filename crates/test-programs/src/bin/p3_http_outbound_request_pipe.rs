use test_programs::p3::service;
use test_programs::p3::wasi::cli::environment;
use test_programs::p3::wasi::http::{
    client,
    types::{ErrorCode, Fields, Method, Request, Response, Scheme},
};
use test_programs::p3::wit_future;

struct Component;

test_programs::p3::export!(Component);
service::export!(Component);

///! This component can be run as an HTTP server or a cli. In either case it
///! will first make a query to `/a`. When run as a server the body of that
///! response will be spliced into the returned response. When run as a CLI the
///! body will be spliced into a new request to `/b`. In both cases, the function
///! will then return. This test the behavior of the host when response bodies
///! have been spliced into new bodies and the Instance or Store is then dropped.

async fn get_body() -> (
    wit_bindgen::StreamReader<u8>,
    wit_bindgen::FutureReader<Result<Option<Fields>, ErrorCode>>,
) {
    let authority = environment::get_environment()
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

impl service::exports::wasi::http::handler::Guest for Component {
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        drop(request);
        let (body, trailers) = get_body().await;
        // After returning we have given up ownership of all requests and
        // responses, as well as the request transmission future.
        // The host may continue piping the first response body into the second
        // request body, but that should still not leak resources.
        Ok(Response::new(Fields::new(), Some(body), trailers).0)
    }
}

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        let authority = environment::get_environment()
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
