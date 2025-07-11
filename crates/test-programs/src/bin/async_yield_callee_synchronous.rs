mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "yield-callee",
        async: ["-local:local/run#[async]run"],
    });

    use super::Component;
    export!(Component);
}

use {
    bindings::{exports::local::local::run::Guest, local::local::continue_},
    wit_bindgen_rt::async_support,
};

struct Component;

impl Guest for Component {
    fn run() {
        while continue_::get_continue() {
            assert!(async_support::yield_blocking());
        }
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
