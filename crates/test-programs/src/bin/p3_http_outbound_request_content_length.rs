use anyhow::Context as _;
use futures::join;
use test_programs::p3::wasi::http::handler;
use test_programs::p3::wasi::http::types::{ErrorCode, Headers, Method, Request, Scheme, Trailers};
use test_programs::p3::{wit_future, wit_stream};
use wit_bindgen::FutureReader;
use wit_bindgen_rt::async_support::{FutureWriter, StreamWriter};

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
        {
            let (request, mut contents_tx, trailers_tx, transmit) = make_request();
            let (transmit, handle) = join!(async { transmit.await }, async {
                let res = handler::handle(request)
                    .await
                    .context("failed to send request")?;
                println!("writing enough");
                let remaining = contents_tx.write_all(b"long enough".to_vec()).await;
                assert!(
                    remaining.is_empty(),
                    "{}",
                    String::from_utf8_lossy(&remaining)
                );
                drop(contents_tx);
                trailers_tx
                    .write(Ok(None))
                    .await
                    .context("failed to finish body")?;
                anyhow::Ok(res)
            });
            let res = handle.unwrap();
            drop(res);
            transmit.expect("failed to transmit request");
        }

        {
            let (request, mut contents_tx, trailers_tx, transmit) = make_request();
            let (transmit, handle) = join!(async { transmit.await }, async {
                let res = handler::handle(request)
                    .await
                    .context("failed to send request")?;
                println!("writing too little");
                let remaining = contents_tx.write_all(b"msg".to_vec()).await;
                assert!(
                    remaining.is_empty(),
                    "{}",
                    String::from_utf8_lossy(&remaining)
                );
                drop(contents_tx);
                trailers_tx
                    .write(Ok(None))
                    .await
                    .context("failed to finish body")?;
                anyhow::Ok(res)
            });
            let res = handle.unwrap();
            drop(res);
            let err = transmit.expect_err("request transmission should have failed");
            assert!(
                matches!(err, ErrorCode::HttpRequestBodySize(Some(3))),
                "unexpected error: {err:#?}"
            );
        }

        {
            let (request, mut contents_tx, trailers_tx, transmit) = make_request();
            let (transmit, handle) = join!(async { transmit.await }, async {
                let res = handler::handle(request)
                    .await
                    .context("failed to send request")?;
                println!("writing too much");
                let remaining = contents_tx.write_all(b"more than 11 bytes".to_vec()).await;
                assert!(
                    remaining.is_empty(),
                    "{}",
                    String::from_utf8_lossy(&remaining)
                );
                drop(contents_tx);
                _ = trailers_tx.write(Ok(None)).await;
                anyhow::Ok(res)
            });
            let res = handle.unwrap();
            drop(res);
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
