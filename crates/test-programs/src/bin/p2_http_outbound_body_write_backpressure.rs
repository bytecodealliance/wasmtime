use test_programs::wasi::http::types as http_types;

fn main() {
    let request = http_types::OutgoingRequest::new(http_types::Headers::new());
    let outgoing_body = request.body().unwrap();

    let request_body = outgoing_body.write().unwrap();
    request_body.subscribe().block();
    let permitted = request_body.check_write().unwrap() as usize;
    let too_much = vec![0; permitted + 1];
    let _ = request_body.write(&too_much);
    unreachable!("writing more than permitted should trap");
}
