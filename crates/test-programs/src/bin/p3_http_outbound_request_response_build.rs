use test_programs::p3::wasi::http::types::{Fields, Headers, Method, Request, Response, Scheme};
use test_programs::p3::{wit_future, wit_stream};

struct Component;

test_programs::p3::export!(Component);

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        println!("Called _start");
        {
            let headers = Headers::from_list(&[(
                "Content-Type".to_string(),
                "application/json".to_string().into_bytes(),
            )])
            .unwrap();
            let (mut contents_tx, contents_rx) = wit_stream::new();
            let (_, trailers_rx) = wit_future::new(|| Ok(None));
            let (request, _) = Request::new(headers, Some(contents_rx), trailers_rx, None);

            request.set_method(&Method::Get).expect("setting method");
            request
                .set_scheme(Some(&Scheme::Https))
                .expect("setting scheme");
            request
                .set_authority(Some("www.example.com"))
                .expect("setting authority");
            let (remaining, ()) =
                futures::join!(contents_tx.write_all(b"request-body".to_vec()), async {
                    drop(request);
                },);
            assert!(!remaining.is_empty());
        }
        {
            let headers = Headers::from_list(&[(
                "Content-Type".to_string(),
                "application/text".to_string().into_bytes(),
            )])
            .unwrap();
            let (mut contents_tx, contents_rx) = wit_stream::new();
            let (_, trailers_rx) = wit_future::new(|| Ok(None));
            let _ = Response::new(headers, Some(contents_rx), trailers_rx);
            let remaining = contents_tx.write_all(b"response-body".to_vec()).await;
            assert!(!remaining.is_empty());
        }

        {
            let (_, trailers_rx) = wit_future::new(|| Ok(None));
            let (req, _) = Request::new(Fields::new(), None, trailers_rx, None);

            assert!(
                req.set_method(&Method::Other("invalid method".to_string()))
                    .is_err()
            );

            assert!(req.set_authority(Some("bad-port:99999")).is_err());
            assert!(req.set_authority(Some("bad-\nhost")).is_err());
            assert!(req.set_authority(Some("too-many-ports:80:80:80")).is_err());

            assert!(
                req.set_scheme(Some(&Scheme::Other("bad\nscheme".to_string())))
                    .is_err()
            );

            assert!(req.set_path_with_query(Some("/bad\npath")).is_err());
        }

        println!("Done");
        Ok(())
    }
}

fn main() {}
