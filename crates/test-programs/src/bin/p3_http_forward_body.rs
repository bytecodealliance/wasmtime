use test_programs::p3::{
    service::exports::wasi::http::handler::Guest as Handler,
    wasi::http::types::{ErrorCode, Fields, Request, Response},
    wit_future,
};

struct Component;

test_programs::p3::service::export!(Component);

impl Handler for Component {
    /// Forward the request body as the response body, optionally reading from
    /// it or replacing its trailers first, depending on the `x-mode` header.
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        let mode = request
            .get_headers()
            .get("x-mode")
            .pop()
            .unwrap_or_default();
        let (mut body, mut trailers) = Request::consume_body(request, wit_future::new(|| Ok(())).1);
        match &mode[..] {
            b"read-0" => _ = body.read(Vec::new()).await,
            b"read-1" => _ = body.read(Vec::with_capacity(1)).await,
            b"trailers-error" => {
                trailers = wit_future::new(|| Err(ErrorCode::InternalError(None))).1;
            }
            _ => {}
        }
        Ok(Response::new(Fields::new(), Some(body), trailers).0)
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
