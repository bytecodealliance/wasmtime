use test_programs::wasi::http::types::{Method, Scheme};

fn main() {
    let res = test_programs::http::request(
        Method::Other("bad\nmethod".to_owned()),
        Scheme::Http,
        "localhost:3000",
        "/",
        None,
        None,
        None,
        None,
        None,
    );

    // This error arises from input validation in the `set_method` function on `OutgoingRequest`.
    assert_eq!(res.unwrap_err().to_string(), "failed to set method");
}
