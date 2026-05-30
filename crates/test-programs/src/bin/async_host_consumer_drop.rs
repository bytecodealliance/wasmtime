mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "host-consumer-drop-guest",
        async: true,
    });

    use super::Component;
    export!(Component);
}

use {bindings::exports::local::local::host_consumer_drop::Guest, wit_bindgen::StreamReader};

struct Component;

impl Guest for Component {
    async fn get() -> StreamReader<u8> {
        let (mut tx, rx) = bindings::wit_stream::new();
        // Keep the writable end and hand the readable end to the host. The host
        // attaches a consumer (read side -> `HostReady`); the write below blocks
        // until that consumer reads, after which we drop the writer. Dropping it
        // while the consumer is still `HostReady` is the path that used to leak.
        wit_bindgen::spawn(async move {
            assert!(tx.write_one(42).await.is_none());
            drop(tx);
        });
        rx
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
