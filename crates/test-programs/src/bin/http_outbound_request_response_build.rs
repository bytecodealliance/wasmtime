use test_programs::wasi::http::types as http_types;

fn main() {
    println!("Called _start");
    {
        let headers = http_types::Headers::from_list(&[(
            "Content-Type".to_string(),
            "application/json".to_string().into_bytes(),
        )])
        .unwrap();
        let request = http_types::OutgoingRequest::new(headers);

        request
            .set_method(&http_types::Method::Get)
            .expect("setting method");
        request
            .set_scheme(Some(&http_types::Scheme::Https))
            .expect("setting scheme");
        request
            .set_authority(Some("www.example.com"))
            .expect("setting authority");

        let outgoing_body = request.body().unwrap();
        let request_body = outgoing_body.write().unwrap();
        request_body
            .blocking_write_and_flush("request-body".as_bytes())
            .unwrap();
    }
    {
        let headers = http_types::Headers::from_list(&[(
            "Content-Type".to_string(),
            "application/text".to_string().into_bytes(),
        )])
        .unwrap();
        let response = http_types::OutgoingResponse::new(headers);
        let outgoing_body = response.body().unwrap();
        let response_body = outgoing_body.write().unwrap();
        response_body
            .blocking_write_and_flush("response-body".as_bytes())
            .unwrap();
    }

    {
        let req = http_types::OutgoingRequest::new(http_types::Fields::new());

        assert!(
            req.set_method(&http_types::Method::Other("invalid method".to_string()))
                .is_err()
        );

        assert!(req.set_authority(Some("bad-port:99999")).is_err());
        assert!(req.set_authority(Some("bad-\nhost")).is_err());
        assert!(req.set_authority(Some("too-many-ports:80:80:80")).is_err());

        assert!(
            req.set_scheme(Some(&http_types::Scheme::Other("bad\nscheme".to_string())))
                .is_err()
        );

        assert!(req.set_path_with_query(Some("/bad\npath")).is_err());
    }

    println!("Done");
}
