use anyhow::{bail, Result};
use bindings::wasi::http::types::{
    Fields, IncomingRequest, Method, OutgoingBody, OutgoingRequest, OutgoingResponse,
    ResponseOutparam, Scheme,
};
use futures::{stream, SinkExt, StreamExt, TryStreamExt};
use url::Url;

mod bindings {
    use super::Handler;

    wit_bindgen::generate!({
        path: "../wasi-http/wit",
        world: "wasi:http/proxy",
        exports: {
            "wasi:http/incoming-handler": Handler,
        },
    });
}

const MAX_CONCURRENCY: usize = 16;

struct Handler;

impl bindings::exports::wasi::http::incoming_handler::Guest for Handler {
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
        executor::run(async move {
            handle_request(request, response_out).await;
        })
    }
}

async fn handle_request(request: IncomingRequest, response_out: ResponseOutparam) {
    let headers = request.headers().entries();

    match (request.method(), request.path_with_query().as_deref()) {
        (Method::Get, Some("/hash-all")) => {
            let urls = headers.iter().filter_map(|(k, v)| {
                (k == "url")
                    .then_some(v)
                    .and_then(|v| std::str::from_utf8(v).ok())
                    .and_then(|v| Url::parse(v).ok())
            });

            let results = urls.map(|url| async move {
                let result = hash(&url).await;
                (url, result)
            });

            let mut results = stream::iter(results).buffer_unordered(MAX_CONCURRENCY);

            let response = OutgoingResponse::new(
                200,
                &Fields::new(&[("content-type".to_string(), b"text/plain".to_vec())]),
            );

            let mut body =
                executor::outgoing_body(response.write().expect("response should be writable"));

            ResponseOutparam::set(response_out, Ok(response));

            while let Some((url, result)) = results.next().await {
                let payload = match result {
                    Ok(hash) => format!("{url}: {hash}\n"),
                    Err(e) => format!("{url}: {e:?}\n"),
                }
                .into_bytes();

                if let Err(e) = body.send(payload).await {
                    eprintln!("Error sending payload: {e}");
                }
            }
        }

        (Method::Post, Some("/echo")) => {
            let response = OutgoingResponse::new(
                200,
                &Fields::new(
                    &headers
                        .into_iter()
                        .filter_map(|(k, v)| (k == "content-type").then_some((k, v)))
                        .collect::<Vec<_>>(),
                ),
            );

            let mut body =
                executor::outgoing_body(response.write().expect("response should be writable"));

            ResponseOutparam::set(response_out, Ok(response));

            let mut stream =
                executor::incoming_body(request.consume().expect("request should be readable"));

            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(chunk) => {
                        if let Err(e) = body.send(chunk).await {
                            eprintln!("Error sending body: {e}");
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("Error receiving body: {e}");
                        break;
                    }
                }
            }
        }

        _ => {
            let response = OutgoingResponse::new(405, &Fields::new(&[]));

            let body = response.write().expect("response should be writable");

            ResponseOutparam::set(response_out, Ok(response));

            OutgoingBody::finish(body, None);
        }
    }
}

async fn hash(url: &Url) -> Result<String> {
    let request = OutgoingRequest::new(
        &Method::Get,
        Some(url.path()),
        Some(&match url.scheme() {
            "http" => Scheme::Http,
            "https" => Scheme::Https,
            scheme => Scheme::Other(scheme.into()),
        }),
        Some(&format!(
            "{}{}",
            url.host_str().unwrap_or(""),
            if let Some(port) = url.port() {
                format!(":{port}")
            } else {
                String::new()
            }
        )),
        &Fields::new(&[]),
    );

    let response = executor::outgoing_request_send(request).await?;

    let status = response.status();

    if !(200..300).contains(&status) {
        bail!("unexpected status: {status}");
    }

    let mut body =
        executor::incoming_body(response.consume().expect("response should be readable"));

    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    while let Some(chunk) = body.try_next().await? {
        hasher.update(&chunk);
    }

    use base64::Engine;
    Ok(base64::engine::general_purpose::STANDARD_NO_PAD.encode(hasher.finalize()))
}

// Technically this should not be here for a proxy, but given the current
// framework for tests it's required since this file is built as a `bin`
fn main() {}

mod executor {
    use super::bindings::wasi::{
        http::{
            outgoing_handler,
            types::{
                self, IncomingBody, IncomingResponse, InputStream, OutgoingBody, OutgoingRequest,
                OutputStream,
            },
        },
        io::{self, streams::StreamError},
    };
    use anyhow::{anyhow, Error, Result};
    use futures::{future, sink, stream, Sink, Stream};
    use std::{
        cell::RefCell,
        future::Future,
        mem,
        rc::Rc,
        sync::{Arc, Mutex},
        task::{Context, Poll, Wake, Waker},
    };

    const READ_SIZE: u64 = 16 * 1024;

    static WAKERS: Mutex<Vec<(io::poll::Pollable, Waker)>> = Mutex::new(Vec::new());

    pub fn run<T>(future: impl Future<Output = T>) -> T {
        futures::pin_mut!(future);

        struct DummyWaker;

        impl Wake for DummyWaker {
            fn wake(self: Arc<Self>) {}
        }

        let waker = Arc::new(DummyWaker).into();

        loop {
            match future.as_mut().poll(&mut Context::from_waker(&waker)) {
                Poll::Pending => {
                    let mut new_wakers = Vec::new();

                    let wakers = mem::take::<Vec<_>>(&mut WAKERS.lock().unwrap());

                    assert!(!wakers.is_empty());

                    let pollables = wakers
                        .iter()
                        .map(|(pollable, _)| pollable)
                        .collect::<Vec<_>>();

                    let mut ready = vec![false; wakers.len()];

                    for index in io::poll::poll_list(&pollables) {
                        ready[usize::try_from(index).unwrap()] = true;
                    }

                    for (ready, (pollable, waker)) in ready.into_iter().zip(wakers) {
                        if ready {
                            waker.wake()
                        } else {
                            new_wakers.push((pollable, waker));
                        }
                    }

                    *WAKERS.lock().unwrap() = new_wakers;
                }
                Poll::Ready(result) => break result,
            }
        }
    }

    pub fn outgoing_body(body: OutgoingBody) -> impl Sink<Vec<u8>, Error = Error> {
        struct Outgoing(Option<(OutputStream, OutgoingBody)>);

        impl Drop for Outgoing {
            fn drop(&mut self) {
                if let Some((stream, body)) = self.0.take() {
                    drop(stream);
                    OutgoingBody::finish(body, None);
                }
            }
        }

        let stream = body.write().expect("response body should be writable");
        let pair = Rc::new(RefCell::new(Outgoing(Some((stream, body)))));

        sink::unfold((), {
            move |(), chunk: Vec<u8>| {
                future::poll_fn({
                    let mut offset = 0;
                    let mut flushing = false;
                    let pair = pair.clone();

                    move |context| {
                        let pair = pair.borrow();
                        let (stream, _) = &pair.0.as_ref().unwrap();

                        loop {
                            match stream.check_write() {
                                Ok(0) => {
                                    WAKERS
                                        .lock()
                                        .unwrap()
                                        .push((stream.subscribe(), context.waker().clone()));

                                    break Poll::Pending;
                                }
                                Ok(count) => {
                                    if offset == chunk.len() {
                                        if flushing {
                                            break Poll::Ready(Ok(()));
                                        } else {
                                            stream.flush().expect("stream should be flushable");
                                            flushing = true;
                                        }
                                    } else {
                                        let count = usize::try_from(count)
                                            .unwrap()
                                            .min(chunk.len() - offset);

                                        match stream.write(&chunk[offset..][..count]) {
                                            Ok(()) => {
                                                offset += count;
                                            }
                                            Err(_) => break Poll::Ready(Err(anyhow!("I/O error"))),
                                        }
                                    }
                                }
                                Err(_) => break Poll::Ready(Err(anyhow!("I/O error"))),
                            }
                        }
                    }
                })
            }
        })
    }

    pub fn outgoing_request_send(
        request: OutgoingRequest,
    ) -> impl Future<Output = Result<IncomingResponse, types::Error>> {
        future::poll_fn({
            let response = outgoing_handler::handle(request, None);

            move |context| match &response {
                Ok(response) => {
                    if let Some(response) = response.get() {
                        Poll::Ready(response.unwrap())
                    } else {
                        WAKERS
                            .lock()
                            .unwrap()
                            .push((response.subscribe(), context.waker().clone()));
                        Poll::Pending
                    }
                }
                Err(error) => Poll::Ready(Err(error.clone())),
            }
        })
    }

    pub fn incoming_body(body: IncomingBody) -> impl Stream<Item = Result<Vec<u8>>> {
        struct Incoming(Option<(InputStream, IncomingBody)>);

        impl Drop for Incoming {
            fn drop(&mut self) {
                if let Some((stream, body)) = self.0.take() {
                    drop(stream);
                    IncomingBody::finish(body);
                }
            }
        }

        stream::poll_fn({
            let stream = body.stream().expect("response body should be readable");
            let pair = Incoming(Some((stream, body)));

            move |context| {
                if let Some((stream, _)) = &pair.0 {
                    match stream.read(READ_SIZE) {
                        Ok(buffer) => {
                            if buffer.is_empty() {
                                WAKERS
                                    .lock()
                                    .unwrap()
                                    .push((stream.subscribe(), context.waker().clone()));
                                Poll::Pending
                            } else {
                                Poll::Ready(Some(Ok(buffer)))
                            }
                        }
                        Err(StreamError::Closed) => Poll::Ready(None),
                        Err(StreamError::LastOperationFailed(error)) => {
                            Poll::Ready(Some(Err(anyhow!("{}", error.to_debug_string()))))
                        }
                    }
                } else {
                    Poll::Ready(None)
                }
            }
        })
    }
}
