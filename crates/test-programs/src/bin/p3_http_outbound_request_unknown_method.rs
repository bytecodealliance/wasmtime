use test_programs::p3::wasi::http::types::{Method, Scheme};

struct Component;

test_programs::p3::export!(Component);

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        let res = test_programs::p3::http::request(
            Method::Other("bad\nmethod".to_owned()),
            Scheme::Http,
            "localhost:3000",
            "/",
            None,
            None,
            None,
            None,
            None,
        )
        .await;

        // This error arises from input validation in the `set_method` function on `OutgoingRequest`.
        assert_eq!(res.unwrap_err().to_string(), "failed to set method");
        Ok(())
    }
}

fn main() {}
