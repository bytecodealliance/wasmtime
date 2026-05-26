use anyhow::{Context, Result};
use test_programs::wasi::http::types::{
    Headers, IncomingRequest, Method, OutgoingBody, OutgoingResponse, ResponseOutparam,
};

struct T;

test_programs::proxy::export!(T);

impl test_programs::proxy::exports::wasi::http::incoming_handler::Guest for T {
    fn handle(request: IncomingRequest, outparam: ResponseOutparam) {
        assert!(request.scheme().is_some());
        assert!(request.authority().is_some());
        assert!(request.path_with_query().is_some());

        test_filesystem();

        match (request.method(), request.path_with_query().as_deref()) {
            (Method::Get, Some("/early_drop")) => {
                // Ignore all the errors for this endpoint.
                let resp = OutgoingResponse::new(Headers::new());
                let body = resp.body().expect("outgoing response");
                ResponseOutparam::set(outparam, Ok(resp));
                let _ = body.write().and_then(|out| {
                    let _ = out.blocking_write_and_flush(b"hello, world!");
                    drop(out);
                    Ok(())
                });
                let _ = OutgoingBody::finish(body, None);

                return;
            }
            (Method::Get, Some(p)) if p.starts_with("/modify_fields/") => {
                let r = modify_fields_handler(request);
                response_for(r, outparam);
                return;
            }
            (Method::Get, Some(p)) if p.starts_with("/new_fields/") => {
                let r = new_fields_handler(request);
                response_for(r, outparam);
                return;
            }

            _ => {}
        }

        let header = String::from("custom-forbidden-header");
        let req_hdrs = request.headers();

        assert!(
            !req_hdrs.has(&header),
            "forbidden `custom-forbidden-header` found in request"
        );

        assert!(req_hdrs.delete(&header).is_err());
        assert!(req_hdrs.append(&header, b"no".as_ref()).is_err());

        assert!(
            !req_hdrs.has(&header),
            "append of forbidden header succeeded"
        );

        assert!(
            !req_hdrs.has("host"),
            "forbidden host header present in incoming request"
        );

        let hdrs = Headers::new();
        let resp = OutgoingResponse::new(hdrs);
        let body = resp.body().expect("outgoing response");

        ResponseOutparam::set(outparam, Ok(resp));

        let out = body.write().expect("outgoing stream");
        out.blocking_write_and_flush(b"hello, world!")
            .expect("writing response");

        drop(out);
        OutgoingBody::finish(body, None).expect("outgoing-body.finish");
    }
}

fn response_for(r: Result<()>, outparam: ResponseOutparam) {
    let resp = OutgoingResponse::new(Headers::new());
    resp.set_status_code(if r.is_ok() { 200 } else { 500 })
        .unwrap();
    let body = resp.body().expect("outgoing response");
    ResponseOutparam::set(outparam, Ok(resp));
    let _ = body.write().and_then(|out| {
        let _ = out.blocking_write_and_flush(format!("{r:?}").as_bytes());
        drop(out);
        Ok(())
    });
    let _ = OutgoingBody::finish(body, None);
}

// Technically this should not be here for a proxy, but given the current
// framework for tests it's required since this file is built as a `bin`
fn main() {}

fn test_filesystem() {
    assert!(std::fs::File::open(".").is_err());
}

fn add_bytes_to_headers(headers: Headers, size: usize) {
    if size == 0 {
        return;
    } else if size < 10 {
        headers.append("k", &b"abcdefghi"[0..size - 1]).unwrap()
    } else {
        for chunk in 0..(size / 10) {
            let k = format!("g{chunk:04}");
            let mut v = format!("h{chunk:04}");
            if chunk == 0 {
                for _ in 0..(size % 10) {
                    v.push('#');
                }
            }
            headers.append(k.as_str(), v.as_bytes()).unwrap()
        }
    }
}

fn modify_fields_handler(request: IncomingRequest) -> Result<()> {
    let path = request.path_with_query().unwrap();
    let rest = path.trim_start_matches("/modify_fields/");
    let added_field_bytes: usize = rest
        .parse()
        .context("expect remainder of url to parse as number")?;
    add_bytes_to_headers(request.headers().clone(), added_field_bytes);

    Ok(())
}
fn new_fields_handler(request: IncomingRequest) -> Result<()> {
    let path = request.path_with_query().unwrap();
    let rest = path.trim_start_matches("/new_fields/");
    let added_field_bytes: usize = rest
        .parse()
        .context("expect remainder of url to parse as number")?;
    add_bytes_to_headers(Headers::new(), added_field_bytes);

    Ok(())
}
