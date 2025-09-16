mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "coop-threads-callee",
    });

    use super::Component;
    export!(Component);
}

use {
    bindings::exports::local::local::coop::Guest as CoopThreads,
    test_programs::async_::thread_index,
};

struct Component;

impl CoopThreads for Component {
    async fn get_index() -> u32 {
        unsafe { thread_index() }
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
