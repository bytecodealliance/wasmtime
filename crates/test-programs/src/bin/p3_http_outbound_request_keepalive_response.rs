use futures::join;
use test_programs::p3::wasi::http::client;
use test_programs::p3::wasi::http::types::{Headers, Method, Request, Scheme};
use test_programs::p3::{wit_future, wit_stream};

struct Component;

test_programs::p3::export!(Component);

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        let addr = test_programs::p3::wasi::cli::environment::get_environment()
            .into_iter()
            .find_map(|(key, value)| (key == "HTTP_SERVER").then_some(value))
            .unwrap();

        let (contents_tx, contents_rx) = wit_stream::new();
        let (trailers_tx, trailers_rx) = wit_future::new(|| Ok(None));
        let (request, transmitted) =
            Request::new(Headers::new(), Some(contents_rx), trailers_rx, None);
        request.set_method(&Method::Get).unwrap();
        request.set_scheme(Some(&Scheme::Http)).unwrap();
        request.set_authority(Some(&addr)).unwrap();
        request.set_path_with_query(Some("/")).unwrap();
        drop(contents_tx);

        let ((), response) = join!(
            async {
                trailers_tx.write(Ok(None)).await.unwrap();
            },
            client::send(request),
        );
        let response = response.unwrap();

        // The response body never yields a frame, keeping the HTTP connection
        // open. Request transmission must resolve independently of that body.
        transmitted.await.unwrap();
        drop(response);
        Ok(())
    }
}

fn main() {}
