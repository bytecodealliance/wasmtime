use crate::{bindings::io::poll, WasiView};
use anyhow::{anyhow, Result};
use std::any::Any;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use wasmtime::component::{Resource, ResourceTable};

pub type PollableFuture<'a> = Pin<Box<dyn Future<Output = ()> + Send + 'a>>;
pub type MakeFuture = for<'a> fn(&'a mut dyn Any) -> PollableFuture<'a>;
pub type ClosureFuture = Box<dyn Fn() -> PollableFuture<'static> + Send + 'static>;

/// A host representation of the `wasi:io/poll.pollable` resource.
///
/// A pollable is not the same thing as a Rust Future: the same pollable may be used to
/// repeatedly check for readiness of a given condition, e.g. if a stream is readable
/// or writable. So, rather than containing a Future, which can only become Ready once, a
/// Pollable contains a way to create a Future in each call to `poll`.
pub struct Pollable {
    index: u32,
    make_future: MakeFuture,
    remove_index_on_delete: Option<fn(&mut ResourceTable, u32) -> Result<()>>,
}

#[async_trait::async_trait]
pub trait Subscribe: Send + 'static {
    async fn ready(&mut self);
}

/// Creates a `pollable` resource which is susbcribed to the provided
/// `resource`.
///
/// If `resource` is an owned resource then it will be deleted when the returned
/// resource is deleted. Otherwise the returned resource is considered a "child"
/// of the given `resource` which means that the given resource cannot be
/// deleted while the `pollable` is still alive.
pub fn subscribe<T>(table: &mut ResourceTable, resource: Resource<T>) -> Result<Resource<Pollable>>
where
    T: Subscribe,
{
    fn make_future<'a, T>(stream: &'a mut dyn Any) -> PollableFuture<'a>
    where
        T: Subscribe,
    {
        stream.downcast_mut::<T>().unwrap().ready()
    }

    let pollable = Pollable {
        index: resource.rep(),
        remove_index_on_delete: if resource.owned() {
            Some(|table, idx| {
                let resource = Resource::<T>::new_own(idx);
                table.delete(resource)?;
                Ok(())
            })
        } else {
            None
        },
        make_future: make_future::<T>,
    };

    Ok(table.push_child(pollable, &resource)?)
}

#[async_trait::async_trait]
impl<T: WasiView> poll::Host for T {
    async fn poll(&mut self, pollables: Vec<Resource<Pollable>>) -> Result<Vec<u32>> {
        type ReadylistIndex = u32;

        if pollables.is_empty() {
            return Err(anyhow!("empty poll list"));
        }

        let table = self.table();

        let mut table_futures: HashMap<u32, (MakeFuture, Vec<ReadylistIndex>)> = HashMap::new();

        for (ix, p) in pollables.iter().enumerate() {
            let ix: u32 = ix.try_into()?;

            let pollable = table.get(p)?;
            let (_, list) = table_futures
                .entry(pollable.index)
                .or_insert((pollable.make_future, Vec::new()));
            list.push(ix);
        }

        let mut futures: Vec<(PollableFuture<'_>, Vec<ReadylistIndex>)> = Vec::new();
        for (entry, (make_future, readylist_indices)) in table.iter_entries(table_futures) {
            let entry = entry?;
            futures.push((make_future(entry), readylist_indices));
        }

        struct PollList<'a> {
            futures: Vec<(PollableFuture<'a>, Vec<ReadylistIndex>)>,
        }
        impl<'a> Future for PollList<'a> {
            type Output = Vec<u32>;

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                let mut any_ready = false;
                let mut results = Vec::new();
                for (fut, readylist_indicies) in self.futures.iter_mut() {
                    match fut.as_mut().poll(cx) {
                        Poll::Ready(()) => {
                            results.extend_from_slice(readylist_indicies);
                            any_ready = true;
                        }
                        Poll::Pending => {}
                    }
                }
                if any_ready {
                    Poll::Ready(results)
                } else {
                    Poll::Pending
                }
            }
        }

        Ok(PollList { futures }.await)
    }
}

#[async_trait::async_trait]
impl<T: WasiView> crate::bindings::io::poll::HostPollable for T {
    async fn block(&mut self, pollable: Resource<Pollable>) -> Result<()> {
        let table = self.table();
        let pollable = table.get(&pollable)?;
        let ready = (pollable.make_future)(table.get_any_mut(pollable.index)?);
        ready.await;
        Ok(())
    }
    async fn ready(&mut self, pollable: Resource<Pollable>) -> Result<bool> {
        let table = self.table();
        let pollable = table.get(&pollable)?;
        let ready = (pollable.make_future)(table.get_any_mut(pollable.index)?);
        futures::pin_mut!(ready);
        Ok(matches!(
            futures::future::poll_immediate(ready).await,
            Some(())
        ))
    }
    fn drop(&mut self, pollable: Resource<Pollable>) -> Result<()> {
        let pollable = self.table().delete(pollable)?;
        if let Some(delete) = pollable.remove_index_on_delete {
            delete(self.table(), pollable.index)?;
        }
        Ok(())
    }
}

pub mod sync {
    use crate::{
        bindings::io::poll as async_poll,
        bindings::sync_io::io::poll::{self, Pollable},
        in_tokio, WasiView,
    };
    use anyhow::Result;
    use wasmtime::component::Resource;

    impl<T: WasiView> poll::Host for T {
        fn poll(&mut self, pollables: Vec<Resource<Pollable>>) -> Result<Vec<u32>> {
            in_tokio(async { async_poll::Host::poll(self, pollables).await })
        }
    }

    impl<T: WasiView> crate::bindings::sync_io::io::poll::HostPollable for T {
        fn ready(&mut self, pollable: Resource<Pollable>) -> Result<bool> {
            in_tokio(async { async_poll::HostPollable::ready(self, pollable).await })
        }
        fn block(&mut self, pollable: Resource<Pollable>) -> Result<()> {
            in_tokio(async { async_poll::HostPollable::block(self, pollable).await })
        }
        fn drop(&mut self, pollable: Resource<Pollable>) -> Result<()> {
            async_poll::HostPollable::drop(self, pollable)
        }
    }
}
