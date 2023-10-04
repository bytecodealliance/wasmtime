use wasi_http_tests::bindings::wasi::http::types::{Method, Scheme};

fn main() {
    let addr = std::env::var("HTTP_SERVER").unwrap();
    let res = wasi_http_tests::request(Method::Connect, Scheme::Http, &addr, "/", None, Some(&[]));

    let error = res.unwrap_err().to_string();
    if !error.starts_with("Error::ProtocolError(\"") {
        panic!(
            r#"assertion failed: `(left == right)`
      left: `"{error}"`,
      right: `"Error::ProtocolError(\"invalid HTTP version parsed\")"`
            or `"Error::ProtocolError(\"operation was canceled\")"`)"#
        )
    }
}
