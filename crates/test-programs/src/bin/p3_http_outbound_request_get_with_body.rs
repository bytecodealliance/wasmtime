use anyhow::Context;
use test_programs::p3::wasi::http::types::{Method, Scheme};

struct Component;

test_programs::p3::export!(Component);

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        const BODY: &[u8] = b"0123456789";
        let addr = test_programs::p3::wasi::cli::environment::get_environment()
            .into_iter()
            .find_map(|(k, v)| k.eq("HTTP_SERVER").then_some(v))
            .unwrap();

        // Same shape with `POST`, which the encoder already frames as chunked.
        let res = test_programs::p3::http::request(
            Method::Post,
            Scheme::Http,
            &addr,
            "/post-with-body",
            Some(BODY),
            None,
            None,
            None,
            None,
        )
        .await
        .context("/post-with-body")
        .unwrap();
        println!("/post-with-body: {res:?}");
        assert_eq!(res.status, 200);
        assert_eq!(res.body, BODY, "invalid body returned");

        // The request has a body but no framing header, which the HTTP/1.1 encoder
        // requires for `GET` to send the body at all.
        let res = test_programs::p3::http::request(
            Method::Get,
            Scheme::Http,
            &addr,
            "/get-with-body",
            Some(BODY),
            None,
            None,
            None,
            None,
        )
        .await
        .context("/get-with-body")
        .unwrap();
        println!("/get-with-body: {res:?}");
        assert_eq!(res.status, 200);
        assert_eq!(res.body, BODY, "invalid body returned");
        let res = test_programs::p3::http::request(
            Method::Get,
            Scheme::Http,
            &addr,
            "/get-with-empty-body",
            Some(b""),
            None,
            None,
            None,
            None,
        )
        .await
        .context("/get-with-empty-body")
        .unwrap();
        println!("/get-with-empty-body: {res:?}");
        assert_eq!(res.status, 200);
        assert_eq!(res.body, b"", "invalid body returned");
        Ok(())
    }
}

fn main() {}
