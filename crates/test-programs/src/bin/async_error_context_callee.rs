mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "error-context-callee",
    });

    use super::Component;
    export!(Component);
}
use wit_bindgen_rt::async_support::ErrorContext;

struct Component;

impl bindings::exports::local::local::run_result::Guest for Component {
    async fn run_fail() -> Result<(), ErrorContext> {
        Err(ErrorContext::new("error"))
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
