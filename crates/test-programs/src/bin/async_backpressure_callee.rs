mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "backpressure-callee",
        async: {
            exports: [
                "local:local/run#run"
            ]
        }
    });

    use super::Component;
    export!(Component);
}

use {
    bindings::exports::local::local::{backpressure::Guest as Backpressure, run::Guest as Run},
    wit_bindgen_rt::async_support,
};

struct Component;

impl Run for Component {
    async fn run() {
        // do nothing
    }
}

impl Backpressure for Component {
    fn set_backpressure(enabled: bool) {
        async_support::task_backpressure(enabled);
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
