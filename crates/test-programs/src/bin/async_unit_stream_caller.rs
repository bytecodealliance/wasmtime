mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "unit-stream-caller",
    });

    use super::Component;
    export!(Component);
}

use bindings::{exports::local::local::run::Guest, local::local::unit_stream};

struct Component;

impl Guest for Component {
    async fn run() {
        let count = 42;
        let rx = unit_stream::run(count).await;

        let received = rx.collect().await.len();

        assert_eq!(count, u32::try_from(received).unwrap());
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
