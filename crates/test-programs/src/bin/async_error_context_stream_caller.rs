mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "error-context-stream-caller",
        async: {
            imports: [
                "local:local/run-stream#run-error",
            ],
            exports: [
                "local:local/run#run",
            ],
        }
    });

    use super::Component;
    export!(Component);
}
use bindings::exports::local::local::run::Guest;
use futures::StreamExt;

struct Component;

impl Guest for Component {
    async fn run() {
        let mut stream = bindings::local::local::run_stream::produce_then_error(2);
        let Some(_) = stream.next().await else {
            panic!("unexpected send #1");
        };
        let Some(_) = stream.next().await else {
            panic!("unexpected send #1");
        };
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
