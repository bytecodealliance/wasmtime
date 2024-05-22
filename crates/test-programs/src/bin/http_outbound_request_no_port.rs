use test_programs::wasi::http::types::{Method, Scheme};

fn main() {
    let res = test_programs::http::request(
        Method::Get,
        Scheme::Http,
        "example.com",
        "/",
        None,
        None,
        None,
        None,
        None,
    )
    .expect("expected response");

    assert_eq!(res.status, 200);
}
