use crate::proxy::types::new_fields;
use crate::proxy::types::new_outgoing_request;
use crate::proxy::types::MethodParam;
use crate::proxy::types::SchemeParam;

pub mod proxy;

fn request(method: MethodParam, scheme: Option<SchemeParam>, host: &str, path: &str) {
    let headers = new_fields(&[
        ("Content-type", "text/plain"),
        ("User-agent", "wasm32-wasi-rust"),
    ]);

    let req = new_outgoing_request(method, path, "", scheme, host, headers);
    let fut = crate::proxy::default_outgoing_http::handle(req, None);
    let res = crate::proxy::types::future_incoming_response_get(fut)
        .unwrap()
        .unwrap();
    let code = crate::proxy::types::incoming_response_status(res);
    let response_headers = crate::proxy::types::incoming_response_headers(res);
    let stream = crate::proxy::types::incoming_response_consume(res).unwrap();
    let body = crate::proxy::streams::read(stream, 60 * 1024).unwrap().0;

    println!("Status is {}", code);
    println!("Headers are:");
    let entries = crate::proxy::types::fields_entries(response_headers);
    for tuple in entries.iter() {
        println!("{}: {}", tuple.0, tuple.1);
    }
    println!("{}", String::from_utf8(body).unwrap());
}

fn main() {
    request(
        MethodParam::Get,
        Some(SchemeParam::Https),
        "postman-echo.com",
        "/get",
    );
    request(
        MethodParam::Post,
        Some(SchemeParam::Https),
        "postman-echo.com",
        "/post",
    );
    request(
        MethodParam::Put,
        Some(SchemeParam::Http),
        "postman-echo.com",
        "/put",
    );
}
