use futures::join;
use test_programs::p3::wasi::http::client;
use test_programs::p3::wasi::http::types::{Headers, Method, Request, Response, Scheme};
use test_programs::p3::{wit_future, wit_stream};
use wit_bindgen::StreamResult;

struct Component;

test_programs::p3::export!(Component);

fn bytes(offset: &mut usize, len: usize) -> Vec<u8> {
    let mut buf = Vec::with_capacity(len);
    for i in 0..len {
        buf.push(((*offset + i) % 251) as u8);
    }
    *offset += len;
    buf
}

fn addr() -> String {
    test_programs::p3::wasi::cli::environment::get_environment()
        .into_iter()
        .find_map(|(k, v)| k.eq("HTTP_SERVER").then_some(v))
        .unwrap()
}

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        test_chunked_write().await;
        Ok(())
    }
}

async fn test_chunked_write() {
    let headers = Headers::from_list(&[]).unwrap();
    let (mut contents_tx, contents_rx) = wit_stream::new();
    let (trailers_tx, trailers_rx) = wit_future::new(|| Ok(None));
    let (request, transmit) = Request::new(headers, Some(contents_rx), trailers_rx, None);
    configure(&request);

    let (transmit, written, echoed) = join!(
        async { transmit.await },
        async {
            let mut len = 16;
            let mut pos = 0;
            loop {
                assert!(len <= 128 << 20);
                let (result, remaining) = contents_tx.write(bytes(&mut pos, len)).await;
                assert_eq!(result, StreamResult::Complete(len - remaining.remaining()));
                if remaining.remaining() == 0 {
                    len = len.checked_mul(2).unwrap();
                } else {
                    pos -= remaining.remaining();
                    break;
                }
            }
            drop(contents_tx);
            _ = trailers_tx.write(Ok(None)).await;
            pos
        },
        async { send_and_collect(request).await },
    );
    transmit.unwrap();
    assert_eq!(echoed, bytes(&mut 0, written));
}

fn configure(request: &Request) {
    request.set_method(&Method::Post).unwrap();
    request.set_scheme(Some(&Scheme::Http)).unwrap();
    request.set_authority(Some(&addr())).unwrap();
    request.set_path_with_query(Some("/")).unwrap();
}

async fn send_and_collect(request: Request) -> Vec<u8> {
    let response = client::send(request).await.unwrap();
    assert_eq!(response.get_status_code(), 200);
    let (_, result_rx) = wit_future::new(|| Ok(()));
    let (body_rx, trailers_rx) = Response::consume_body(response, result_rx);
    let body = body_rx.collect().await;
    trailers_rx.await.unwrap();
    body
}

fn main() {
    unreachable!()
}
