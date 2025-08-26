use test_programs::p3::wasi::http::types::{Method, Scheme};

struct Component;

test_programs::p3::export!(Component);

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        let res = test_programs::p3::http::request(
            Method::Get,
            Scheme::Http,
            "localhost:99999",
            "/",
            None,
            None,
            None,
            None,
            None,
        )
        .await;

        assert!(res.is_err());
        Ok(())
    }
}

fn main() {}
