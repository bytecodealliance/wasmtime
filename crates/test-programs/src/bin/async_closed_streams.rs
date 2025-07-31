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
    wit_bindgen_rt::async_support::{self, FutureReader, StreamReader, StreamResult},
};

struct Component;

impl Guest for Component {
    async fn read_stream(mut rx: StreamReader<u8>, expected: Vec<u8>) {
        let (result, buf) = rx.read(Vec::with_capacity(expected.len())).await;
        assert_eq!(result, StreamResult::Complete(expected.len()));
        assert_eq!(buf, expected);
    }

    async fn read_future(rx: FutureReader<u8>, expected: u8, _rx_ignored: FutureReader<u8>) {
        assert_eq!(rx.await, expected);
    }

    async fn read_future_post_return(
        rx: FutureReader<u8>,
        expected: u8,
        _rx_ignored: FutureReader<u8>,
    ) {
        async_support::spawn(async move {
            assert_eq!(rx.await, expected);
        });
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
