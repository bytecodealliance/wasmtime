//! Provides utilities useful for dispatching incoming HTTP requests
//! `wasi:http/handler` guest instances.

#[cfg(feature = "p3")]
use crate::p3;
use futures::stream::{FuturesUnordered, StreamExt};
use std::collections::VecDeque;
use std::collections::btree_map::{BTreeMap, Entry};
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
use std::time::{Duration, Instant};
use tokio::sync::Notify;
use wasmtime::AsContextMut;
use wasmtime::component::Accessor;
use wasmtime::{Result, Store, StoreContextMut, format_err};

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
    type StoreData: Send;

    /// Create a new [`Store`] for handling one or more requests.
    ///
    /// The `req_id` parameter is the value passed in the call to
    /// [`ProxyHandler::spawn`] that created the worker to which the new `Store`
    /// will belong.  See that function's documentation for details.
    fn new_store(&self, req_id: Option<u64>) -> Result<StoreBundle<Self::StoreData>>;

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

    /// Called when a worker exits with an error.
    fn handle_worker_error(&self, error: wasmtime::Error);
}

struct ProxyHandlerInner<S: HandlerState> {
    state: S,
    instance_pre: ProxyPre<S::StoreData>,
    next_id: AtomicU64,
    task_queue: Queue<TaskFn<S::StoreData>>,
    worker_count: AtomicUsize,
}

/// Helper utility to track the start times of tasks accepted by a worker.
///
/// This is used to ensure that timeouts are enforced even when the
/// `StoreContextMut::run_concurrent` event loop is unable to make progress due
/// to the guest either busy looping or being blocked on a synchronous call to a
/// host function which has exclusive access to the `Store`.
#[derive(Default)]
struct StartTimes(BTreeMap<Instant, usize>);

impl StartTimes {
    fn add(&mut self, time: Instant) {
        *self.0.entry(time).or_insert(0) += 1;
    }

    fn remove(&mut self, time: Instant) {
        let Entry::Occupied(mut entry) = self.0.entry(time) else {
            unreachable!()
        };
        match *entry.get() {
            0 => unreachable!(),
            1 => {
                entry.remove();
            }
            _ => {
                *entry.get_mut() -= 1;
            }
        }
    }

    fn earliest(&self) -> Option<Instant> {
        self.0.first_key_value().map(|(&k, _)| k)
    }
}

struct Worker<S>
where
    S: HandlerState,
{
    handler: ProxyHandler<S>,
    available: bool,
}

impl<S> Worker<S>
where
    S: HandlerState,
{
    fn set_available(&mut self, available: bool) {
        if available != self.available {
            self.available = available;
            if available {
                self.handler.0.worker_count.fetch_add(1, Relaxed);
            } else {
                // Here we use `SeqCst` to ensure the load/store is ordered
                // correctly with respect to the `Queue::is_empty` check we do
                // below.
                let count = self.handler.0.worker_count.fetch_sub(1, SeqCst);
                // This addresses what would otherwise be a race condition in
                // `ProxyHandler::spawn` where it only starts a worker if the
                // available worker count is zero.  If we decrement the count to
                // zero right after `ProxyHandler::spawn` checks it, then no
                // worker will be started; thus it becomes our responsibility to
                // start a worker here instead.
                if count == 1 && !self.handler.0.task_queue.is_empty() {
                    self.handler.start_worker(None, None);
                }
            }
        }
    }

    async fn run(mut self, task: Option<TaskFn<S::StoreData>>, req_id: Option<u64>) {
        if let Err(error) = self.run_(task, req_id).await {
            self.handler.0.state.handle_worker_error(error);
        }
    }

    async fn run_(
        &mut self,
        task: Option<TaskFn<S::StoreData>>,
        req_id: Option<u64>,
    ) -> Result<()> {
        // NB: The code the follows is rather subtle in that it is structured
        // carefully to provide a few key invariants related to how instance
        // reuse and request timeouts interact:
        //
        // - A task must never be allowed to run for more than 2x the request
        // timeout, if any.
        //
        // - Every task we accept here must be allowed to run for at least 1x
        // the request timeout, if any.
        //
        // - When more than one task is run concurrently in the same instance,
        // we must stop accepting new tasks as soon as any existing task reaches
        // the request timeout.  This serves to cap the amount of time we need
        // to keep the instance alive before _all_ tasks have either completed
        // or timed out.
        //
        // As of this writing, there's an additional wrinkle that makes
        // guaranteeing those invariants particularly tricky: per #11869 and
        // #11870, busy guest loops, epoch interruption, and host functions
        // registered using `Linker::func_{wrap,new}_async` all require
        // blocking, exclusive access to the `Store`, which effectively prevents
        // the `StoreContextMut::run_concurrent` event loop from making
        // progress.  That, in turn, prevents any concurrent tasks from
        // executing, and also prevents the `AsyncFnOnce` passed to
        // `run_concurrent` from being polled.  Consequently, we must rely on a
        // "second line of defense" to ensure tasks are timed out promptly,
        // which is to check for timeouts _outside_ the `run_concurrent` future.
        // Once the aforementioned issues have been addressed, we'll be able to
        // remove that check and its associated baggage.

        let handler = &self.handler.0;

        let StoreBundle {
            mut store,
            write_profile,
        } = handler.state.new_store(req_id)?;

        let request_timeout = handler.state.request_timeout();
        let idle_instance_timeout = handler.state.idle_instance_timeout();
        let max_instance_reuse_count = handler.state.max_instance_reuse_count();
        let max_instance_concurrent_reuse_count =
            handler.state.max_instance_concurrent_reuse_count();

        let proxy = &handler.instance_pre.instantiate_async(&mut store).await?;
        let accept_concurrent = AtomicBool::new(true);
        let task_start_times = Mutex::new(StartTimes::default());

        let mut future = pin!(store.run_concurrent(async |accessor| {
            let mut reuse_count = 0;
            let mut timed_out = false;
            let mut futures = FuturesUnordered::new();

            let accept_task = |task: TaskFn<S::StoreData>,
                               futures: &mut FuturesUnordered<_>,
                               reuse_count: &mut usize| {
                // Set `accept_concurrent` to false, conservatively assuming
                // that the new task will be CPU-bound, at least to begin with.
                // Only once the `StoreContextMut::run_concurrent` event loop
                // returns `Pending` will we set `accept_concurrent` back to
                // true and consider accepting more tasks.
                //
                // This approach avoids taking on more than one CPU-bound task
                // at a time, which would hurt throughput vs. leaving the
                // additional tasks for other workers to handle.
                accept_concurrent.store(false, Relaxed);
                *reuse_count += 1;

                let start_time = Instant::now().checked_add(request_timeout);
                if let Some(start_time) = start_time {
                    task_start_times.lock().unwrap().add(start_time);
                }

                futures.push(tokio::time::timeout(request_timeout, async move {
                    (task)(accessor, proxy).await;
                    start_time
                }));
            };

            if let Some(task) = task {
                accept_task(task, &mut futures, &mut reuse_count);
            }

            let handler = self.handler.clone();
            while !(futures.is_empty() && reuse_count >= max_instance_reuse_count) {
                let new_task = {
                    let future_count = futures.len();
                    let mut next_future = pin!(async {
                        if futures.is_empty() {
                            future::pending().await
                        } else {
                            futures.next().await.unwrap()
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
                    //
                    // Note the the order of operations here is important.  By
                    // polling `next_future` first, we'll disover any tasks that
                    // may have timed out, at which point we'll stop accepting
                    // new tasks altogether (see below for details).  This is
                    // especially imporant in the case where the task was
                    // blocked on a synchronous call to a host function which
                    // has exclusive access to the `Store`; once that call
                    // finishes, the first think we need to do is time out the
                    // task.  If we were to poll for a new task first, then we'd
                    // have to wait for _that_ task to finish or time out before
                    // we could kill the instance.
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
                        Poll::Ready(Ok(start_time)) => {
                            // Task completed; carry on!
                            if let Some(start_time) = start_time {
                                task_start_times.lock().unwrap().remove(start_time);
                            }
                            Poll::Ready(None)
                        }
                        Poll::Ready(Err(_)) => {
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
                    })
                    .await
                };

                match new_task {
                    Some(Ok(task)) => {
                        accept_task(task, &mut futures, &mut reuse_count);
                    }
                    Some(Err(_)) => break,
                    None => {}
                }
            }

            accessor.with(|mut access| write_profile(access.as_context_mut()));

            if timed_out {
                Err(format_err!("guest timed out"))
            } else {
                wasmtime::error::Ok(())
            }
        }));

        let mut sleep = pin!(tokio::time::sleep(Duration::MAX));

        future::poll_fn(|cx| {
            let poll = future.as_mut().poll(cx);
            if poll.is_pending() {
                // If the future returns `Pending`, that's either because it's
                // idle (in which case it can definitely accept a new task) or
                // because all its tasks are awaiting I/O, in which case it may
                // have capacity for additional tasks to run concurrently.
                //
                // However, if one of the tasks is blocked on a sync call to a
                // host function which has exclusive access to the `Store`, the
                // `StoreContextMut::run_concurrent` event loop will be unable
                // to make progress until that call finishes.  Similarly, if the
                // task loops indefinitely, subject only to epoch interruption,
                // the event loop will also be stuck.  Either way, any task
                // timeouts created inside the `AsyncFnOnce` we passed to
                // `run_concurrent` won't have a chance to trigger.
                // Consequently, we need to _also_ enforce timeouts here,
                // outside the event loop.
                //
                // Therefore, we check if the oldest outstanding task has been
                // running for at least `request_timeout*2`, which is the
                // maximum time needed for any other concurrent tasks to
                // complete or time out, at which point we can safely discard
                // the instance.  If that deadline has not yet arrived, we
                // schedule a wakeup to occur when it does.
                //
                // We uphold the "never kill an instance with a task which has
                // been running for less than the request timeout" invariant
                // here by noting that this timeout will only trigger if the
                // `AsyncFnOnce` we passed to `run_concurrent` has been unable
                // to run for at least the past `request_timeout` amount of
                // time, meaning it can't possibly have accepted a task newer
                // than that.
                if let Some(deadline) = task_start_times
                    .lock()
                    .unwrap()
                    .earliest()
                    .and_then(|v| v.checked_add(request_timeout.saturating_mul(2)))
                {
                    sleep.as_mut().reset(deadline.into());
                    // Note that this will schedule a wakeup for later if the
                    // deadline has not yet arrived:
                    if sleep.as_mut().poll(cx).is_ready() {
                        // Deadline has been reached; kill the instance with an
                        // error.
                        return Poll::Ready(Err(format_err!("guest timed out")));
                    }
                }

                // Otherwise, if no timeouts have elapsed, we set
                // `accept_concurrent` to true and, if it wasn't already true
                // before, poll the future one more time so it can ask for
                // another task if appropriate.
                if !accept_concurrent.swap(true, Relaxed) {
                    return future.as_mut().poll(cx);
                }
            }

            poll
        })
        .await?
    }
}

impl<S> Drop for Worker<S>
where
    S: HandlerState,
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
pub struct ProxyHandler<S: HandlerState>(Arc<ProxyHandlerInner<S>>);

impl<S: HandlerState> Clone for ProxyHandler<S> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<S> ProxyHandler<S>
where
    S: HandlerState,
{
    /// Create a new `ProxyHandler` with the specified application state and
    /// pre-instance.
    pub fn new(state: S, instance_pre: ProxyPre<S::StoreData>) -> Self {
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
    ///
    /// The `req_id` will be passed to `<S as HandlerState>::new_store` _if_ a
    /// new worker is started for this task.  It is intended to be used as a
    /// "request identifier" corresponding to that task and can be used e.g. to
    /// prefix all logging from the `Store` with that identifier.  Note that a
    /// non-`None` value only makes sense when `<S as
    /// HandlerState>::max_instance_reuse_count == 1`; otherwise the identifier
    /// will not match subsequent tasks handled by the worker.
    pub fn spawn(&self, req_id: Option<u64>, task: TaskFn<S::StoreData>) {
        match self.0.state.max_instance_reuse_count() {
            0 => panic!("`max_instance_reuse_count` must be at least 1"),
            _ => {
                if self.0.worker_count.load(Relaxed) == 0 {
                    // There are no available workers; skip the queue and pass
                    // the task directly to the worker, which improves
                    // performance as measured by `wasmtime-server-rps.sh` by
                    // about 15%.
                    self.start_worker(Some(task), req_id);
                } else {
                    self.0.task_queue.push(task);
                    // Start a new worker to handle the task if the last worker
                    // just went unavailable.  See also `Worker::set_available`
                    // for what happens if the available worker count goes to
                    // zero right after we check it here, and note that we only
                    // check the count _after_ we've pushed the task to the
                    // queue.  We use `SeqCst` here to ensure that we get an
                    // updated view of `worker_count` as it exists after the
                    // `Queue::push` above.
                    //
                    // The upshot is that at least one (or more) of the
                    // following will happen:
                    //
                    // - An existing worker will accept the task
                    // - We'll start a new worker here to accept the task
                    // - `Worker::set_available` will start a new worker to accept the task
                    //
                    // I.e. it should not be possible for the task to be
                    // orphaned indefinitely in the queue without being
                    // accepted.
                    if self.0.worker_count.load(SeqCst) == 0 {
                        self.start_worker(None, None);
                    }
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
    pub fn instance_pre(&self) -> &ProxyPre<S::StoreData> {
        &self.0.instance_pre
    }

    fn start_worker(&self, task: Option<TaskFn<S::StoreData>>, req_id: Option<u64>) {
        tokio::spawn(
            Worker {
                handler: self.clone(),
                available: false,
            }
            .run(task, req_id),
        );
    }
}
