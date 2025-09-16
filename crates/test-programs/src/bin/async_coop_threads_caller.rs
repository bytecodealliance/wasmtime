mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "coop-threads-caller",
    });

    use super::Component;
    export!(Component);
}

use {
    crate::bindings::local::local::coop::get_index,
    bindings::exports::local::local::run::Guest as Run,
};

struct Component;

impl Run for Component {
    async fn run() {
        assert_eq!(get_index().await, 10)
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
