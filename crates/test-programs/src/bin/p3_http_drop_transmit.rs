use anyhow::Context as _;
use futures::join;
use test_programs::p3::wasi::http::client;
use test_programs::p3::wasi::http::types::{
    Headers, Method, Request, RequestOptions, Response, Scheme,
};
use test_programs::p3::{wit_future, wit_stream};

struct Component;

test_programs::p3::export!(Component);

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        const LEN: usize = 1 << 20;
        let body = vec![1; LEN];

        let addr = test_programs::p3::wasi::cli::environment::get_environment()
            .into_iter()
            .find_map(|(k, v)| k.eq("HTTP_SERVER").then_some(v))
            .unwrap();

        let headers = Headers::from_list(&[]).unwrap();
        let (mut contents_tx, contents_rx) = wit_stream::new();
        let (trailers_tx, trailers_rx) = wit_future::new(|| Ok(None));
        drop(trailers_tx);
        let options = RequestOptions::new();
        let (request, transmit) =
            Request::new(headers, Some(contents_rx), trailers_rx, Some(options));
        request.set_method(&Method::Post).unwrap();
        request.set_scheme(Some(&Scheme::Http)).unwrap();
        request.set_authority(Some(&addr)).unwrap();
        request.set_path_with_query(Some("/post")).unwrap();

        drop(transmit);

        let ((), echoed) = join!(
            async {
                let remaining = contents_tx.write_all((&body[..]).into()).await;
                assert!(remaining.is_empty());
                drop(contents_tx);
            },
            async {
                let response = client::send(request).await.context("send failed").unwrap();
                let status = response.get_status_code();
                assert_eq!(status, 200);
                let (_, result_rx) = wit_future::new(|| Ok(()));
                let (body_rx, _trailers_rx) = Response::consume_body(response, result_rx);
                body_rx.collect().await
            },
        );

        assert_eq!(
            echoed.len(),
            LEN,
            "response body was truncated after dropping the transmit future"
        );
        Ok(())
    }
}

fn main() {}
