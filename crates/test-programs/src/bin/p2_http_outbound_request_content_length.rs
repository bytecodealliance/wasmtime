use test_programs::wasi::http::types as http_types;

fn make_request() -> http_types::OutgoingRequest {
    let request = http_types::OutgoingRequest::new(
        http_types::Headers::from_list(&[("Content-Length".to_string(), b"11".to_vec())]).unwrap(),
    );

    request
        .set_method(&http_types::Method::Post)
        .expect("setting method");
    request
        .set_scheme(Some(&http_types::Scheme::Http))
        .expect("setting scheme");
    let addr = std::env::var("HTTP_SERVER").unwrap();
    request
        .set_authority(Some(&addr))
        .expect("setting authority");
    request
        .set_path_with_query(Some("/"))
        .expect("setting path with query");

    request
}

fn main() {
    {
        println!("writing enough");
        let request = make_request();
        let outgoing_body = request.body().unwrap();

        {
            let request_body = outgoing_body.write().unwrap();
            request_body
                .blocking_write_and_flush("long enough".as_bytes())
                .unwrap();
        }

        http_types::OutgoingBody::finish(outgoing_body, None).expect("enough written")
    }

    {
        println!("writing too little");
        let request = make_request();
        let outgoing_body = request.body().unwrap();

        {
            let request_body = outgoing_body.write().unwrap();
            request_body
                .blocking_write_and_flush("msg".as_bytes())
                .unwrap();
        }

        let e =
            http_types::OutgoingBody::finish(outgoing_body, None).expect_err("finish should fail");

        assert!(
            matches!(&e, http_types::ErrorCode::HttpRequestBodySize(Some(3))),
            "unexpected error: {e:#?}"
        );
    }

    {
        println!("writing too much");
        let request = make_request();
        let outgoing_body = request.body().unwrap();

        {
            let request_body = outgoing_body.write().unwrap();
            let e = request_body
                .blocking_write_and_flush("more than 11 bytes".as_bytes())
                .expect_err("write should fail");

            let e = match e {
                test_programs::wasi::io::streams::StreamError::LastOperationFailed(e) => {
                    http_types::http_error_code(&e)
                }
                test_programs::wasi::io::streams::StreamError::Closed => panic!("request closed"),
            };

            assert!(
                matches!(
                    e,
                    Some(http_types::ErrorCode::HttpRequestBodySize(Some(18)))
                ),
                "unexpected error {e:?}"
            );
        }

        let e =
            http_types::OutgoingBody::finish(outgoing_body, None).expect_err("finish should fail");

        assert!(
            matches!(&e, http_types::ErrorCode::HttpRequestBodySize(Some(18))),
            "unexpected error: {e:#?}"
        );
    }
}
