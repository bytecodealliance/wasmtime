use futures::join;
use test_programs::p3::wasi::http::types::{ErrorCode, Headers, Request, Response};
use test_programs::p3::{wit_future, wit_stream};
use wit_bindgen::spawn;

struct T;

test_programs::p3::service::export!(T);

impl test_programs::p3::service::exports::wasi::http::handler::Guest for T {
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        assert!(request.get_scheme().is_some());
        assert!(request.get_authority().is_some());
        assert!(request.get_path_with_query().is_some());

        // TODO: adapt below
        //test_filesystem();

        let header = String::from("custom-forbidden-header");
        let req_hdrs = request.get_headers();

        assert!(
            !req_hdrs.has(&header),
            "forbidden `custom-forbidden-header` found in request"
        );

        assert!(req_hdrs.delete(&header).is_err());
        assert!(req_hdrs.append(&header, b"no".as_ref()).is_err());

        assert!(
            !req_hdrs.has(&header),
            "append of forbidden header succeeded"
        );

        let hdrs = Headers::new();
        let (mut contents_tx, contents_rx) = wit_stream::new();
        let (trailers_tx, trailers_rx) = wit_future::new(|| todo!());
        let (resp, transmit) = Response::new(hdrs, Some(contents_rx), trailers_rx);
        spawn(async {
            join!(
                async {
                    let remaining = contents_tx.write_all(b"hello, world!".to_vec()).await;
                    assert!(remaining.is_empty());
                    drop(contents_tx);
                    trailers_tx
                        .write(Ok(None))
                        .await
                        .expect("failed to write trailers");
                },
                async { transmit.await.unwrap() }
            );
        });
        Ok(resp)
    }
}

// Technically this should not be here for a service, but given the current
// framework for tests it's required since this file is built as a `bin`
fn main() {}

// TODO: adapt below
//fn test_filesystem() {
//    assert!(std::fs::File::open(".").is_err());
//}
