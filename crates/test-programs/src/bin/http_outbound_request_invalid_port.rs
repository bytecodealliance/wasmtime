use test_programs::wasi::http::types::{Method, Scheme};

fn main() {
    let res = test_programs::http::request(
        Method::Get,
        Scheme::Http,
        "localhost:99999",
        "/",
        None,
        None,
    );

    let error = res.unwrap_err();
    assert_eq!(
        error.to_string(),
        "Error::InvalidUrl(\"invalid port value\")"
    );
}
