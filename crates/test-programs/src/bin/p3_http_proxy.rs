use {
    test_programs::p3::{
        service::exports::wasi::http::handler::Guest as Handler,
        wasi::http::{
            client,
            types::{ErrorCode, Fields, Request, Response, Scheme},
        },
        wit_future,
    },
    url::Url,
};

struct Component;

test_programs::p3::service::export!(Component);

impl Handler for Component {
    // Forward the request body and trailers to a URL specified in a header.
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        let headers = request.get_headers().copy_all();
        Ok(
            if let Some(url) = headers.iter().find_map(|(k, v)| {
                (k == "url")
                    .then_some(v)
                    .and_then(|v| std::str::from_utf8(v).ok())
                    .and_then(|v| Url::parse(v).ok())
            }) {
                let method = request.get_method();
                let (rx, trailers) = Request::consume_body(request, wit_future::new(|| Ok(())).1);
                let outgoing_request = Request::new(Fields::new(), Some(rx), trailers, None).0;
                outgoing_request.set_method(&method).unwrap();
                outgoing_request
                    .set_path_with_query(Some(url.path()))
                    .unwrap();
                outgoing_request
                    .set_scheme(Some(&match url.scheme() {
                        "http" => Scheme::Http,
                        "https" => Scheme::Https,
                        scheme => Scheme::Other(scheme.into()),
                    }))
                    .unwrap();
                outgoing_request
                    .set_authority(Some(url.authority()))
                    .unwrap();
                client::send(outgoing_request).await?
            } else {
                bad_request()
            },
        )
    }
}

fn bad_request() -> Response {
    respond(400)
}

fn respond(status: u16) -> Response {
    let response = Response::new(Fields::new(), None, wit_future::new(|| Ok(None)).1).0;
    response.set_status_code(status).unwrap();
    response
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
