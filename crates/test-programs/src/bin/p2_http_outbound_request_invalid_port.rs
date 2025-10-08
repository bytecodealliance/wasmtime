use test_programs::wasi::http::types::{Method, Scheme};

fn main() {
    let res = test_programs::http::request(
        Method::Get,
        Scheme::Http,
        "localhost:99999",
        "/",
        None,
        None,
        None,
        None,
        None,
    );

    assert!(res.is_err());
}
