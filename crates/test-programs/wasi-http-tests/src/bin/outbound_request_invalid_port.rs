use anyhow::Result;
use wasi_http_tests::bindings::wasi::http::types::{Method, Scheme};

struct Component;

fn main() -> Result<(), ()> {
    let res = wasi_http_tests::request(
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

    Ok(())
}

impl wasi_http_tests::bindings::CommandExtended for Component {
    fn run() -> Result<(), ()> {
        main()
    }
}

wasi_http_tests::export_command_extended!(Component);
