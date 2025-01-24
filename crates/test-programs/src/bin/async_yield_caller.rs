mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "yield-caller",
        async: {
            imports: [
                "local:local/ready#when-ready",
                "local:local/run#run",
            ],
            exports: [
                "local:local/run#run",
            ],
        }
    });

    use super::Component;
    export!(Component);
}

use {
    bindings::{
        exports::local::local::run::Guest,
        local::local::{continue_, ready, run},
    },
    futures::future,
    std::{future::Future, task::Poll},
};

struct Component;

impl Guest for Component {
    async fn run() {
        ready::set_ready(false);
        continue_::set_continue(true);

        let mut ready = Some(Box::pin(ready::when_ready()));
        let mut run = Some(Box::pin(run::run()));
        future::poll_fn(move |cx| {
            let ready_poll = ready.as_mut().map(|v| v.as_mut().poll(cx));
            ready::set_ready(true);
            let run_poll = run.as_mut().map(|v| v.as_mut().poll(cx));

            match (run_poll, ready_poll) {
                (None | Some(Poll::Ready(())), None | Some(Poll::Ready(()))) => {
                    return Poll::Ready(());
                }
                (Some(Poll::Ready(())), _) => run = None,
                (_, Some(Poll::Ready(()))) => {
                    ready = None;
                    continue_::set_continue(false);
                }
                _ => {}
            }

            Poll::Pending
        })
        .await
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
