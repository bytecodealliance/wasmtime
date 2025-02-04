mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "error-context-callee",
        async: {
            exports: [
                "local:local/run#run",
                "local:local/run-result#run-pass",
                "local:local/run-result#run-fail",
            ],
        }
    });

    use super::Component;
    export!(Component);
}
use wit_bindgen_rt::async_support::{error_context_new, ErrorContext};

struct Component;

impl bindings::exports::local::local::run_result::Guest for Component {
    async fn run_fail() -> Result<(), ErrorContext> {
        Err(error_context_new("error".into()))
    }

    async fn run_pass() -> Result<(), ErrorContext> {
        Ok(())
    }
}

impl bindings::exports::local::local::run::Guest for Component {
    async fn run() {}
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
