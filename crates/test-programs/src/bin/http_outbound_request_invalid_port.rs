use test_programs::wasi::http::types::{Method, Scheme, ValidationError};

fn main() {
    let res = test_programs::http::request(
        Method::Get,
        Scheme::Http,
        "localhost:99999",
        "/",
        None,
        None,
    );

    assert!(matches!(
        res.unwrap_err().downcast(),
        Ok(ValidationError::InvalidSyntax),
    ));
}
