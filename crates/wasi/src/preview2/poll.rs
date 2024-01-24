use crate::preview2::{bindings::io::poll, WasiView};
use anyhow::{anyhow, Result};
use futures::Future;
use smallvec::{smallvec, SmallVec};
use std::{
    collections::{hash_map::Entry, HashMap},
    pin::Pin,
    task::{Context, Poll},
};
use wasmtime::component::{Lease, Resource, ResourceTable};

/// A host representation of the `wasi:io/poll.pollable` resource.
///
/// A pollable is not the same thing as a Rust Future: in WASI, the same pollable
/// may be used to repeatedly check for readiness of a given condition, e.g. if
/// a stream is readable or writable.
pub trait Pollable: Send + 'static {
    /// Check the current readiness status. When the pollable is not ready yet,
    /// `poll_ready` returns [Poll::Pending] and stores a clone of the Waker
    /// copied from the current [Context]. Unlike [std::future::Future::poll],
    /// this method _will_ be called again after it previously returned
    /// [Poll::Ready]. This is normal behavior and should not panic.
    fn poll_ready(&mut self, cx: &mut Context<'_>, view: &mut dyn WasiView) -> Poll<()>;
}

/// Convenience trait for implementing [Pollable] in terms of an `async` method.
/// There is a blanket implementation of [Pollable] for all [Subscribe]'s,
/// so all Subscribe implementations are automatically Pollable.
#[async_trait::async_trait]
pub trait Subscribe: Send + 'static {
    /// Wait for the pollable to be ready.
    ///
    /// # Cancel safety
    /// The implementation must make sure to only await futures that are
    /// cancel-safe, as the returned future will most liekly be canceled, even
    /// during normal operation.
    async fn ready(&mut self);
}

impl<T: Subscribe> Pollable for T {
    fn poll_ready(&mut self, cx: &mut Context<'_>, _view: &mut dyn WasiView) -> Poll<()> {
        self.ready().as_mut().poll(cx)
    }
}

/// Create a pollable that is always ready.
///
/// Similar to [std::future::ready].
pub fn ready() -> impl Pollable {
    struct Ready;
    impl Pollable for Ready {
        fn poll_ready(&mut self, _cx: &mut Context<'_>, _view: &mut dyn WasiView) -> Poll<()> {
            Poll::Ready(())
        }
    }

    Ready
}

/// Create a pollable that is never ready.
///
/// Similar to [std::future::pending].
pub fn pending() -> impl Pollable {
    struct Pending;
    impl Pollable for Pending {
        fn poll_ready(&mut self, _cx: &mut Context<'_>, _view: &mut dyn WasiView) -> Poll<()> {
            Poll::Pending
        }
    }

    Pending
}

/// Create a pollable that initially starts out as pending and transitions to
/// ready once the future resolves. After that the pollable will always be ready.
pub fn once<F>(future: F) -> impl Pollable
where
    F: Future<Output = ()> + Send + 'static,
{
    enum Once<F> {
        Pending(Pin<Box<F>>),
        Ready,
    }
    impl<F: Future<Output = ()> + Send + 'static> Pollable for Once<F> {
        fn poll_ready(&mut self, cx: &mut Context<'_>, _view: &mut dyn WasiView) -> Poll<()> {
            let Self::Pending(future) = self else {
                return Poll::Ready(());
            };

            let Poll::Ready(()) = future.as_mut().poll(cx) else {
                return Poll::Pending;
            };

            *self = Once::Ready;
            Poll::Ready(())
        }
    }

    Once::Pending(Box::pin(future))
}

/// Create a pollable that wraps a function returning [`Poll`]. Polling the
/// pollable delegates to the wrapped function.
///
/// Similar to [std::future::poll_fn].
pub fn poll_ready_fn<F>(poll_ready_fn: F) -> impl Pollable
where
    F: FnMut(&mut Context<'_>, &mut dyn WasiView) -> Poll<()> + Send + 'static,
{
    struct PollReadyFn<F> {
        poll_ready_fn: F,
    }
    impl<F: FnMut(&mut Context<'_>, &mut dyn WasiView) -> Poll<()> + Send + 'static> Pollable
        for PollReadyFn<F>
    {
        fn poll_ready(&mut self, cx: &mut Context<'_>, view: &mut dyn WasiView) -> Poll<()> {
            (self.poll_ready_fn)(cx, view)
        }
    }

    PollReadyFn { poll_ready_fn }
}

/// Creates a `pollable` resource which is subscribed to the provided `resource`.
/// The pollable will be added as a child of `resource`.
pub fn subscribe<T: Pollable>(
    table: &mut ResourceTable,
    resource: &Resource<T>,
) -> Result<Resource<PollableResource>> {
    let resource_rep = resource.rep();
    let pollable = PollableResource::new(Box::new(poll_ready_fn(move |cx, view| {
        let mut parent = view
            .table()
            .take(Resource::<T>::new_borrow(resource_rep))
            .expect("parent to exist");
        let poll = parent.poll_ready(cx, view);
        view.table().restore(parent);
        poll
    })));
    Ok(table.push_child(pollable, &resource)?)
}

/// A host representation of the `wasi:io/poll.pollable` resource.
pub struct PollableResource {
    inner: Box<dyn Pollable>,
}

impl PollableResource {
    pub fn new(pollable: Box<dyn Pollable>) -> Self {
        Self { inner: pollable }
    }
}

#[async_trait::async_trait]
impl<T: WasiView> poll::Host for T {
    async fn poll(&mut self, pollables: Vec<Resource<PollableResource>>) -> Result<Vec<u32>> {
        if pollables.is_empty() {
            return Err(anyhow!("empty poll list"));
        }

        type PollableRep = u32;
        type ReadylistIndex = u32;
        struct PollEntry {
            pollable: Lease<PollableResource>,
            input_indexes: SmallVec<[ReadylistIndex; 1]>,
        }

        let table = self.table();

        let mut entries: HashMap<PollableRep, PollEntry> = HashMap::with_capacity(pollables.len());
        for (input_index, pollable) in pollables.into_iter().enumerate() {
            let input_index = ReadylistIndex::try_from(input_index).expect("poll list too big");
            match entries.entry(pollable.rep()) {
                Entry::Vacant(v) => {
                    v.insert(PollEntry {
                        pollable: table.take(pollable)?,
                        input_indexes: smallvec![input_index],
                    });
                }
                Entry::Occupied(mut o) => {
                    o.get_mut().input_indexes.push(input_index);
                }
            }
        }

        let self_ref = &mut self;
        let entries_ref = &mut entries;

        let results = futures::future::poll_fn(move |cx| {
            let mut results = Vec::new();

            for entry in entries_ref.values_mut() {
                match entry.pollable.inner.poll_ready(cx, *self_ref) {
                    Poll::Ready(()) => results.extend_from_slice(&entry.input_indexes[..]),
                    Poll::Pending => {}
                }
            }
            if results.is_empty() {
                Poll::Pending
            } else {
                Poll::Ready(results)
            }
        })
        .await;

        let table = self.table();
        for entry in entries.into_values() {
            table.restore(entry.pollable);
        }

        Ok(results)
    }
}

#[async_trait::async_trait]
impl<T: WasiView> crate::preview2::bindings::io::poll::HostPollable for T {
    async fn block(&mut self, pollable: Resource<PollableResource>) -> Result<()> {
        let mut pollable = self.table().take(pollable)?;
        futures::future::poll_fn(|cx| pollable.inner.poll_ready(cx, self)).await;
        self.table().restore(pollable);
        Ok(())
    }
    async fn ready(&mut self, pollable: Resource<PollableResource>) -> Result<bool> {
        let mut pollable = self.table().take(pollable)?;
        let mut cx = Context::from_waker(futures::task::noop_waker_ref());
        let poll = pollable.inner.poll_ready(&mut cx, self);
        self.table().restore(pollable);
        Ok(poll.is_ready())
    }
    fn drop(&mut self, pollable: Resource<PollableResource>) -> Result<()> {
        self.table().delete(pollable)?;
        Ok(())
    }
}

pub(crate) mod sync {
    use crate::preview2::{bindings::io::poll as async_poll, in_tokio, PollableResource, WasiView};
    use anyhow::Result;
    use wasmtime::component::Resource;

    impl<T: WasiView> crate::preview2::bindings::sync_io::io::poll::Host for T {
        fn poll(&mut self, pollables: Vec<Resource<PollableResource>>) -> Result<Vec<u32>> {
            in_tokio(async { async_poll::Host::poll(self, pollables).await })
        }
    }

    impl<T: WasiView> crate::preview2::bindings::sync_io::io::poll::HostPollable for T {
        fn ready(&mut self, pollable: Resource<PollableResource>) -> Result<bool> {
            in_tokio(async { async_poll::HostPollable::ready(self, pollable).await })
        }
        fn block(&mut self, pollable: Resource<PollableResource>) -> Result<()> {
            in_tokio(async { async_poll::HostPollable::block(self, pollable).await })
        }
        fn drop(&mut self, pollable: Resource<PollableResource>) -> Result<()> {
            async_poll::HostPollable::drop(self, pollable)
        }
    }
}
