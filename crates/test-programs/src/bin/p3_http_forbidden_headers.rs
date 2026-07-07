use test_programs::p3::{
    service::exports::wasi::http::handler::Guest as Handler,
    wasi::http::{
        client,
        types::{ErrorCode, Request, Response, Scheme},
    },
};

struct Component;

test_programs::p3::service::export!(Component);

impl Handler for Component {
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        request.set_scheme(Some(&Scheme::Http)).unwrap();
        request.set_authority(Some("p3-test")).unwrap();
        request.set_path_with_query(Some("/")).unwrap();
        client::send(request).await
    }
}

fn main() {}
