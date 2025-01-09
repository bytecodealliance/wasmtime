use anyhow::{anyhow, bail, Result};
use futures::{future, stream, Future, SinkExt, StreamExt, TryStreamExt};
use test_programs::wasi::http::types::{
    Fields, IncomingRequest, IncomingResponse, Method, OutgoingBody, OutgoingRequest,
    OutgoingResponse, ResponseOutparam, Scheme,
};
use url::Url;

const MAX_CONCURRENCY: usize = 16;

struct Handler;

test_programs::proxy::export!(Handler);

impl test_programs::proxy::exports::wasi::http::incoming_handler::Guest for Handler {
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
        executor::run(async move {
            handle_request(request, response_out).await;
        })
    }
}

async fn handle_request(request: IncomingRequest, response_out: ResponseOutparam) {
    let headers = request.headers().entries();

    assert!(request.authority().is_some());

    match (request.method(), request.path_with_query().as_deref()) {
        (Method::Get, Some("/hash-all")) => {
            // Send outgoing GET requests to the specified URLs and stream the hashes of the response bodies as
            // they arrive.

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
                Fields::from_list(&[("content-type".to_string(), b"text/plain".to_vec())]).unwrap(),
            );

            let mut body =
                executor::outgoing_body(response.body().expect("response should be writable"));

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
            // Echo the request body without buffering it.

            let response = OutgoingResponse::new(
                Fields::from_list(
                    &headers
                        .into_iter()
                        .filter_map(|(k, v)| (k == "content-type").then_some((k, v)))
                        .collect::<Vec<_>>(),
                )
                .unwrap(),
            );

            let mut body =
                executor::outgoing_body(response.body().expect("response should be writable"));

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

        (Method::Post, Some("/double-echo")) => {
            // Pipe the request body to an outgoing request and stream the response back to the client.

            if let Some(url) = headers.iter().find_map(|(k, v)| {
                (k == "url")
                    .then_some(v)
                    .and_then(|v| std::str::from_utf8(v).ok())
                    .and_then(|v| Url::parse(v).ok())
            }) {
                match double_echo(request, &url).await {
                    Ok((request_copy, response)) => {
                        let mut stream = executor::incoming_body(
                            response.consume().expect("response should be consumable"),
                        );

                        let response = OutgoingResponse::new(
                            Fields::from_list(
                                &headers
                                    .into_iter()
                                    .filter_map(|(k, v)| (k == "content-type").then_some((k, v)))
                                    .collect::<Vec<_>>(),
                            )
                            .unwrap(),
                        );

                        let mut body = executor::outgoing_body(
                            response.body().expect("response should be writable"),
                        );

                        ResponseOutparam::set(response_out, Ok(response));

                        let response_copy = async move {
                            while let Some(chunk) = stream.next().await {
                                body.send(chunk?).await?;
                            }
                            Ok::<_, anyhow::Error>(())
                        };

                        let (request_copy, response_copy) =
                            future::join(request_copy, response_copy).await;
                        if let Err(e) = request_copy.and(response_copy) {
                            eprintln!("error piping to and from {url}: {e}");
                        }
                    }

                    Err(e) => {
                        eprintln!("Error sending outgoing request to {url}: {e}");
                        server_error(response_out);
                    }
                }
            } else {
                bad_request(response_out);
            }
        }

        _ => method_not_allowed(response_out),
    }
}

async fn double_echo(
    incoming_request: IncomingRequest,
    url: &Url,
) -> Result<(impl Future<Output = Result<()>> + use<>, IncomingResponse)> {
    let outgoing_request = OutgoingRequest::new(Fields::new());

    outgoing_request
        .set_method(&Method::Post)
        .map_err(|()| anyhow!("failed to set method"))?;

    outgoing_request
        .set_path_with_query(Some(url.path()))
        .map_err(|()| anyhow!("failed to set path_with_query"))?;

    outgoing_request
        .set_scheme(Some(&match url.scheme() {
            "http" => Scheme::Http,
            "https" => Scheme::Https,
            scheme => Scheme::Other(scheme.into()),
        }))
        .map_err(|()| anyhow!("failed to set scheme"))?;

    outgoing_request
        .set_authority(Some(&format!(
            "{}{}",
            url.host_str().unwrap_or(""),
            if let Some(port) = url.port() {
                format!(":{port}")
            } else {
                String::new()
            }
        )))
        .map_err(|()| anyhow!("failed to set authority"))?;

    let mut body = executor::outgoing_body(
        outgoing_request
            .body()
            .expect("request body should be writable"),
    );

    let response = executor::outgoing_request_send(outgoing_request);

    let mut stream = executor::incoming_body(
        incoming_request
            .consume()
            .expect("request should be consumable"),
    );

    let copy = async move {
        while let Some(chunk) = stream.next().await {
            body.send(chunk?).await?;
        }
        Ok::<_, anyhow::Error>(())
    };

    let response = response.await?;

    let status = response.status();

    if !(200..300).contains(&status) {
        bail!("unexpected status: {status}");
    }

    Ok((copy, response))
}

fn server_error(response_out: ResponseOutparam) {
    respond(500, response_out)
}

fn bad_request(response_out: ResponseOutparam) {
    respond(400, response_out)
}

fn method_not_allowed(response_out: ResponseOutparam) {
    respond(405, response_out)
}

fn respond(status: u16, response_out: ResponseOutparam) {
    let response = OutgoingResponse::new(Fields::new());
    response
        .set_status_code(status)
        .expect("setting status code");

    let body = response.body().expect("response should be writable");

    ResponseOutparam::set(response_out, Ok(response));

    OutgoingBody::finish(body, None).expect("outgoing-body.finish");
}

async fn hash(url: &Url) -> Result<String> {
    let request = OutgoingRequest::new(Fields::new());

    request
        .set_path_with_query(Some(url.path()))
        .map_err(|()| anyhow!("failed to set path_with_query"))?;
    request
        .set_scheme(Some(&match url.scheme() {
            "http" => Scheme::Http,
            "https" => Scheme::Https,
            scheme => Scheme::Other(scheme.into()),
        }))
        .map_err(|()| anyhow!("failed to set scheme"))?;
    request
        .set_authority(Some(&format!(
            "{}{}",
            url.host_str().unwrap_or(""),
            if let Some(port) = url.port() {
                format!(":{port}")
            } else {
                String::new()
            }
        )))
        .map_err(|()| anyhow!("failed to set authority"))?;

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
    use test_programs::wasi::{
        http::{
            outgoing_handler,
            types::{
                self, FutureTrailers, IncomingBody, IncomingResponse, InputStream, OutgoingBody,
                OutgoingRequest, OutputStream,
            },
        },
        io::{self, streams::StreamError},
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

                    for index in io::poll::poll(&pollables) {
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
                    OutgoingBody::finish(body, None).expect("outgoing-body.finish");
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
    ) -> impl Future<Output = Result<IncomingResponse, types::ErrorCode>> {
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
        enum Inner {
            Stream {
                stream: InputStream,
                body: IncomingBody,
            },
            Trailers(FutureTrailers),
            Closed,
        }

        struct Incoming(Inner);

        impl Drop for Incoming {
            fn drop(&mut self) {
                match mem::replace(&mut self.0, Inner::Closed) {
                    Inner::Stream { stream, body } => {
                        drop(stream);
                        IncomingBody::finish(body);
                    }
                    Inner::Trailers(_) | Inner::Closed => {}
                }
            }
        }

        stream::poll_fn({
            let stream = body.stream().expect("response body should be readable");
            let mut incoming = Incoming(Inner::Stream { stream, body });

            move |context| {
                loop {
                    match &incoming.0 {
                        Inner::Stream { stream, .. } => match stream.read(READ_SIZE) {
                            Ok(buffer) => {
                                return if buffer.is_empty() {
                                    WAKERS
                                        .lock()
                                        .unwrap()
                                        .push((stream.subscribe(), context.waker().clone()));
                                    Poll::Pending
                                } else {
                                    Poll::Ready(Some(Ok(buffer)))
                                };
                            }
                            Err(StreamError::Closed) => {
                                let Inner::Stream { stream, body } =
                                    mem::replace(&mut incoming.0, Inner::Closed)
                                else {
                                    unreachable!();
                                };
                                drop(stream);
                                incoming.0 = Inner::Trailers(IncomingBody::finish(body));
                            }
                            Err(StreamError::LastOperationFailed(error)) => {
                                return Poll::Ready(Some(Err(anyhow!(
                                    "{}",
                                    error.to_debug_string()
                                ))));
                            }
                        },

                        Inner::Trailers(trailers) => {
                            match trailers.get() {
                                Some(Ok(trailers)) => {
                                    incoming.0 = Inner::Closed;
                                    match trailers {
                                        Ok(Some(_)) => {
                                            // Currently, we just ignore any trailers.  TODO: Add a test that
                                            // expects trailers and verify they match the expected contents.
                                        }
                                        Ok(None) => {
                                            // No trailers; nothing else to do.
                                        }
                                        Err(error) => {
                                            // Error reading the trailers: pass it on to the application.
                                            return Poll::Ready(Some(Err(anyhow!("{error:?}"))));
                                        }
                                    }
                                }
                                Some(Err(_)) => {
                                    // Should only happen if we try to retrieve the trailers twice, i.e. a bug in
                                    // this code.
                                    unreachable!();
                                }
                                None => {
                                    WAKERS
                                        .lock()
                                        .unwrap()
                                        .push((trailers.subscribe(), context.waker().clone()));
                                    return Poll::Pending;
                                }
                            }
                        }

                        Inner::Closed => {
                            return Poll::Ready(None);
                        }
                    }
                }
            }
        })
    }
}
