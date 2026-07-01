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
        if enabled {
            wit_bindgen::backpressure_inc();
        } else {
            wit_bindgen::backpressure_dec();
        }
    }
    fn inc_backpressure() {
        wit_bindgen::backpressure_inc();
    }
    fn dec_backpressure() {
        wit_bindgen::backpressure_dec();
    }
    async fn inc_then_later_dec_backpressure() {
        wit_bindgen::backpressure_inc();

        wit_bindgen::spawn_local(async {
            for _ in 0..10 {
                wit_bindgen::yield_async().await;
            }
            wit_bindgen::backpressure_dec();
        });
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
