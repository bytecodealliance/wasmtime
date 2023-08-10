use anyhow::Result;
use wasi_http_tests::bindings::wasi::http::types::{Method, Scheme};

struct Component;

fn main() -> Result<(), ()> {
    let res = wasi_http_tests::request(
        Method::Connect,
        Scheme::Http,
        "localhost:3001",
        "/",
        None,
        Some(&[]),
    );

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

    Ok(())
}

impl wasi_http_tests::bindings::CommandExtended for Component {
    fn run() -> Result<(), ()> {
        main()
    }
}

wasi_http_tests::export_command_extended!(Component);
