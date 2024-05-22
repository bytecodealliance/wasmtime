use anyhow::{anyhow, Result};

use test_programs::wasi::http::{
    outgoing_handler::{self, ErrorCode, OutgoingRequest},
    types::{IncomingRequest, IncomingResponse, OutgoingBody, OutgoingResponse, ResponseOutparam},
};

struct T;

test_programs::proxy::export!(T);

impl test_programs::proxy::exports::wasi::http::incoming_handler::Guest for T {
    fn handle(request: IncomingRequest, outparam: ResponseOutparam) {
        assert!(request.scheme().is_some());
        assert!(request.authority().is_some());
        assert!(request.path_with_query().is_some());

        test_filesystem();

        let req_hdrs = request.headers();

        let out_request = OutgoingRequest::new(req_hdrs);
        out_request
            .set_authority(request.authority().as_deref())
            .expect("set authority");
        out_request
            .set_method(&request.method())
            .expect("set method");

        out_request
            .set_scheme(request.scheme().as_ref())
            .expect("set scheme");

        let incoming_response = Self::outgoing_request_send(out_request);

        match incoming_response {
            Ok(resp) => {
                let outgoing_response = OutgoingResponse::new(resp.headers());
                outgoing_response
                    .set_status_code(resp.status())
                    .expect("set status");

                let body = outgoing_response.body().expect("outgoing response body");

                let out = body.write().expect("outgoing stream");
                let input_stream = resp
                    .consume()
                    .expect("incoming response body")
                    .stream()
                    .expect("input stream");

                let content = input_stream.read(1000).expect("reading response body");
                out.blocking_write_and_flush(&content)
                    .expect("writing response");
                drop(out);
                ResponseOutparam::set(outparam, Ok(outgoing_response));
                OutgoingBody::finish(body, None).expect("outgoing-body.finish");
            }
            Err(e) => {
                ResponseOutparam::set(outparam, Err(ErrorCode::InternalError(Some(e.to_string()))));
            }
        }
    }
}

impl T {
    pub fn outgoing_request_send(request: OutgoingRequest) -> Result<IncomingResponse> {
        let outgoing_body = request
            .body()
            .map_err(|_| anyhow!("outgoing request write failed"))?;

        let future_response = outgoing_handler::handle(request, None)?;
        OutgoingBody::finish(outgoing_body, None)?;

        let incoming_response = match future_response.get() {
            Some(result) => result.map_err(|()| anyhow!("response already taken"))?,
            None => {
                let pollable = future_response.subscribe();
                pollable.block();
                future_response
                    .get()
                    .expect("incoming response available")
                    .map_err(|()| anyhow!("response already taken"))?
            }
        }?;

        Ok(incoming_response)
    }
}

// Technically this should not be here for a proxy, but given the current
// framework for tests it's required since this file is built as a `bin`
fn main() {}

fn test_filesystem() {
    assert!(std::fs::File::open(".").is_err());
}
