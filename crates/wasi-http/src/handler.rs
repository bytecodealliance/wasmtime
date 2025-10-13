//! Provides utilities useful for dispatching incoming HTTP requests
//! `wasi:http/handler` guest instances.

#[cfg(feature = "p3")]
use crate::p3;
use anyhow::{Result, anyhow};
use futures::{
    future::FutureExt,
    stream::{FuturesUnordered, StreamExt},
};
use std::collections::VecDeque;
use std::future;
use std::pin::{Pin, pin};
use std::sync::{
    Arc, Mutex,
    atomic::{
        AtomicBool, AtomicU64, AtomicUsize,
        Ordering::{Relaxed, SeqCst},
    },
};
use std::task::Poll;
use std::time::Duration;
use tokio::sync::Notify;
use wasmtime::AsContextMut;
use wasmtime::component::Accessor;
use wasmtime::{Store, StoreContextMut};

/// Alternative p2 bindings generated with `exports: { default: async | store }`
/// so we can use `TypedFunc::call_concurrent` with both p2 and p3 instances.
pub mod p2 {
    #[expect(missing_docs, reason = "bindgen-generated code")]
    pub mod bindings {
        wasmtime::component::bindgen!({
            path: "wit",
            world: "wasi:http/proxy",
            imports: { default: tracing },
            exports: { default: async | store },
            require_store_data_send: true,
            with: {
                // http is in this crate
                "wasi:http": crate::bindings::http,
                // Upstream package dependencies
                "wasi:io": wasmtime_wasi::p2::bindings::io,
            }
        });

        pub use wasi::*;
    }
}

/// Represents either a `wasi:http/incoming-handler@0.2.x` or
/// `wasi:http/handler@0.3.x` pre-instance.
pub enum ProxyPre<T: 'static> {
    /// A `wasi:http/incoming-handler@0.2.x` pre-instance.
    P2(p2::bindings::ProxyPre<T>),
    /// A `wasi:http/handler@0.3.x` pre-instance.
    #[cfg(feature = "p3")]
    P3(p3::bindings::ProxyPre<T>),
}

impl<T: 'static> ProxyPre<T> {
    async fn instantiate_async(&self, store: impl AsContextMut<Data = T>) -> Result<Proxy>
    where
        T: Send,
    {
        Ok(match self {
            Self::P2(pre) => Proxy::P2(pre.instantiate_async(store).await?),
            #[cfg(feature = "p3")]
            Self::P3(pre) => Proxy::P3(pre.instantiate_async(store).await?),
        })
    }
}

/// Represents either a `wasi:http/incoming-handler@0.2.x` or
/// `wasi:http/handler@0.3.x` instance.
pub enum Proxy {
    /// A `wasi:http/incoming-handler@0.2.x` instance.
    P2(p2::bindings::Proxy),
    /// A `wasi:http/handler@0.3.x` instance.
    #[cfg(feature = "p3")]
    P3(p3::bindings::Proxy),
}

/// Represents a task to run using a `wasi:http/incoming-handler@0.2.x` or
/// `wasi:http/handler@0.3.x` instance.
pub type TaskFn<T> = Box<
    dyn for<'a> FnOnce(&'a Accessor<T>, &'a Proxy) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>
        + Send,
>;

/// Async MPMC channel where each item is delivered to at most one consumer.
struct Queue<T> {
    queue: Mutex<VecDeque<T>>,
    notify: Notify,
}

impl<T> Default for Queue<T> {
    fn default() -> Self {
        Self {
            queue: Default::default(),
            notify: Default::default(),
        }
    }
}

impl<T> Queue<T> {
    fn is_empty(&self) -> bool {
        self.queue.lock().unwrap().is_empty()
    }

    fn push(&self, item: T) {
        self.queue.lock().unwrap().push_back(item);
        self.notify.notify_one();
    }

    fn try_pop(&self) -> Option<T> {
        self.queue.lock().unwrap().pop_front()
    }

    async fn pop(&self) -> T {
        // This code comes from the Unbound MPMC Channel example in [the
        // `tokio::sync::Notify`
        // docs](https://docs.rs/tokio/latest/tokio/sync/struct.Notify.html).

        let mut notified = pin!(self.notify.notified());

        loop {
            notified.as_mut().enable();
            if let Some(item) = self.try_pop() {
                return item;
            }
            notified.as_mut().await;
            notified.set(self.notify.notified());
        }
    }
}

/// Bundles a [`Store`] with a callback to write a profile (if configured).
pub struct StoreBundle<T: 'static> {
    /// The [`Store`] to use to handle requests.
    pub store: Store<T>,
    /// Callback to write a profile (if enabled) once all requests have been
    /// handled.
    pub write_profile: Box<dyn FnOnce(StoreContextMut<T>) + Send>,
}

/// Represents the application-specific state of a web server.
pub trait HandlerState: 'static + Sync + Send {
    /// The type of the associated data for [`Store`]s created using
    /// [`new_store`].
    type StoreData;

    /// Create a new [`Store`] for handling one or more requests.
    fn new_store(&self) -> Result<StoreBundle<Self::StoreData>>;

    /// Maximum time allowed to handle a request.
    ///
    /// In practice, a guest may be allowed to run up to 2x this time in the
    /// case of instance reuse to avoid penalizing concurrent requests being
    /// handled by the same instance.
    fn request_timeout(&self) -> Duration;

    /// Maximum time to keep an idle instance around before dropping it.
    fn idle_instance_timeout(&self) -> Duration;

    /// Maximum number of requests to handle using a single instance before
    /// dropping it.
    fn max_instance_reuse_count(&self) -> usize;

    /// Maximum number of requests to handle concurrently using a single
    /// instance.
    fn max_instance_concurrent_reuse_count(&self) -> usize;
}

struct ProxyHandlerInner<S, T: 'static> {
    state: S,
    instance_pre: ProxyPre<T>,
    next_id: AtomicU64,
    task_queue: Queue<TaskFn<T>>,
    worker_count: AtomicUsize,
}

struct Worker<S, T: 'static>
where
    T: Send,
    S: HandlerState<StoreData = T>,
{
    handler: ProxyHandler<S, T>,
    available: bool,
}

impl<S, T> Worker<S, T>
where
    T: Send,
    S: HandlerState<StoreData = T>,
{
    fn set_available(&mut self, available: bool) {
        if available != self.available {
            self.available = available;
            if available {
                self.handler.0.worker_count.fetch_add(1, SeqCst);
            } else {
                let count = self.handler.0.worker_count.fetch_sub(1, SeqCst);
                // This addresses what would otherwise be a race condition in
                // `ProxyHandler::spawn` where it only starts a worker if the
                // available worker count is zero.  If we decrement the count to
                // zero right after `ProxyHandler::spawn` checks it, then no
                // worker will be started; thus it becomes our responsibility to
                // start a worker here instead.
                if count == 1 && !self.handler.0.task_queue.is_empty() {
                    self.handler.start_worker();
                }
            }
        }
    }

    async fn run(mut self) -> Result<()> {
        let handler = &self.handler.0;

        let StoreBundle {
            mut store,
            write_profile,
        } = handler.state.new_store()?;

        let request_timeout = handler.state.request_timeout();
        let idle_instance_timeout = handler.state.idle_instance_timeout();
        let max_instance_reuse_count = handler.state.max_instance_reuse_count();
        let max_instance_concurrent_reuse_count =
            handler.state.max_instance_concurrent_reuse_count();

        let proxy = &handler.instance_pre.instantiate_async(&mut store).await?;
        let accept_concurrent = AtomicBool::new(true);

        let mut future = pin!(store.run_concurrent(async |accessor| {
            let mut reuse_count = 0;
            let mut timed_out = false;
            let mut futures = FuturesUnordered::new();
            let handler = self.handler.clone();
            while !(futures.is_empty() && reuse_count >= max_instance_reuse_count) {
                let new_task = {
                    let future_count = futures.len();
                    let mut next_future = pin!(async {
                        if futures.is_empty() {
                            future::pending().await
                        } else {
                            futures.next().await
                        }
                    });
                    let mut next_task = pin!(tokio::time::timeout(
                        if future_count == 0 {
                            idle_instance_timeout
                        } else {
                            Duration::MAX
                        },
                        handler.0.task_queue.pop()
                    ));
                    // Poll any existing tasks, and if they're all `Pending`
                    // _and_ we haven't reached any reuse limits yet, poll for a
                    // new task from the queue.
                    future::poll_fn(|cx| match next_future.as_mut().poll(cx) {
                        Poll::Pending => {
                            // Note that `Pending` here doesn't necessarily mean
                            // all tasks are blocked on I/O.  They might simply
                            // be waiting for some deferred work to be done by
                            // the next turn of the
                            // `StoreContextMut::run_concurrent` event loop.
                            // Therefore, we check `accept_concurrent` here and
                            // only advertise we have capacity for another task
                            // if either we have no tasks at all or all our
                            // tasks really are blocked on I/O.
                            self.set_available(
                                reuse_count < max_instance_reuse_count
                                    && future_count < max_instance_concurrent_reuse_count
                                    && (future_count == 0 || accept_concurrent.load(Relaxed)),
                            );

                            if self.available {
                                next_task.as_mut().poll(cx).map(Some)
                            } else {
                                Poll::Pending
                            }
                        }
                        Poll::Ready(Some(Ok(()))) => {
                            // Task completed; carry on!
                            Poll::Ready(None)
                        }
                        Poll::Ready(Some(Err(_))) => {
                            // Task timed out; stop accepting new tasks, but
                            // continue polling until any other, in-progress
                            // tasks until they have either finished or timed
                            // out.  This effectively kicks off a "graceful
                            // shutdown" of the worker, allowing any other
                            // concurrent tasks time to finish before we drop
                            // the instance.
                            //
                            // TODO: We should also send a cancel request to the
                            // timed-out task to give it a chance to shut down
                            // gracefully (and delay dropping the instance for a
                            // reasonable amount of time), but as of this
                            // writing Wasmtime does not yet provide an API for
                            // doing that.  See issue #11833.
                            timed_out = true;
                            reuse_count = max_instance_reuse_count;
                            Poll::Ready(None)
                        }
                        Poll::Ready(None) => unreachable!(),
                    })
                    .await
                };

                match new_task {
                    Some(Ok(task)) => {
                        // Set `accept_concurrent` to false, conservatively
                        // assuming that the new task will be CPU-bound, at
                        // least to begin with.  Only once the
                        // `StoreContextMut::run_concurrent` event loop returns
                        // `Pending` will we set `accept_concurrent` back to
                        // true and consider accepting more tasks.
                        //
                        // This approach avoids taking on more than one
                        // CPU-bound task at a time, which would hurt
                        // throughput vs. leaving the additional tasks
                        // for other workers to handle.
                        accept_concurrent.store(false, Relaxed);
                        reuse_count += 1;

                        futures.push(tokio::time::timeout(request_timeout, async move {
                            (task)(accessor, proxy).await
                        }));
                    }
                    Some(Err(_)) => break,
                    None => {}
                }
            }

            accessor.with(|mut access| write_profile(access.as_context_mut()));

            if timed_out {
                Err(anyhow!("guest timed out"))
            } else {
                anyhow::Ok(())
            }
        }));

        future::poll_fn(|cx| {
            let poll = future.as_mut().poll(cx);
            // If the future returns `Pending`, it's either because it's idle
            // (in which case it can definitely accept a new task) or because
            // all its tasks are awaiting I/O, in which case it may have
            // capacity for additional tasks to run concurrently.  Here we set
            // `accept_concurrent` to true and, if it wasn't already true
            // before, poll the future one more time so it can ask for another
            // task if appropriate.
            if poll.is_pending() && !accept_concurrent.swap(true, Relaxed) {
                future.as_mut().poll(cx)
            } else {
                poll
            }
        })
        .await?
    }
}

impl<S, T> Drop for Worker<S, T>
where
    T: Send,
    S: HandlerState<StoreData = T>,
{
    fn drop(&mut self) {
        self.set_available(false);
    }
}

/// Represents the state of a web server.
///
/// Note that this supports optional instance reuse, enabled when
/// `S::max_instance_reuse_count()` returns a number greater than one.  See
/// [`Self::push`] for details.
pub struct ProxyHandler<S, T: 'static>(Arc<ProxyHandlerInner<S, T>>);

impl<S, T> Clone for ProxyHandler<S, T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<S, T> ProxyHandler<S, T> {
    /// Create a new `ProxyHandler` with the specified application state and
    /// pre-instance.
    pub fn new(state: S, instance_pre: ProxyPre<T>) -> Self {
        Self(Arc::new(ProxyHandlerInner {
            state,
            instance_pre,
            next_id: AtomicU64::from(0),
            task_queue: Default::default(),
            worker_count: AtomicUsize::from(0),
        }))
    }

    /// Push a task to the task queue for this handler.
    ///
    /// This will either spawn a new background worker to run the task or
    /// deliver it to an already-running worker.
    pub fn spawn(&self, task: TaskFn<T>)
    where
        T: Send,
        S: HandlerState<StoreData = T>,
    {
        match self.0.state.max_instance_reuse_count() {
            0 => panic!("`max_instance_reuse_count` must be at least 1"),
            1 => {
                // Use a simplified path when instance reuse is disabled.
                //
                // This provides somewhat (e.g. ~20%) better throughput as
                // measured by `wasmtime-serve-rps.sh` than the
                // task-queue-and-workers approach below, so probably worth the
                // slight code duplication.  TODO: Can we narrow the gap and
                // remove this path?
                let handler = self.clone();

                tokio::task::spawn(
                    async move {
                        let StoreBundle {
                            mut store,
                            write_profile,
                        } = handler.0.state.new_store()?;

                        let proxy = &handler.0.instance_pre.instantiate_async(&mut store).await?;

                        let result = store
                            .run_concurrent(async |accessor| {
                                tokio::time::timeout(handler.0.state.request_timeout(), async {
                                    (task)(accessor, proxy).await
                                })
                                .await
                                .map_err(|_| anyhow!("guest timed out"))
                            })
                            .await;

                        write_profile(store.as_context_mut());

                        result?
                    }
                    .map(|result| {
                        if let Err(error) = result {
                            eprintln!("worker error: {error:?}");
                        }
                    }),
                );
            }
            _ => {
                self.0.task_queue.push(task);
                // Start a new worker to handle the task if there aren't already
                // any available.  See also `Worker::set_available` for what
                // happens if the available worker count goes to zero right
                // after we check it here, and note that we only check the count
                // _after_ we've pushed the task to the queue.
                if self.0.worker_count.load(SeqCst) == 0 {
                    self.start_worker();
                }
            }
        }
    }

    /// Generate a unique request ID.
    pub fn next_req_id(&self) -> u64 {
        self.0.next_id.fetch_add(1, Relaxed)
    }

    /// Return a reference to the application state.
    pub fn state(&self) -> &S {
        &self.0.state
    }

    /// Return a reference to the pre-instance.
    pub fn instance_pre(&self) -> &ProxyPre<T> {
        &self.0.instance_pre
    }

    fn start_worker(&self)
    where
        T: Send,
        S: HandlerState<StoreData = T>,
    {
        tokio::spawn(
            Worker {
                handler: self.clone(),
                available: false,
            }
            .run()
            .map(|result| {
                if let Err(error) = result {
                    eprintln!("worker error: {error:?}");
                }
            }),
        );
    }
}
