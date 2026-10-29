use crate::http_server::Server;
use wasmtime::Result;

/// Runs a guest which fetches `/dup` from a raw server, whose response
/// carries two conflicting `content-length` values, and then forwards those
/// headers in a new request to `/victim`, which the host must reject.
///
/// `run_guest` instantiates and runs the guest against the given server; the
/// guest itself asserts the error it observes. Afterwards this asserts that
/// nothing but the guest's `/dup` requests ever reached the wire.
pub async fn forward_duplicate_content_length(
    run_guest: impl AsyncFnOnce(&Server) -> Result<()>,
) -> Result<()> {
    // `hyper`'s client accepts this response since `transfer-encoding` takes
    // precedence over `content-length`, but both values reach the guest.
    const DUP: &[u8] = b"HTTP/1.1 200 OK\r\n\
        content-length: 5\r\n\
        content-length: 100\r\n\
        transfer-encoding: chunked\r\n\
        \r\n\
        0\r\n\r\n";
    // The guest fetches `/dup` twice, and must never send `/victim`; the
    // extra connections let a `/victim` request be observed should it
    // escape.
    let (server, requests) = Server::raw(4, |head| {
        if head.starts_with(b"GET /dup ") {
            DUP.to_vec()
        } else {
            b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n".to_vec()
        }
    })?;
    run_guest(&server).await?;
    drop(server);

    let requests = requests.get();
    assert_eq!(requests.len(), 2, "unexpected requests: {requests:?}");
    for head in &requests {
        assert!(
            head.starts_with(b"GET /dup "),
            "unexpected request reached the wire: {}",
            String::from_utf8_lossy(head)
        );
    }
    Ok(())
}
