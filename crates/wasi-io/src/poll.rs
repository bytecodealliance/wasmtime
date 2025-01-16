use crate::bindings::wasi::io::poll;
use crate::view::{IoImpl, IoView};
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

/// A trait used internally within a [`Pollable`] to create a `pollable`
/// resource in `wasi:io/poll`.
///
/// This trait is the internal implementation detail of any pollable resource in
/// this crate's implementation of WASI. The `ready` function is an `async fn`
/// which resolves when the implementation is ready. Using native `async` Rust
/// enables this type's readiness to compose with other types' readiness
/// throughout the WASI implementation.
///
/// This trait is used in conjunction with [`subscribe`] to create a `pollable`
/// resource.
///
/// # Example
///
/// This is a simple example of creating a `Pollable` resource from a few
/// parameters.
///
/// ```
/// use tokio::time::{self, Duration, Instant};
/// use wasmtime_wasi::{IoView, Subscribe, subscribe, Pollable, async_trait};
/// use wasmtime::component::Resource;
/// use wasmtime::Result;
///
/// fn sleep(cx: &mut dyn IoView, dur: Duration) -> Result<Resource<Pollable>> {
///     let end = Instant::now() + dur;
///     let sleep = MySleep { end };
///     let sleep_resource = cx.table().push(sleep)?;
///     subscribe(cx.table(), sleep_resource)
/// }
///
/// struct MySleep {
///     end: Instant,
/// }
///
/// #[async_trait]
/// impl Subscribe for MySleep {
///     async fn ready(&mut self) {
///         tokio::time::sleep_until(self.end).await;
///     }
/// }
/// ```
#[async_trait::async_trait]
pub trait Subscribe: Send + 'static {
    /// An asynchronous function which resolves when this object's readiness
    /// operation is ready.
    ///
    /// This function is invoked as part of `poll` in `wasi:io/poll`. The
    /// meaning of when this function Returns depends on what object this
    /// [`Subscribe`] is attached to. When the returned future resolves then the
    /// corresponding call to `wasi:io/poll` will return.
    ///
    /// Note that this method does not return an error. Returning an error
    /// should be done through accessors on the object that this `pollable` is
    /// connected to. The call to `wasi:io/poll` itself does not return errors,
    /// only a list of ready objects.
    async fn ready(&mut self);
}

/// Creates a `pollable` resource which is subscribed to the provided
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

impl<T> poll::Host for IoImpl<T>
where
    T: IoView,
{
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

impl<T> crate::bindings::wasi::io::poll::HostPollable for IoImpl<T>
where
    T: IoView,
{
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
