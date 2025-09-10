mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "backpressure-callee",
    });

    use super::Component;
    export!(Component);
}

use bindings::exports::local::local::{backpressure::Guest as Backpressure, run::Guest as Run};

struct Component;

impl Run for Component {
    async fn run() {
        // do nothing
    }
}

impl Backpressure for Component {
    fn set_backpressure(enabled: bool) {
        #[expect(deprecated, reason = "will replace with backpressure.inc/dec soon")]
        wit_bindgen::backpressure_set(enabled);
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
