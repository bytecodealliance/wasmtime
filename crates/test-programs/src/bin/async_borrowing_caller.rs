mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "borrowing-caller",
        async: {
            imports: [
                "local:local/borrowing#foo"
            ],
            exports: [
                "local:local/run-bool#run"
            ]
        }
    });

    use super::Component;
    export!(Component);
}

use bindings::{
    exports::local::local::run_bool::Guest,
    local::local::{borrowing::foo, borrowing_types::X},
};

struct Component;

impl Guest for Component {
    async fn run(misbehave: bool) {
        foo(&X::new(), misbehave).await
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
