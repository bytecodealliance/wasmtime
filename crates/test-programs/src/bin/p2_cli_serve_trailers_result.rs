use test_programs::proxy;
use test_programs::wasi::http::types::{
    Fields, IncomingBody, IncomingRequest, OutgoingBody, OutgoingResponse, ResponseOutparam,
};
use test_programs::wasi::io::streams::StreamError;

struct T;

proxy::export!(T);

impl proxy::exports::wasi::http::incoming_handler::Guest for T {
    fn handle(request: IncomingRequest, outparam: ResponseOutparam) {
        // Reports what the body stream saw and what `future-trailers` said,
        // so one request can assert both together.
        let (read, trailers) = match request.consume() {
            Err(e) => (format!("consume-error={e:?}"), String::from("not-reached")),
            Ok(body) => {
                let read = match body.stream() {
                    Err(e) => format!("stream-error={e:?}"),
                    Ok(stream) => {
                        let mut bytes = 0usize;
                        let note = loop {
                            match stream.read(4096) {
                                Ok(buf) if buf.is_empty() => break format!("eof bytes={bytes}"),
                                Ok(buf) => bytes += buf.len(),
                                Err(StreamError::Closed) => break format!("closed bytes={bytes}"),
                                Err(StreamError::LastOperationFailed(_)) => {
                                    break format!("failed bytes={bytes}");
                                }
                            }
                        };
                        drop(stream);
                        note
                    }
                };

                let future = IncomingBody::finish(body);
                future.subscribe().block();
                let trailers = match future.get() {
                    Some(Ok(Ok(Some(fields)))) => format!("field-count={}", fields.entries().len()),
                    Some(Ok(Ok(None))) => String::from("none"),
                    Some(Ok(Err(code))) => format!("error-code={code:?}"),
                    Some(Err(())) => String::from("outer-error"),
                    None => String::from("pending"),
                };
                (read, trailers)
            }
        };

        let resp = OutgoingResponse::new(Fields::new());
        let body = resp.body().expect("outgoing response");
        ResponseOutparam::set(outparam, Ok(resp));
        let out = body.write().expect("outgoing stream");
        out.blocking_write_and_flush(format!("read={read} trailers={trailers}").as_bytes())
            .expect("writing response");
        drop(out);
        OutgoingBody::finish(body, None).expect("outgoing-body.finish");
    }
}

fn main() {}
