use test_programs::wasi::http::types as http_types;

fn main() {
    println!("Called _start");
    {
        let headers = http_types::Headers::from_list(&[(
            "Content-Type".to_string(),
            "application/json".to_string().into_bytes(),
        )])
        .unwrap();
        let request = http_types::OutgoingRequest::new(
            &http_types::Method::Get,
            None,
            Some(&http_types::Scheme::Https),
            Some("www.example.com"),
            headers,
        );
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
    println!("Done");
}
