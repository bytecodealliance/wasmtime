use test_programs::wasi::http::types::{Method, Scheme};

fn main() {
    let addr = std::env::var("HTTP_SERVER").unwrap();
    // A path of at least 65535 bytes before the `?`, which exceeds the maximum
    // length supported by `http::uri::PathAndQuery`.
    let path = format!("/{}?some=arg", "é".repeat(u16::MAX as usize + 10));
    let res = test_programs::http::request(
        Method::Get,
        Scheme::Http,
        &addr,
        &path,
        None,
        None,
        None,
        None,
        None,
    );
    println!("{addr} long path: {res:?}");
}
