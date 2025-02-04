mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "unit-stream-caller",
        async: {
            imports: [
                "local:local/unit-stream#run",
            ],
            exports: [
                "local:local/run#run",
            ],
        }
    });

    use super::Component;
    export!(Component);
}

use {
    bindings::{exports::local::local::run::Guest, local::local::unit_stream},
    futures::StreamExt,
};

struct Component;

impl Guest for Component {
    async fn run() {
        let count = 42;
        let mut rx = unit_stream::run(count).await;

        let mut received = 0;
        while let Some(chunk) = rx.next().await {
            received += chunk.len();
        }

        assert_eq!(count, u32::try_from(received).unwrap());
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
