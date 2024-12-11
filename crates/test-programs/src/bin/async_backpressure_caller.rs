mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "backpressure-caller",
        async: {
            imports: [
                "local:local/run#run"
            ],
            exports: [
                "local:local/run#run"
            ]
        }
    });

    use super::Component;
    export!(Component);
}

use {
    bindings::{
        exports::local::local::run::Guest,
        local::local::{backpressure, run},
    },
    futures::future,
    std::{
        future::Future,
        pin::Pin,
        task::{Context, Poll},
    },
};

struct Component;

impl Guest for Component {
    async fn run() {
        backpressure::set_backpressure(true);

        let mut a = Some(Box::pin(run::run()));
        let mut b = Some(Box::pin(run::run()));
        let mut c = Some(Box::pin(run::run()));

        let mut backpressure_is_set = true;
        future::poll_fn(move |cx| {
            let a_ready = is_ready(cx, &mut a);
            let b_ready = is_ready(cx, &mut b);
            let c_ready = is_ready(cx, &mut c);

            if backpressure_is_set {
                assert!(!a_ready);
                assert!(!b_ready);
                assert!(!c_ready);

                backpressure::set_backpressure(false);
                backpressure_is_set = false;

                Poll::Pending
            } else if a_ready && b_ready && c_ready {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        })
        .await
    }
}

fn is_ready(cx: &mut Context, fut: &mut Option<Pin<Box<impl Future<Output = ()>>>>) -> bool {
    if let Some(v) = fut.as_mut() {
        if v.as_mut().poll(cx).is_ready() {
            *fut = None;
            true
        } else {
            false
        }
    } else {
        true
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
