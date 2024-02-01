use crate::preview2::{bindings::io::poll, WasiView};
use anyhow::{anyhow, Result};
use futures::Future;
use smallvec::{smallvec, SmallVec};
use std::{
    any::Any,
    collections::{hash_map::Entry, HashMap},
    pin::Pin,
    task::{Context, Poll},
};
use wasmtime::component::{Resource, ResourceTable};

/// For all intents and purposes this is just a regular [`Future`], except that
/// the `poll` method has access to the current [`WasiView`].
///
/// There is a blanket implementation of [`WasiFuture`] for all [`Future`]'s,
/// so all regular futures are automatically WASI futures.
pub trait WasiFuture {
    /// See [Future::Output]
    type Output;

    /// See [Future::poll]
    fn poll(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        view: &mut dyn WasiView,
    ) -> Poll<Self::Output>;
}

impl<F: Future> WasiFuture for F {
    type Output = F::Output;

    fn poll(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        _view: &mut dyn WasiView,
    ) -> Poll<Self::Output> {
        Future::poll(self, cx)
    }
}

/// Create a WASI future that wraps a function returning [`Poll`]. Polling the
/// pollable delegates to the wrapped function.
///
/// Similar to [std::future::poll_fn].
pub fn poll_fn<T, F>(poll_fn: F) -> impl WasiFuture<Output = T>
where
    F: FnMut(&mut Context<'_>, &mut dyn WasiView) -> Poll<T> + Send,
{
    struct PollFn<F> {
        poll_fn: F,
    }

    impl<T, F> WasiFuture for PollFn<F>
    where
        F: FnMut(&mut Context<'_>, &mut dyn WasiView) -> Poll<T> + Send,
    {
        type Output = T;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>, view: &mut dyn WasiView) -> Poll<T> {
            // The following code is adapted from [std::future::poll_fn]:

            // SAFETY: We are not moving out of the pinned field.
            (unsafe { &mut self.get_unchecked_mut().poll_fn })(cx, view)
        }
    }

    PollFn { poll_fn }
}

/// A host implementation of the `wasi:io/poll.pollable` contract.
///
/// A pollable is not the same thing as a Rust Future: in WASI, the same pollable
/// may be used to repeatedly check for readiness of a given condition, e.g. if
/// a stream is readable or writable. So, rather than containing a Future, which
/// can only become Ready once, a Pollable contains a way to create a Future in
/// each call to `poll`.
pub trait Pollable: Send + 'static {
    /// Wait for the pollable to be ready.
    ///
    /// This can be called repeatedly as the readiness state of a pollable is
    /// able to change many times during its lifetime.
    ///
    /// # Cancel safety
    /// The implementation must make sure to only await futures that are
    /// cancel-safe, as the returned future will most likely be canceled, even
    /// during normal operation.
    fn ready<'a>(&'a mut self) -> Pin<Box<dyn WasiFuture<Output = ()> + Send + 'a>>;

    /// Check to see if the pollable is currently ready.
    fn is_ready(&mut self, view: &mut dyn WasiView) -> bool {
        let mut future = self.ready();
        let mut cx = Context::from_waker(futures::task::noop_waker_ref());
        future.as_mut().poll(&mut cx, view).is_ready()
    }
}

/// Convenience trait for implementing [`Pollable`] in terms of an `async` method.
/// If you need access to the current [`WasiView`], implement `Pollable` directly instead.
///
/// There is a blanket implementation of `Pollable` for all `PollableAsync`'s,
/// so all `PollableAsync` implementations are automatically `Pollable`.
#[async_trait::async_trait]
pub trait PollableAsync: Send + 'static {
    /// Wait for the pollable to be ready.
    ///
    /// # Cancel safety
    /// The implementation must make sure to only await futures that are
    /// cancel-safe, as the returned future will most likely be canceled, even
    /// during normal operation.
    async fn ready(&mut self);

    /// Check to see if the pollable is currently ready.
    fn is_ready(&mut self) -> bool {
        let mut future = self.ready();
        let mut cx = Context::from_waker(futures::task::noop_waker_ref());
        future.as_mut().poll(&mut cx).is_ready()
    }
}

impl<T: PollableAsync> Pollable for T {
    fn ready<'a>(&'a mut self) -> Pin<Box<dyn WasiFuture<Output = ()> + Send + 'a>> {
        Box::pin(PollableAsync::ready(self))
    }

    fn is_ready(&mut self, _view: &mut dyn WasiView) -> bool {
        PollableAsync::is_ready(self)
    }
}

/// Create a pollable that is always ready.
pub fn ready() -> impl Pollable {
    poll_ready_fn(|_, _| Poll::Ready(()))
}

/// Create a pollable that is never ready.
pub fn pending() -> impl Pollable {
    poll_ready_fn(|_, _| Poll::Pending)
}

/// Create an ad-hoc Pollable implementation from a closure. The closure will be
/// called repeatedly, even after it has already returned [Poll::Ready] before.
pub fn poll_ready_fn<F>(poll_ready_fn: F) -> impl Pollable
where
    F: FnMut(&mut Context<'_>, &mut dyn WasiView) -> Poll<()> + Send + 'static,
{
    struct PollReadyFn<R> {
        poll_ready_fn: R,
    }
    impl<F> Pollable for PollReadyFn<F>
    where
        F: FnMut(&mut Context<'_>, &mut dyn WasiView) -> Poll<()> + Send + 'static,
    {
        fn ready<'a>(&'a mut self) -> Pin<Box<dyn WasiFuture<Output = ()> + Send + 'a>> {
            Box::pin(PollReadyFnFuture { pollable: self })
        }
    }

    struct PollReadyFnFuture<'a, F> {
        pollable: &'a mut PollReadyFn<F>,
    }

    impl<F> WasiFuture for PollReadyFnFuture<'_, F>
    where
        F: FnMut(&mut Context<'_>, &mut dyn WasiView) -> Poll<()> + Send + 'static,
    {
        type Output = ();

        fn poll(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            view: &mut dyn WasiView,
        ) -> Poll<()> {
            (self.pollable.poll_ready_fn)(cx, view)
        }
    }

    PollReadyFn { poll_ready_fn }
}

/// Create a pollable that initially starts out as pending and transitions to
/// ready once the future resolves. After that the pollable will always be ready.
pub fn once<F>(future: F) -> impl Pollable
where
    F: WasiFuture<Output = ()> + Send + 'static,
{
    enum Once<F> {
        Pending(Pin<Box<F>>),
        Ready,
    }
    impl<F> Pollable for Once<F>
    where
        F: WasiFuture<Output = ()> + Send + 'static,
    {
        fn ready<'a>(&'a mut self) -> Pin<Box<dyn WasiFuture<Output = ()> + Send + 'a>> {
            Box::pin(OnceFuture { pollable: self })
        }
    }

    struct OnceFuture<'a, F> {
        pollable: &'a mut Once<F>,
    }

    impl<F> WasiFuture for OnceFuture<'_, F>
    where
        F: WasiFuture<Output = ()> + Send + 'static,
    {
        type Output = ();

        fn poll(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            view: &mut dyn WasiView,
        ) -> Poll<()> {
            let Once::Pending(future) = &mut self.pollable else {
                return Poll::Ready(());
            };

            let Poll::Ready(()) = future.as_mut().poll(cx, view) else {
                return Poll::Pending;
            };

            *self.pollable = Once::Ready;
            Poll::Ready(())
        }
    }

    Once::Pending(Box::pin(future))
}

/// Creates a new handle which is subscribed to the pollable `parent`.
/// The handle will be added as a child of `parent`.
pub fn subscribe<T: Pollable>(
    table: &mut ResourceTable,
    parent: &Resource<T>,
) -> Result<Resource<PollableHandle>> {
    let pollable = PollableHandle::child(parent);
    Ok(table.push_child(pollable, &parent)?)
}

type AsPollableFn = for<'a> fn(&'a mut dyn Any) -> &'a mut dyn Pollable;
type TargetKey = u32;

/// A host representation of the `wasi:io/poll.pollable` resource.
pub enum PollableHandle {
    Own(Box<dyn Pollable>),
    Child {
        parent_key: TargetKey,
        as_pollable: AsPollableFn,
    },
}

impl PollableHandle {
    /// Create a new standalone pollable resource.
    pub fn own(pollable: Box<dyn Pollable>) -> Self {
        Self::Own(pollable)
    }

    /// Create a new pollable handle that delegates to its parent's Pollable
    /// implementation.
    pub fn child<P: Pollable>(parent: &Resource<P>) -> Self {
        Self::Child {
            parent_key: parent.rep(),
            as_pollable: |target| target.downcast_mut::<P>().unwrap(),
        }
    }
}

/// Using the term "target" to mean: where the actual Pollable implementation lives.
/// Sometimes this is the PollableHandle itself, sometimes this is a parent.
struct TargetInfo {
    key: TargetKey,
    as_pollable: AsPollableFn,
}
impl TargetInfo {
    fn gather(table: &ResourceTable, handle: &Resource<PollableHandle>) -> Result<Self> {
        match table.get(&handle)? {
            PollableHandle::Own(_) => Ok(Self {
                key: handle.rep(),
                as_pollable: |target| match target.downcast_mut().unwrap() {
                    PollableHandle::Own(p) => p.as_mut(),
                    PollableHandle::Child { .. } => unreachable!(),
                },
            }),
            PollableHandle::Child {
                parent_key,
                as_pollable,
            } => Ok(Self {
                key: *parent_key,
                as_pollable: *as_pollable,
            }),
        }
    }

    fn lease(self, table: &mut ResourceTable) -> Result<TargetLease> {
        Ok(TargetLease {
            data: table.take_any(self.key)?,
            info: self,
        })
    }
}

struct TargetLease {
    info: TargetInfo,
    data: Box<dyn Any + Send>,
}
impl TargetLease {
    fn take(table: &mut ResourceTable, handle: &Resource<PollableHandle>) -> Result<Self> {
        Ok(TargetInfo::gather(table, handle)?.lease(table)?)
    }

    fn restore(self, table: &mut ResourceTable) -> Result<()> {
        table.restore_any(self.info.key, self.data)?;
        Ok(())
    }

    fn as_pollable(&mut self) -> &mut dyn Pollable {
        (self.info.as_pollable)(self.data.as_mut())
    }
}

#[async_trait::async_trait]
impl<T: WasiView> poll::Host for T {
    async fn poll(&mut self, pollables: Vec<Resource<PollableHandle>>) -> Result<Vec<u32>> {
        if pollables.is_empty() {
            return Err(anyhow!("empty poll list"));
        }

        type ReadylistIndex = u32;
        struct PollEntry {
            lease: TargetLease,
            indexes: SmallVec<[ReadylistIndex; 1]>,
        }

        let table = self.table();

        let mut entries: HashMap<TargetKey, PollEntry> = HashMap::with_capacity(pollables.len());
        for (input_index, pollable) in pollables.into_iter().enumerate() {
            let input_index = ReadylistIndex::try_from(input_index).expect("poll list too big");

            let info = TargetInfo::gather(table, &pollable)?;
            match entries.entry(info.key) {
                Entry::Vacant(v) => {
                    v.insert(PollEntry {
                        lease: info.lease(table)?,
                        indexes: smallvec![input_index],
                    });
                }
                Entry::Occupied(mut o) => {
                    o.get_mut().indexes.push(input_index);
                }
            }
        }

        let self_ref = &mut self;
        let mut futures: Vec<_> = entries
            .values_mut()
            .map(|e| (e.lease.as_pollable().ready(), &e.indexes))
            .collect();

        let results = futures::future::poll_fn(move |cx| {
            let mut results = Vec::new();

            for (future, indexes) in futures.iter_mut() {
                match future.as_mut().poll(cx, *self_ref) {
                    Poll::Ready(()) => results.extend_from_slice(indexes.as_slice()),
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
            entry.lease.restore(table)?;
        }

        Ok(results)
    }
}

#[async_trait::async_trait]
impl<T: WasiView> crate::preview2::bindings::io::poll::HostPollable for T {
    async fn block(&mut self, handle: Resource<PollableHandle>) -> Result<()> {
        let mut lease = TargetLease::take(self.table(), &handle)?;
        {
            let mut future = lease.as_pollable().ready();
            let self_ref = &mut self;
            futures::future::poll_fn(move |cx| future.as_mut().poll(cx, *self_ref)).await;
        }
        lease.restore(self.table())?;
        Ok(())
    }
    async fn ready(&mut self, handle: Resource<PollableHandle>) -> Result<bool> {
        let mut lease = TargetLease::take(self.table(), &handle)?;
        let is_ready = lease.as_pollable().is_ready(self);
        lease.restore(self.table())?;
        Ok(is_ready)
    }
    fn drop(&mut self, handle: Resource<PollableHandle>) -> Result<()> {
        self.table().delete(handle)?;
        Ok(())
    }
}

pub(crate) mod sync {
    use crate::preview2::{bindings::io::poll as async_poll, in_tokio, PollableHandle, WasiView};
    use anyhow::Result;
    use wasmtime::component::Resource;

    impl<T: WasiView> crate::preview2::bindings::sync_io::io::poll::Host for T {
        fn poll(&mut self, pollables: Vec<Resource<PollableHandle>>) -> Result<Vec<u32>> {
            in_tokio(async { async_poll::Host::poll(self, pollables).await })
        }
    }

    impl<T: WasiView> crate::preview2::bindings::sync_io::io::poll::HostPollable for T {
        fn ready(&mut self, pollable: Resource<PollableHandle>) -> Result<bool> {
            in_tokio(async { async_poll::HostPollable::ready(self, pollable).await })
        }
        fn block(&mut self, pollable: Resource<PollableHandle>) -> Result<()> {
            in_tokio(async { async_poll::HostPollable::block(self, pollable).await })
        }
        fn drop(&mut self, pollable: Resource<PollableHandle>) -> Result<()> {
            async_poll::HostPollable::drop(self, pollable)
        }
    }
}
