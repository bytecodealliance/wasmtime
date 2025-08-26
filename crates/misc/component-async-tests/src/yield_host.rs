use super::Ctx;
use futures::future;
use std::ops::DerefMut;
use std::task::Poll;
use wasmtime::component::Accessor;

pub mod bindings {
    wasmtime::component::bindgen!({
        path: "wit",
        world: "yield-host",
    });
}

impl bindings::local::local::continue_::Host for Ctx {
    fn set_continue(&mut self, v: bool) {
        self.continue_ = v;
    }

    fn get_continue(&mut self) -> bool {
        self.continue_
    }
}

impl bindings::local::local::ready::Host for Ctx {
    fn set_ready(&mut self, ready: bool) {
        let mut wakers = self.wakers.lock().unwrap();
        if ready {
            if let Some(wakers) = wakers.take() {
                for waker in wakers {
                    waker.wake();
                }
            }
        } else if wakers.is_none() {
            *wakers = Some(Vec::new());
        }
    }
}

impl bindings::local::local::ready::HostWithStore for Ctx {
    async fn when_ready<T>(accessor: &Accessor<T, Self>) {
        let wakers = accessor.with(|mut view| view.get().wakers.clone());
        future::poll_fn(move |cx| {
            let mut wakers = wakers.lock().unwrap();
            if let Some(wakers) = wakers.deref_mut() {
                wakers.push(cx.waker().clone());
                Poll::Pending
            } else {
                Poll::Ready(())
            }
        })
        .await
    }
}
