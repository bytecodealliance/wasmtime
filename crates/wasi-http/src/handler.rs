//! Provides utilities useful for dispatching incoming HTTP requests
//! `wasi:http/handler` guest instances.

use crate::bindings;
#[cfg(feature = "p3")]
use crate::p3;
use anyhow::Result;
#[cfg(feature = "p3")]
use anyhow::{anyhow, bail};
#[cfg(feature = "p3")]
use futures::{
    future::FutureExt,
    stream::{FuturesUnordered, StreamExt},
};
#[cfg(feature = "p3")]
use std::collections::VecDeque;
#[cfg(feature = "p3")]
use std::future;
#[cfg(feature = "p3")]
use std::pin::{Pin, pin};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering::Relaxed},
};
#[cfg(feature = "p3")]
use std::sync::{
    Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst},
};
#[cfg(feature = "p3")]
use std::task::Poll;
use std::time::Duration;
#[cfg(feature = "p3")]
use tokio::sync::Notify;
#[cfg(feature = "p3")]
use wasmtime::AsContextMut;
#[cfg(feature = "p3")]
use wasmtime::component::Accessor;
use wasmtime::{Store, StoreContextMut};

/// Represents either a `wasi:http/incoming-handler@0.2.x` or
/// `wasi:http/handler@0.3.x` pre-instance.
pub enum ProxyPre<T: 'static> {
    /// A `wasi:http/incoming-handler@0.2.x` pre-instance.
    P2(bindings::ProxyPre<T>),
    /// A `wasi:http/handler@0.3.x` pre-instance.
    #[cfg(feature = "p3")]
    P3(p3::bindings::ProxyPre<T>),
}

/// Represents a task to run using a `wasi:http/handler@0.3.x` instance.
#[cfg(feature = "p3")]
pub type TaskFn<T> = Box<
    dyn for<'a> FnOnce(
            &'a Accessor<T>,
            &'a p3::bindings::Proxy,
        ) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>>
        + Send,
>;

/// Pairs a [`TaskFn`] with a request ID.
#[cfg(feature = "p3")]
pub struct Task<T: 'static> {
    run: TaskFn<T>,
    request_id: u64,
}

#[cfg(feature = "p3")]
impl<T> Task<T> {
    /// Create a new `Task` using the specified function and request ID.
    pub fn new(run: TaskFn<T>, request_id: u64) -> Self {
        Self { run, request_id }
    }
}

/// Async MPMC channel where each item is delivered to at most one consumer.
#[cfg(feature = "p3")]
struct Queue<T> {
    queue: Mutex<VecDeque<T>>,
    notify: Notify,
}

#[cfg(feature = "p3")]
impl<T> Default for Queue<T> {
    fn default() -> Self {
        Self {
            queue: Default::default(),
            notify: Default::default(),
        }
    }
}

#[cfg(feature = "p3")]
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
    #[cfg(feature = "p3")]
    task_queue: Queue<Task<T>>,
    #[cfg(feature = "p3")]
    worker_count: AtomicUsize,
}

#[cfg(feature = "p3")]
struct Worker<S, T: 'static>
where
    T: Send,
    S: HandlerState<StoreData = T>,
{
    handler: ProxyHandler<S, T>,
    available: bool,
}

#[cfg(feature = "p3")]
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

        let ProxyPre::P3(pre) = &handler.instance_pre else {
            // See check in `Self::push`
            unreachable!()
        };

        let proxy = &pre.instantiate_async(&mut store).await?;
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
                            if let Err(error) = (task.run)(accessor, proxy).await {
                                eprintln!("[{}] :: {error:?}", task.request_id);
                            }
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

#[cfg(feature = "p3")]
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
/// Note that this has special support for WASIp3 instance reuse.  See
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
            #[cfg(feature = "p3")]
            task_queue: Default::default(),
            #[cfg(feature = "p3")]
            worker_count: AtomicUsize::from(0),
        }))
    }

    /// Push a task to the task queue for this handler.
    ///
    /// This will either spawn a new background worker to run the task or
    /// deliver it to an already-running worker.
    ///
    /// # Panics
    ///
    /// This will currently panic if the backing pre-instance is not a
    /// `ProxyPre::P3`.  `ProxyPre::P2` support may be added in the future.
    #[cfg(feature = "p3")]
    pub async fn push(&self, task: Task<T>) -> Result<()>
    where
        T: Send,
        S: HandlerState<StoreData = T>,
    {
        // TODO: Support p2 instances as well as p3 ones
        let ProxyPre::P3(pre) = &self.0.instance_pre else {
            panic!("ProxyHandler::push is only supported for WASIp3 handlers");
        };

        match self.0.state.max_instance_reuse_count() {
            0 => bail!("`max_instance_reuse_count` must be at least 1"),
            1 => {
                // Use a simplified path when instance reuse is disabled.
                //
                // This provides somewhat (e.g. ~20%) better throughput as
                // measured by `wasmtime-serve-rps.sh` than the
                // task-queue-and-workers approach below, so probably worth the
                // slight code duplication.
                let StoreBundle {
                    mut store,
                    write_profile,
                } = self.0.state.new_store()?;

                let proxy = &pre.instantiate_async(&mut store).await?;

                store
                    .run_concurrent(async |accessor| {
                        tokio::time::timeout(self.0.state.request_timeout(), async {
                            if let Err(error) = (task.run)(accessor, proxy).await {
                                eprintln!("[{}] :: {error:?}", task.request_id);
                            }
                        })
                        .await
                        .map_err(|_| anyhow!("guest timed out"))
                    })
                    .await??;

                write_profile(store.as_context_mut());
            }
            _ => {
                self.0.task_queue.push(task);
                self.maybe_start_worker();
            }
        }

        Ok(())
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

    #[cfg(feature = "p3")]
    fn maybe_start_worker(&self)
    where
        T: Send,
        S: HandlerState<StoreData = T>,
    {
        if self.0.worker_count.load(SeqCst) == 0 {
            self.start_worker();
        }
    }

    #[cfg(feature = "p3")]
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
