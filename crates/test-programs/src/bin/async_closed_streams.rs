mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "closed-streams",
        async: true,
    });

    use super::Component;
    export!(Component);
}

use {
    bindings::exports::local::local::closed::Guest,
    std::mem,
    wit_bindgen::{FutureReader, StreamReader, StreamResult},
};

struct Component;

impl Guest for Component {
    async fn read_stream(mut rx: StreamReader<u8>, expected: Vec<u8>) {
        let mut buffer = Vec::with_capacity(expected.len());
        loop {
            let (result, buf) = rx.read(mem::replace(&mut buffer, Vec::new())).await;
            buffer = buf;
            if !matches!(result, StreamResult::Complete(_)) {
                break;
            }
        }
        assert_eq!(buffer, expected);
    }

    async fn read_future(rx: FutureReader<u8>, expected: u8, _rx_ignored: FutureReader<u8>) {
        assert_eq!(rx.await, expected);
    }

    async fn read_future_post_return(
        rx: FutureReader<u8>,
        expected: u8,
        _rx_ignored: FutureReader<u8>,
    ) {
        wit_bindgen::spawn(async move {
            assert_eq!(rx.await, expected);
        });
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
