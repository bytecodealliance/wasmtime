use wasi_http_tests::bindings::wasi::http::types::{Method, Scheme};

fn main() {
    wasi_http_tests::in_tokio(async { run().await })
}

async fn run() {
    let res = wasi_http_tests::request(
        Method::Connect,
        Scheme::Http,
        "localhost:3001",
        "/",
        None,
        Some(&[]),
    )
    .await;

    let error = res.unwrap_err().to_string();
    if error.ne("Error::ProtocolError(\"invalid HTTP version parsed\")")
        && error.ne("Error::ProtocolError(\"operation was canceled\")")
    {
        panic!(
            r#"assertion failed: `(left == right)`
      left: `"{error}"`,
      right: `"Error::ProtocolError(\"invalid HTTP version parsed\")"`
            or `"Error::ProtocolError(\"operation was canceled\")"`)"#
        )
    }
}
