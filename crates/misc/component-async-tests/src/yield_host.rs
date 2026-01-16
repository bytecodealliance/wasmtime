use super::Ctx;
use futures::future;
use std::ops::DerefMut;
use std::sync::{Arc, Mutex};
use std::task::{Poll, Waker};
use wasmtime::component::{Accessor, Resource};

pub mod bindings {
    wasmtime::component::bindgen!({
        path: "wit",
        world: "yield-host",
        imports: { default: trappable },
        with: {
            "local:local/ready.thing": super::Thing,
        },
    });
}

#[derive(Default)]
pub struct Thing {
    wakers: Arc<Mutex<Option<Vec<Waker>>>>,
}

impl bindings::local::local::continue_::Host for Ctx {
    fn set_continue(&mut self, v: bool) -> wasmtime::Result<()> {
        self.continue_ = v;
        Ok(())
    }

    fn get_continue(&mut self) -> wasmtime::Result<bool> {
        Ok(self.continue_)
    }
}

impl bindings::local::local::ready::HostThing for Ctx {
    fn new(&mut self) -> wasmtime::Result<Resource<Thing>> {
        Ok(self.table.push(Thing::default())?)
    }

    fn set_ready(&mut self, thing: Resource<Thing>, ready: bool) -> wasmtime::Result<()> {
        let thing = self.table.get(&thing)?;
        let mut wakers = thing.wakers.lock().unwrap();
        if ready {
            if let Some(wakers) = wakers.take() {
                for waker in wakers {
                    waker.wake();
                }
            }
        } else if wakers.is_none() {
            *wakers = Some(Vec::new());
        }
        Ok(())
    }

    fn drop(&mut self, thing: Resource<Thing>) -> wasmtime::Result<()> {
        self.table.delete(thing)?;
        Ok(())
    }
}

impl bindings::local::local::ready::HostThingWithStore for Ctx {
    async fn when_ready<T>(
        accessor: &Accessor<T, Self>,
        thing: Resource<Thing>,
    ) -> wasmtime::Result<()> {
        let wakers = accessor.with(|mut view| {
            Ok::<_, wasmtime::Error>(view.get().table.get(&thing)?.wakers.clone())
        })?;

        future::poll_fn(move |cx| {
            let mut wakers = wakers.lock().unwrap();
            if let Some(wakers) = wakers.deref_mut() {
                wakers.push(cx.waker().clone());
                Poll::Pending
            } else {
                Poll::Ready(())
            }
        })
        .await;

        Ok(())
    }
}

impl bindings::local::local::ready::Host for Ctx {}
