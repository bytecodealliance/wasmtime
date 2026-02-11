mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "yield-post-return-caller",
    });

    use super::Component;
    export!(Component);
}

use {
    bindings::{
        exports::local::local::yield_post_return::Guest,
        local::local::{yield_, yield_post_return},
    },
    wit_bindgen::rt::async_support,
};

struct Component;

impl Guest for Component {
    async fn run(times: u64) {
        // Spawn a task to run post-return and otherwise return immediately.
        async_support::spawn(async move {
            // Create a couple of subtasks which will also return immediately
            // and yield post-return.  These will not have completed once we
            // exit and thus will be reparented to the caller.
            yield_post_return::run(times * 2).await;
            yield_post_return::run(times * 2).await;
            // Yield for as long as requested:
            yield_::yield_times(times).await;
        });
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
