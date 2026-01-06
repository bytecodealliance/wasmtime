use futures::join;
use test_programs::p3::wasi::http::client;
use test_programs::p3::wasi::http::types::{ErrorCode, Headers, Method, Request, Scheme, Trailers};
use test_programs::p3::{wit_future, wit_stream};
use wit_bindgen::{FutureReader, FutureWriter, StreamWriter};

struct Component;

test_programs::p3::export!(Component);

fn make_request() -> (
    Request,
    StreamWriter<u8>,
    FutureWriter<Result<Option<Trailers>, ErrorCode>>,
    FutureReader<Result<(), ErrorCode>>,
) {
    let (contents_tx, contents_rx) = wit_stream::new();
    let (trailers_tx, trailers_rx) = wit_future::new(|| todo!());
    let (request, transmit) = Request::new(
        Headers::from_list(&[("Content-Length".to_string(), b"11".to_vec())]).unwrap(),
        Some(contents_rx),
        trailers_rx,
        None,
    );

    request.set_method(&Method::Post).expect("setting method");
    request
        .set_scheme(Some(&Scheme::Http))
        .expect("setting scheme");
    let addr = test_programs::p3::wasi::cli::environment::get_environment()
        .into_iter()
        .find_map(|(k, v)| k.eq("HTTP_SERVER").then_some(v))
        .unwrap();
    request
        .set_authority(Some(&addr))
        .expect("setting authority");
    request
        .set_path_with_query(Some("/"))
        .expect("setting path with query");

    (request, contents_tx, trailers_tx, transmit)
}

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        println!("writing enough");
        {
            let (request, mut contents_tx, trailers_tx, transmit) = make_request();
            let (handle, transmit, ()) = join!(
                async { client::send(request).await },
                async { transmit.await },
                async {
                    let remaining = contents_tx.write_all(b"long enough".to_vec()).await;
                    assert_eq!(String::from_utf8_lossy(&remaining), "");
                    trailers_tx.write(Ok(None)).await.unwrap();
                    drop(contents_tx);
                },
            );
            let _res = handle.expect("failed to send request");
            transmit.expect("failed to transmit request");
        }

        println!("writing too little");
        {
            let (request, mut contents_tx, trailers_tx, transmit) = make_request();
            let (handle, transmit, ()) = join!(
                async { client::send(request).await },
                async { transmit.await },
                async {
                    let remaining = contents_tx.write_all(b"msg".to_vec()).await;
                    assert_eq!(String::from_utf8_lossy(&remaining), "");
                    trailers_tx.write(Ok(None)).await.unwrap();
                    drop(contents_tx);
                },
            );
            // The request body will be polled before `handle` returns.
            // Due to the way implementation is structured, by the time it happens
            // the error will be already available in most cases and `handle` will fail,
            // but it is a race condition, since `handle` may also succeed if
            // polling body returns `Poll::Pending`
            assert!(
                matches!(handle, Ok(..) | Err(ErrorCode::HttpProtocolError)),
                "unexpected handle result: {handle:#?}"
            );
            let err = transmit.expect_err("request transmission should have failed");
            assert!(
                matches!(err, ErrorCode::HttpRequestBodySize(Some(3))),
                "unexpected error: {err:#?}"
            );
        }

        println!("writing too much");
        {
            let (request, mut contents_tx, trailers_tx, transmit) = make_request();
            let (handle, transmit, ()) = join!(
                async { client::send(request).await },
                async { transmit.await },
                async {
                    let remaining = contents_tx.write_all(b"more than 11 bytes".to_vec()).await;
                    assert_eq!(String::from_utf8_lossy(&remaining), "more than 11 bytes");
                    _ = trailers_tx.write(Ok(None)).await;
                },
            );
            // The request body will be polled before `handle` returns.
            // Due to the way implementation is structured, by the time it happens
            // the error will be already available in most cases and `handle` will fail,
            // but it is a race condition, since `handle` may also succeed if
            // polling body returns `Poll::Pending`
            assert!(
                matches!(
                    handle,
                    Ok(..) | Err(ErrorCode::HttpRequestBodySize(Some(18)))
                ),
                "unexpected handle result: {handle:#?}"
            );
            let err = transmit.expect_err("request transmission should have failed");
            assert!(
                matches!(err, ErrorCode::HttpRequestBodySize(Some(18))),
                "unexpected error: {err:#?}"
            );
        }
        Ok(())
    }
}

fn main() {}
