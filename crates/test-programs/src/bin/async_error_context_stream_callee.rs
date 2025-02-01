mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "error-context-stream-callee",
        async: {
            exports: [
                "local:local/run#run",
                "local:local/run-stream#produce-then-error",
            ],
        }
    });

    use super::Component;
    export!(Component);
}
use bindings::wit_stream;
use wit_bindgen_rt::async_support::futures::SinkExt;
use wit_bindgen_rt::async_support::{self, StreamReader};

struct Component;

impl bindings::exports::local::local::run_stream::Guest for Component {
    async fn produce_then_error(times: u32) -> StreamReader<()> {
        let (mut tx, rx) = wit_stream::new();
        async_support::spawn(async move {
            for _ in 0..times {
                let _ = tx.send(vec![()]).await;
            }
            // tx.close_with_error(error_context_new("error".into()));
        });
        rx
    }
}

impl bindings::exports::local::local::run::Guest for Component {
    async fn run() {}
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
