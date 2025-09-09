mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "backpressure-caller",
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
        test(
            || backpressure::set_backpressure(true),
            || backpressure::set_backpressure(false),
        )
        .await;
        test(
            || backpressure::inc_backpressure(),
            || backpressure::dec_backpressure(),
        )
        .await;
    }
}

async fn test(enable: impl Fn(), disable: impl Fn()) {
    enable();

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

            disable();
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
