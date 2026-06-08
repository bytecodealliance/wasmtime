mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "cross-instance-source",
    });

    use super::Component;
    export!(Component);
}

use {
    bindings::{exports::local::local::cross_instance::Guest, wit_future, wit_stream},
    wit_bindgen::{FutureReader, StreamReader},
};

struct Component;

impl Guest for Component {
    async fn make() -> (StreamReader<u8>, FutureReader<u8>) {
        let (mut stream_tx, stream_rx) = wit_stream::new();
        let (future_tx, future_rx) = wit_future::new(|| 0);

        wit_bindgen::spawn_local(async move {
            assert!(stream_tx.write_all(vec![2, 4, 6, 8, 9]).await.is_empty());
            future_tx.write(10).await.unwrap();
        });

        (stream_rx, future_rx)
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
