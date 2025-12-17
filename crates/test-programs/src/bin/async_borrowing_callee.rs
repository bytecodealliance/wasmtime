mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "borrowing-callee",
    });

    use super::Component;
    export!(Component);
}

use bindings::{
    exports::local::local::{borrowing::Guest as Borrowing, run_bool::Guest as RunBool},
    local::local::borrowing_types::X,
};

struct Component;

impl Borrowing for Component {
    async fn foo(x: &X, misbehave: bool) {
        let handle = x.handle();
        wit_bindgen::spawn(async move {
            if misbehave {
                unsafe { X::from_handle(handle) }.foo();
            }
        });
        x.foo();
    }
}

impl RunBool for Component {
    async fn run(misbehave: bool) {
        Self::foo(&X::new(), misbehave).await
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
