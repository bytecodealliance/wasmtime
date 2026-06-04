//! Provides utilities useful for dispatching incoming HTTP requests
//! `wasi:http/handler` guest instances.

#[cfg(feature = "p2")]
use crate::p2;
#[cfg(feature = "p2")]
use crate::p2::bindings::http::types as p2_types;
#[cfg(feature = "p3")]
use crate::p3;
use bytes::Bytes;
use futures::{
    channel::oneshot,
    future::{Either, FutureExt},
    stream::{FuturesUnordered, Stream},
};
use http_body_util::{BodyExt, combinators::UnsyncBoxBody};
#[cfg(feature = "p3")]
use p3::bindings::http::types as p3_types;
use std::collections::VecDeque;
use std::collections::btree_map::{BTreeMap, Entry};
use std::error;
use std::fmt;
use std::future;
use std::mem;
use std::ops::DerefMut;
use std::pin::{Pin, pin};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed},
};
use std::task::{Context, Poll};
use std::time::Instant;
use tokio::sync::Notify;
use wasmtime::component::{Accessor, GuestTaskId, Resource, TypedFuncCallConcurrent};
#[cfg(feature = "p2")]
use wasmtime::error::Context as _;
use wasmtime::{AsContextMut, Result, Store, StoreContextMut, format_err};

/// Represents either a `wasi:http/types@0.2.x` or `wasi:http/types@0.3.x` `error-code`.
pub enum ErrorCode {
    /// A `wasi:http/types@0.2.x` `error-code`.
    #[cfg(feature = "p2")]
    P2(p2_types::ErrorCode),
    /// A `wasi:http/types@0.3.x` `error-code`.
    #[cfg(feature = "p3")]
    P3(p3_types::ErrorCode),
}

#[cfg(feature = "p2")]
impl From<p2_types::ErrorCode> for ErrorCode {
    fn from(code: p2_types::ErrorCode) -> Self {
        Self::P2(code)
    }
}

#[cfg(feature = "p3")]
impl From<p3_types::ErrorCode> for ErrorCode {
    fn from(code: p3_types::ErrorCode) -> Self {
        Self::P3(code)
    }
}

#[cfg(feature = "p2")]
impl From<ErrorCode> for p2_types::ErrorCode {
    fn from(code: ErrorCode) -> p2_types::ErrorCode {
        match code {
            ErrorCode::P2(code) => code,
            #[cfg(feature = "p3")]
            ErrorCode::P3(code) => code.into(),
        }
    }
}

#[cfg(feature = "p3")]
impl From<ErrorCode> for p3_types::ErrorCode {
    fn from(code: ErrorCode) -> p3_types::ErrorCode {
        match code {
            #[cfg(feature = "p2")]
            ErrorCode::P2(code) => code.into(),
            ErrorCode::P3(code) => code,
        }
    }
}

#[cfg(all(feature = "p2", feature = "p3"))]
impl From<p2_types::ErrorCode> for p3_types::ErrorCode {
    fn from(code: p2_types::ErrorCode) -> Self {
        match code {
            p2_types::ErrorCode::DnsTimeout => Self::DnsTimeout,
            p2_types::ErrorCode::DnsError(payload) => Self::DnsError(p3_types::DnsErrorPayload {
                rcode: payload.rcode,
                info_code: payload.info_code,
            }),
            p2_types::ErrorCode::DestinationNotFound => Self::DestinationNotFound,
            p2_types::ErrorCode::DestinationUnavailable => Self::DestinationUnavailable,
            p2_types::ErrorCode::DestinationIpProhibited => Self::DestinationIpProhibited,
            p2_types::ErrorCode::DestinationIpUnroutable => Self::DestinationIpUnroutable,
            p2_types::ErrorCode::ConnectionRefused => Self::ConnectionRefused,
            p2_types::ErrorCode::ConnectionTerminated => Self::ConnectionTerminated,
            p2_types::ErrorCode::ConnectionTimeout => Self::ConnectionTimeout,
            p2_types::ErrorCode::ConnectionReadTimeout => Self::ConnectionReadTimeout,
            p2_types::ErrorCode::ConnectionWriteTimeout => Self::ConnectionWriteTimeout,
            p2_types::ErrorCode::ConnectionLimitReached => Self::ConnectionLimitReached,
            p2_types::ErrorCode::TlsProtocolError => Self::TlsProtocolError,
            p2_types::ErrorCode::TlsCertificateError => Self::TlsCertificateError,
            p2_types::ErrorCode::TlsAlertReceived(payload) => {
                Self::TlsAlertReceived(p3_types::TlsAlertReceivedPayload {
                    alert_id: payload.alert_id,
                    alert_message: payload.alert_message,
                })
            }
            p2_types::ErrorCode::HttpRequestDenied => Self::HttpRequestDenied,
            p2_types::ErrorCode::HttpRequestLengthRequired => Self::HttpRequestLengthRequired,
            p2_types::ErrorCode::HttpRequestBodySize(payload) => Self::HttpRequestBodySize(payload),
            p2_types::ErrorCode::HttpRequestMethodInvalid => Self::HttpRequestMethodInvalid,
            p2_types::ErrorCode::HttpRequestUriInvalid => Self::HttpRequestUriInvalid,
            p2_types::ErrorCode::HttpRequestUriTooLong => Self::HttpRequestUriTooLong,
            p2_types::ErrorCode::HttpRequestHeaderSectionSize(payload) => {
                Self::HttpRequestHeaderSectionSize(payload)
            }
            p2_types::ErrorCode::HttpRequestHeaderSize(payload) => {
                Self::HttpRequestHeaderSize(payload.map(|payload| p3_types::FieldSizePayload {
                    field_name: payload.field_name,
                    field_size: payload.field_size,
                }))
            }
            p2_types::ErrorCode::HttpRequestTrailerSectionSize(payload) => {
                Self::HttpRequestTrailerSectionSize(payload)
            }
            p2_types::ErrorCode::HttpRequestTrailerSize(payload) => {
                Self::HttpRequestTrailerSize(p3_types::FieldSizePayload {
                    field_name: payload.field_name,
                    field_size: payload.field_size,
                })
            }
            p2_types::ErrorCode::HttpResponseIncomplete => Self::HttpResponseIncomplete,
            p2_types::ErrorCode::HttpResponseHeaderSectionSize(payload) => {
                Self::HttpResponseHeaderSectionSize(payload)
            }
            p2_types::ErrorCode::HttpResponseHeaderSize(payload) => {
                Self::HttpResponseHeaderSize(p3_types::FieldSizePayload {
                    field_name: payload.field_name,
                    field_size: payload.field_size,
                })
            }
            p2_types::ErrorCode::HttpResponseBodySize(payload) => {
                Self::HttpResponseBodySize(payload)
            }
            p2_types::ErrorCode::HttpResponseTrailerSectionSize(payload) => {
                Self::HttpResponseTrailerSectionSize(payload)
            }
            p2_types::ErrorCode::HttpResponseTrailerSize(payload) => {
                Self::HttpResponseTrailerSize(p3_types::FieldSizePayload {
                    field_name: payload.field_name,
                    field_size: payload.field_size,
                })
            }
            p2_types::ErrorCode::HttpResponseTransferCoding(payload) => {
                Self::HttpResponseTransferCoding(payload)
            }
            p2_types::ErrorCode::HttpResponseContentCoding(payload) => {
                Self::HttpResponseContentCoding(payload)
            }
            p2_types::ErrorCode::HttpResponseTimeout => Self::HttpResponseTimeout,
            p2_types::ErrorCode::HttpUpgradeFailed => Self::HttpUpgradeFailed,
            p2_types::ErrorCode::HttpProtocolError => Self::HttpProtocolError,
            p2_types::ErrorCode::LoopDetected => Self::LoopDetected,
            p2_types::ErrorCode::ConfigurationError => Self::ConfigurationError,
            p2_types::ErrorCode::InternalError(payload) => Self::InternalError(payload),
        }
    }
}

#[cfg(all(feature = "p2", feature = "p3"))]
impl From<p3_types::ErrorCode> for p2_types::ErrorCode {
    fn from(code: p3_types::ErrorCode) -> Self {
        match code {
            p3_types::ErrorCode::DnsTimeout => Self::DnsTimeout,
            p3_types::ErrorCode::DnsError(payload) => Self::DnsError(p2_types::DnsErrorPayload {
                rcode: payload.rcode,
                info_code: payload.info_code,
            }),
            p3_types::ErrorCode::DestinationNotFound => Self::DestinationNotFound,
            p3_types::ErrorCode::DestinationUnavailable => Self::DestinationUnavailable,
            p3_types::ErrorCode::DestinationIpProhibited => Self::DestinationIpProhibited,
            p3_types::ErrorCode::DestinationIpUnroutable => Self::DestinationIpUnroutable,
            p3_types::ErrorCode::ConnectionRefused => Self::ConnectionRefused,
            p3_types::ErrorCode::ConnectionTerminated => Self::ConnectionTerminated,
            p3_types::ErrorCode::ConnectionTimeout => Self::ConnectionTimeout,
            p3_types::ErrorCode::ConnectionReadTimeout => Self::ConnectionReadTimeout,
            p3_types::ErrorCode::ConnectionWriteTimeout => Self::ConnectionWriteTimeout,
            p3_types::ErrorCode::ConnectionLimitReached => Self::ConnectionLimitReached,
            p3_types::ErrorCode::TlsProtocolError => Self::TlsProtocolError,
            p3_types::ErrorCode::TlsCertificateError => Self::TlsCertificateError,
            p3_types::ErrorCode::TlsAlertReceived(payload) => {
                Self::TlsAlertReceived(p2_types::TlsAlertReceivedPayload {
                    alert_id: payload.alert_id,
                    alert_message: payload.alert_message,
                })
            }
            p3_types::ErrorCode::HttpRequestDenied => Self::HttpRequestDenied,
            p3_types::ErrorCode::HttpRequestLengthRequired => Self::HttpRequestLengthRequired,
            p3_types::ErrorCode::HttpRequestBodySize(payload) => Self::HttpRequestBodySize(payload),
            p3_types::ErrorCode::HttpRequestMethodInvalid => Self::HttpRequestMethodInvalid,
            p3_types::ErrorCode::HttpRequestUriInvalid => Self::HttpRequestUriInvalid,
            p3_types::ErrorCode::HttpRequestUriTooLong => Self::HttpRequestUriTooLong,
            p3_types::ErrorCode::HttpRequestHeaderSectionSize(payload) => {
                Self::HttpRequestHeaderSectionSize(payload)
            }
            p3_types::ErrorCode::HttpRequestHeaderSize(payload) => {
                Self::HttpRequestHeaderSize(payload.map(|payload| p2_types::FieldSizePayload {
                    field_name: payload.field_name,
                    field_size: payload.field_size,
                }))
            }
            p3_types::ErrorCode::HttpRequestTrailerSectionSize(payload) => {
                Self::HttpRequestTrailerSectionSize(payload)
            }
            p3_types::ErrorCode::HttpRequestTrailerSize(payload) => {
                Self::HttpRequestTrailerSize(p2_types::FieldSizePayload {
                    field_name: payload.field_name,
                    field_size: payload.field_size,
                })
            }
            p3_types::ErrorCode::HttpResponseIncomplete => Self::HttpResponseIncomplete,
            p3_types::ErrorCode::HttpResponseHeaderSectionSize(payload) => {
                Self::HttpResponseHeaderSectionSize(payload)
            }
            p3_types::ErrorCode::HttpResponseHeaderSize(payload) => {
                Self::HttpResponseHeaderSize(p2_types::FieldSizePayload {
                    field_name: payload.field_name,
                    field_size: payload.field_size,
                })
            }
            p3_types::ErrorCode::HttpResponseBodySize(payload) => {
                Self::HttpResponseBodySize(payload)
            }
            p3_types::ErrorCode::HttpResponseTrailerSectionSize(payload) => {
                Self::HttpResponseTrailerSectionSize(payload)
            }
            p3_types::ErrorCode::HttpResponseTrailerSize(payload) => {
                Self::HttpResponseTrailerSize(p2_types::FieldSizePayload {
                    field_name: payload.field_name,
                    field_size: payload.field_size,
                })
            }
            p3_types::ErrorCode::HttpResponseTransferCoding(payload) => {
                Self::HttpResponseTransferCoding(payload)
            }
            p3_types::ErrorCode::HttpResponseContentCoding(payload) => {
                Self::HttpResponseContentCoding(payload)
            }
            p3_types::ErrorCode::HttpResponseTimeout => Self::HttpResponseTimeout,
            p3_types::ErrorCode::HttpUpgradeFailed => Self::HttpUpgradeFailed,
            p3_types::ErrorCode::HttpProtocolError => Self::HttpProtocolError,
            p3_types::ErrorCode::LoopDetected => Self::LoopDetected,
            p3_types::ErrorCode::ConfigurationError => Self::ConfigurationError,
            p3_types::ErrorCode::InternalError(payload) => Self::InternalError(payload),
        }
    }
}

/// Represents either a p2 or p3 `WasiHttpCtxView` getter.
pub enum ViewFn<T> {
    /// A p2 getter.
    #[cfg(feature = "p2")]
    P2(fn(&mut T) -> crate::p2::WasiHttpCtxView),
    /// A p3 getter.
    #[cfg(feature = "p3")]
    P3(fn(&mut T) -> p3::WasiHttpCtxView),
}

impl<T> Clone for ViewFn<T> {
    fn clone(&self) -> Self {
        match self {
            #[cfg(feature = "p2")]
            &Self::P2(view) => Self::P2(view),
            #[cfg(feature = "p3")]
            &Self::P3(view) => Self::P3(view),
        }
    }
}

impl<T> Copy for ViewFn<T> {}

/// A Request to be handled using `ProxyHandler::handle`.
pub type Request = http::Request<UnsyncBoxBody<Bytes, ErrorCode>>;

/// A Response returned by `ProxyHandler::handle`.
pub type Response = http::Response<UnsyncBoxBody<Bytes, wasmtime::Error>>;

/// Represents either a `wasi:http/incoming-handler@0.2.x` or
/// `wasi:http/handler@0.3.x` pre-instance.
pub enum ProxyPre<T: 'static> {
    /// A `wasi:http/incoming-handler@0.2.x` pre-instance.
    #[cfg(feature = "p2")]
    P2(p2::bindings::ProxyPre<T>),
    /// A `wasi:http/handler@0.3.x` pre-instance.
    #[cfg(feature = "p3")]
    P3(p3::bindings::ServicePre<T>),
}

impl<T: 'static> ProxyPre<T> {
    /// Instantiates the pre-instance.
    pub async fn instantiate_async(&self, store: impl AsContextMut<Data = T>) -> Result<Proxy>
    where
        T: Send,
    {
        Ok(match self {
            #[cfg(feature = "p2")]
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
    #[cfg(feature = "p2")]
    P2(p2::bindings::Proxy),
    /// A `wasi:http/handler@0.3.x` instance.
    #[cfg(feature = "p3")]
    P3(p3::bindings::Service),
}

/// Async MPMC channel where each item is delivered to at most one consumer.
struct Queue<T> {
    queue: Mutex<VecDeque<T>>,
    notify_push: Notify,
}

impl<T> Default for Queue<T> {
    fn default() -> Self {
        Self {
            queue: Default::default(),
            notify_push: Default::default(),
        }
    }
}

impl<T> Queue<T> {
    fn is_empty(&self) -> bool {
        self.queue.lock().unwrap().is_empty()
    }

    fn try_pop(&self) -> Option<T> {
        self.queue.lock().unwrap().pop_front()
    }

    async fn pop(&self) -> T {
        // This code comes from the Unbounded MPMC Channel example in [the
        // `tokio::sync::Notify`
        // docs](https://docs.rs/tokio/latest/tokio/sync/struct.Notify.html).

        let mut notified = pin!(self.notify_push.notified());

        loop {
            notified.as_mut().enable();
            if let Some(item) = self.try_pop() {
                return item;
            }
            notified.as_mut().await;
            notified.set(self.notify_push.notified());
        }
    }
}

/// Represents the status of a `ProxyHandler` worker task.
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum WorkerStatus {
    /// The worker is not handling any requests, nor is it doing any post-return
    /// work.  It _might_ be doing background work which the guest has indicated
    /// can be interrupted and/or abandoned at any time, i.e. does not prevent
    /// the instance from being disposed.
    Idle,
    /// The instance is handling one or more requests, waiting for each to
    /// either produce a response or expire.
    Requests,
    /// All requests handled so far have either produced a response or expired,
    /// but the guest has post-return work which needs to finish before the
    /// instance can be considered idle.
    PostReturn,
}

/// Represents the application-specific state of a `ProxyHandler` worker.
///
/// [`HandlerState::instantiate`] returns an implementation of this trait for
/// each component instance (and thus each worker) created.  The worker uses it
/// to determine when to exit.
pub trait WorkerExpiration: 'static + Send + Sync {
    /// Poll whether the worker has expired.
    ///
    /// This will return `Poll::Ready(())` if the worker has expired, meaning
    /// the component instance should be dropped.  Otherwise, it will return
    /// `Poll::Pending` and wake the `Waker` if and when it should be polled
    /// again.
    ///
    /// `state` represents the current state of the worker, and `start`
    /// represents when it transitioned into that state (or in the case of
    /// `WorkerState::Requests`, when the most recent outstanding request
    /// was accepted).
    fn poll(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        state: WorkerStatus,
        start: Instant,
    ) -> Poll<()>;
}

/// Represents the application-specific state of a `ProxyHandler` worker.
///
/// [`HandlerState::instantiate`] returns an implementation of this trait for
/// each component instance (and thus each worker) created.  The worker uses it
/// to determine how many requests to accept, how long to wait for the guest to
/// produce responses, etc.
pub trait WorkerState: 'static + Send + Sync {
    /// The type of the associated data for [`Store`] belonging to this worker.
    type StoreData: Send;

    /// An opaque unique identifier that hosts can assigned to requests which is
    /// threaded from [`ProxyHandler::handle`] into
    /// [`WorkerState::on_request_start`]
    type RequestId: Send + Sync;

    /// Indicate whether the worker should accept another request given the
    /// current number it is already handling concurrently and the total it has
    /// handled so far.
    fn should_accept_request(&self, concurrent_count: usize, total_count: usize) -> ShouldAccept;

    /// Notification that a request has been accepted by the worker.
    ///
    /// This method can be used to record anything within `store`, if necessary.
    /// The `task` corresponding to the component-model-level async task about
    /// to be created is additionally passed here.
    ///
    /// If the future returned by this function resolves before the guest has
    /// produced a response, the request will be considered "expired" and the
    /// original `ProxyHandler::handle` future will resolve to an
    /// `Err(ExpirationError.into())`.  In addition, the worker
    /// will stop accepting new requests but will continue running until all
    /// requests that have been accepted by the worker have either produced a
    /// response or expired, at which point the state of the worker will
    /// transition to either `WorkerState::PostReturn` or `WorkerState::Idle`.
    ///
    /// Note that the returned future is polled from within the
    /// `Store::run_concurrent` event loop, and due to #11869 and #11870, it may
    /// not be polled at all for arbitrary lengths of time.  Consequently, the
    /// `Self::Expiration` implementation (which is polled from _outside_ the
    /// `Store::run_concurrent` event loop) must also enforce request expiration
    /// as a second level of defence if desired.
    ///
    /// For example, if a request timeout of N seconds is to be enforced, the
    /// `Self::Expiration::poll` implementation, when called with
    /// `WorkerState::Requests` should calculate the time elapsed since the most
    /// recent outstanding request was accepted as indicated by the `start`
    /// parameter.  If that time is greater than N seconds, we can expire the
    /// instance immediately, confident that all outstanding requests have
    /// expired.
    ///
    /// Once #11869 and #11870 have been addressed, this "second level of
    /// defence" will no longer be necessary.
    fn on_request_start(
        &self,
        store: StoreContextMut<'_, Self::StoreData>,
        id: Self::RequestId,
        task: GuestTaskId,
    ) -> Pin<Box<dyn Future<Output = ()> + 'static + Send + Sync>>;

    /// Dispose of the store belonging to the now-exited worker.
    ///
    /// This may be used to e.g. collect metrics from the store or its
    /// associated data before the store is dropped, as well as e.g. retry
    /// failed instantiations after the store is dropped.
    ///
    /// If the store is being dropped due to an error (e.g. a guest trap or a
    /// host panic) `result` will be `Err(_)`; otherwise it will be `Ok(())`.
    fn drop(&self, store: Store<Self::StoreData>, result: Result<(), wasmtime::Error>);
}

/// Represents the combination of a store and instance with which to handle
/// requests.
pub struct Instance<T: 'static, E: WorkerExpiration, S: WorkerState> {
    /// The store to use to handle requests.
    pub store: Store<T>,
    /// The instance to use to handle requests.
    pub proxy: Proxy,
    /// `WasiHttpCtxView` getter function.
    pub view: ViewFn<T>,
    /// See [`WorkerExpiration`].
    pub expiration: E,
    /// See [`WorkerState`].
    pub state: S,
}

/// Indicates whether a worker should accept new requests.
pub enum ShouldAccept {
    /// Yes, it should.
    Yes,
    /// No, it shouldn't (but ask again later).
    No,
    /// No, it shouldn't (and don't ask again).
    Never,
}

/// Represents the application-specific state of a web server.
pub trait HandlerState: 'static + Sync + Send + Sized {
    /// The type of the associated data for [`Store`]s created using
    /// [`Self::instantiate`].
    type StoreData: Send;
    /// The type of the `WorkerExpiration` implementation to be returned from
    /// [`Self::instantiate`].
    type WorkerExpiration: WorkerExpiration;
    /// The type of the `WorkerState` implementation to be returned from
    /// [`Self::instantiate`].
    type WorkerState: WorkerState<StoreData = Self::StoreData>;

    /// Create a new store and instance for handling one or more requests.
    ///
    /// Note that the implementer is responsible for applying a timeout to the
    /// guest instantiation if appropriate (e.g. as part of an overall request
    /// timeout).
    fn instantiate(
        &self,
    ) -> impl Future<
        Output = Result<Instance<Self::StoreData, Self::WorkerExpiration, Self::WorkerState>>,
    > + Send;
}

struct ProxyHandlerInner<S: HandlerState> {
    state: S,
    request_queue: Queue<WorkerRequest<S>>,
    worker_count: AtomicUsize,
}

/// Tracks request start times.
///
/// This is useful for keeping a [`WorkerState`] appraised of the most recently
/// accepted outstanding request.
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

    fn most_recent(&self) -> Option<Instant> {
        self.0.last_key_value().map(|(&k, _)| k)
    }
}

type WorkerRequest<S> = (
    <<S as HandlerState>::WorkerState as WorkerState>::RequestId,
    Request,
    oneshot::Sender<Result<Response, wasmtime::Error>>,
);

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
                // Decrement the count _before_ checking if the request queue is
                // empty.  This helps ensure that `ProxyHandler::spawn` sees the
                // new value before deciding whether to spawn a new worker.
                let count = self.handler.0.worker_count.fetch_sub(1, Relaxed);
                assert!(count >= 1);

                // This addresses what would otherwise be a race condition in
                // `ProxyHandler::spawn` where it only starts a worker if the
                // available worker count is zero.  If we decrement the count to
                // zero right after `ProxyHandler::spawn` checks it, then no
                // worker will be started; thus it becomes our responsibility to
                // start a worker here instead.
                if count == 1 && !self.handler.0.request_queue.is_empty() {
                    self.handler.start_worker(None);
                }
            }
        }
    }

    async fn run(self, request: Option<WorkerRequest<S>>) {
        match self.handler.0.state.instantiate().await {
            Ok(Instance {
                store,
                proxy,
                view,
                expiration,
                state,
            }) => {
                self.run_(store, proxy, view, expiration, state, request)
                    .await
            }

            Err(error) => {
                let error = Arc::new(error);
                if let Some((request_id, request, tx)) = request {
                    _ = tx.send(Err(InstantiationError {
                        request_id,
                        request: Mutex::new(request),
                        error,
                    }
                    .into()));
                } else {
                    // In this case, the worker was spawned to handle any queued
                    // requests.  Since we can't handle those requests, we send
                    // them all an instantiation error.
                    for (request_id, request, tx) in mem::take(
                        self.handler
                            .0
                            .request_queue
                            .queue
                            .lock()
                            .unwrap()
                            .deref_mut(),
                    ) {
                        _ = tx.send(Err(InstantiationError {
                            request_id,
                            request: Mutex::new(request),
                            error: error.clone(),
                        }
                        .into()));
                    }
                }
            }
        }
    }

    async fn run_(
        mut self,
        store: Store<S::StoreData>,
        proxy: Proxy,
        view: ViewFn<S::StoreData>,
        expiration: S::WorkerExpiration,
        state: S::WorkerState,
        request: Option<WorkerRequest<S>>,
    ) {
        // NB: The code the follows is rather subtle in that it is structured
        // carefully to give the `HandlerState` implementation full control over
        // the component instance lifetime. Specifically, we must keep the
        // `HandlerState` informed of the worker's state and how long it has
        // been in that state, as well as allow it to expire the instance based
        // on whatever combination of timeouts, dynamic resource usage, etc. it
        // may take into consideration.
        //
        // Note that, when more than one request is handled concurrently in the
        // same instance, we must stop accepting new requests as soon as any
        // existing request reaches its expiration.  This serves to cap the
        // amount of time we need to keep the instance alive before _all_
        // requests have either completed or expired.
        //
        // As of this writing, there's an additional wrinkle that makes tracking
        // expiration particularly tricky: per #11869 and #11870, busy guest
        // loops, epoch interruption, and host functions registered using
        // `Linker::func_{wrap,new}_async` all require blocking, exclusive
        // access to the `Store`, which effectively prevents the
        // `StoreContextMut::run_concurrent` event loop from making progress.
        // That, in turn, prevents any concurrent tasks from executing, and also
        // prevents the `AsyncFnOnce` passed to `run_concurrent` from being
        // polled.  Consequently, we must poll `S::WorkerState` from _outside_
        // the `run_concurrent` future to ensure expirations are enforced.  Once
        // the aforementioned issues have been addressed, we'll be able to
        // simplify the code and eliminate the need for communication between
        // the "inside" future and the "outside" one.

        // Wrap `store` in an object which, prior to leaving this scope, will
        // pass the `store` to `HandlerState::drop`.
        struct Dropper<S: HandlerState> {
            state: S::WorkerState,
            store: Option<Store<S::StoreData>>,
        }

        impl<S: HandlerState> Drop for Dropper<S> {
            fn drop(&mut self) {
                if let Some(store) = self.store.take() {
                    self.state
                        .drop(store, Err(wasmtime::format_err!("worker panicked")));
                }
            }
        }

        let mut dropper = Dropper::<S> {
            state,
            store: Some(store),
        };

        let proxy = &proxy;

        let accept_concurrent = AtomicBool::new(true);
        let status = Mutex::new((WorkerStatus::Idle, Instant::now()));
        let mut expiration = pin!(expiration);

        let function = async |accessor: &Accessor<_>| {
            let mut reuse_count = 0;
            let mut may_accept = true;
            let mut futures = FuturesUnordered::new();
            let mut start_times = StartTimes::default();

            let accept_request = |(request_id, request, tx): WorkerRequest<S>,
                                  futures: &mut FuturesUnordered<_>,
                                  start_times: &mut StartTimes,
                                  reuse_count: &mut usize| {
                // Set `accept_concurrent` to false, conservatively assuming
                // that the new task will be CPU-bound, at least to begin with.
                // Only once the `StoreContextMut::run_concurrent` event loop
                // returns `Pending` will we set `accept_concurrent` back to
                // true and consider accepting more requests.
                //
                // This approach avoids taking on more than one CPU-bound task
                // at a time, which would hurt throughput vs. leaving the
                // additional requests for other workers to handle.
                accept_concurrent.store(false, Relaxed);
                *reuse_count += 1;

                let prepared = accessor.with(|mut store| {
                    let prepared = Prepared::new(store.as_context_mut(), proxy, request, view, tx);
                    match prepared {
                        Ok(prepared) => {
                            // Notify the `HandlerState` that we're starting to
                            // handle a request and retrieve the deadline by
                            // which it must produce a response.
                            //
                            // If it fails to produce a response by the
                            // deadline, we'll stop accepting new requests and
                            // eventually exit the worker.
                            let expiration = dropper.state.on_request_start(
                                store.as_context_mut(),
                                request_id,
                                prepared.task(),
                            );
                            Ok((prepared, expiration))
                        }
                        Err(e) => Err(e),
                    }
                });

                let start_time = Instant::now();
                start_times.add(start_time);
                *status.try_lock().unwrap() = (WorkerStatus::Requests, start_time);

                futures.push(async move {
                    let (prepared, expiration) = prepared?;
                    let sent = prepared.run(accessor, expiration).await?;
                    wasmtime::error::Ok((sent, start_time))
                });
            };

            if let Some(req) = request {
                accept_request(req, &mut futures, &mut start_times, &mut reuse_count);
            }

            // This is the main driver loop for this worker. This is modeled as
            // a `poll_fn` which internally loops around the possible events.
            // Events are sourced from the locals here, pinned outside of the
            // `poll_fn` closure.
            let mut futures = pin!(futures);
            let handler = self.handler.clone();
            let mut incoming_requests = pin!(futures::stream::unfold(
                &handler.0.request_queue,
                |queue| async move {
                    let pair = queue.pop().await;
                    Some((pair, queue))
                }
            ));
            future::poll_fn(|cx| {
                loop {
                    // First, and crucially first, poll `futures`. This way
                    // we'll discover any tasks that may have timed out, at
                    // which point we'll stop accepting new tasks altogether
                    // (see below for details). This is especially important in
                    // the case where the task was blocked on a synchronous call
                    // to a host function which has exclusive access to the
                    // `Store`; once that call finishes, the first thing we need
                    // to do is time out the task. If we were to poll for a new
                    // task first, then we'd have to wait for _that_ task to
                    // finish or time out before we could kill the instance.
                    match futures.as_mut().poll_next(cx) {
                        // A request either produced a response or expired.
                        Poll::Ready(Some(Ok((responded, start_time)))) => {
                            // Remove its start time from the map and update the
                            // state.
                            start_times.remove(start_time);
                            *status.try_lock().unwrap() =
                                if let Some(start_time) = start_times.most_recent() {
                                    (WorkerStatus::Requests, start_time)
                                } else {
                                    (WorkerStatus::PostReturn, Instant::now())
                                };

                            if responded {
                                // Response produced; carry on!
                            } else {
                                // Request expired; stop accepting new requests, but
                                // continue polling until any other, in-progress
                                // tasks until they have either finished or expired.
                                // This effectively kicks off a "graceful shutdown"
                                // of the worker, allowing any other concurrent
                                // tasks time to finish before we drop the instance.
                                may_accept = false;
                            }
                        }

                        // Instance trapped.
                        Poll::Ready(Some(Err(error))) => {
                            break Poll::Ready(Err(error));
                        }

                        Poll::Ready(None) | Poll::Pending => {}
                    }

                    // At this point `futures` is either empty or it's `Pending`
                    // meaning nothing is ready. Note that `Pending` here
                    // doesn't necessarily mean all tasks are blocked on I/O.
                    // They might simply be waiting for some deferred work to be
                    // done by the next turn of the
                    // `StoreContextMut::run_concurrent` event loop.  Therefore,
                    // we check `accept_concurrent` here and only advertise we
                    // have capacity for another task if either we have no tasks
                    // at all or all our tasks really are blocked on I/O.
                    self.set_available(
                        may_accept
                            && match dropper
                                .state
                                .should_accept_request(futures.len(), reuse_count)
                            {
                                ShouldAccept::Yes => {
                                    futures.is_empty() || accept_concurrent.load(Relaxed)
                                }
                                ShouldAccept::No => false,
                                ShouldAccept::Never => {
                                    may_accept = false;
                                    false
                                }
                            },
                    );

                    // If we're available for accepting more requests after the
                    // deduction above, then try to accept a new task. If that's
                    // successful then push it into `futures` and turn this loop
                    // again to see where we're at next time around.
                    if self.available
                        && let Poll::Ready(Some(req)) = incoming_requests.as_mut().poll_next(cx)
                    {
                        accept_request(req, &mut futures, &mut start_times, &mut reuse_count);
                        continue;
                    }

                    // If, at this point, we still have some requests that are
                    // being processed then go ahead and bail out of this
                    // singular call to `poll` by saying we're not ready yet.
                    // This means we unconditionally wait for events within
                    // `futures` and we're also registered, optionally, for
                    // listening for incoming connections. That's all the events
                    // we're interested in, so this iteration of `poll` is complete.
                    if !futures.is_empty() {
                        break Poll::Pending;
                    }

                    // At this point `futures` is empty, and we haven't gotten
                    // any incoming tasks. Check the store we're using to see if
                    // there are any "interesting" tasks around. These are tasks
                    // which act as effectively strong references to this worker
                    // to keep it running. If there are still interesting tasks,
                    // then we're done with this iteration of `poll`. We'll get
                    // woken up when anything changes, but otherwise it's time
                    // to let something else happen.
                    if accessor.poll_no_interesting_tasks(cx).is_pending() {
                        break Poll::Pending;
                    }

                    // And now at this point we (a) have no `futures`, (b) no
                    // new requests are available, and (c) the store is
                    // completely devoid of interesting work. In this situation
                    // if we're not actually capable of accepting any more work,
                    // then we're completely done and it's time to exit this
                    // worker.
                    if !may_accept {
                        break Poll::Ready(Ok(()));
                    }

                    // Finally, at this point we're idle but still eligible to
                    // accept new work, so update the state if appropriate and
                    // then return pending while we wait for new work.
                    {
                        let mut status = status.try_lock().unwrap();
                        if status.0 != WorkerStatus::Idle {
                            *status = (WorkerStatus::Idle, Instant::now());
                        }
                    }
                    break Poll::Pending;
                }
            })
            .await
        };

        let result = {
            let mut future = pin!(
                dropper
                    .store
                    .as_mut()
                    .unwrap()
                    .run_concurrent(function)
                    .map(|v| v.flatten())
            );

            future::poll_fn(|cx| {
                let poll = future.as_mut().poll(cx);
                if poll.is_pending() {
                    // If the future returns `Pending`, that's either because it's
                    // idle (in which case it can definitely accept a new request) or
                    // because all its tasks are awaiting I/O, in which case it may
                    // have capacity for additional tasks to run concurrently.
                    //
                    // However, per #11869 and #11870, if one of the tasks is
                    // blocked on a sync call to a host function which has exclusive
                    // access to the `Store`, the `StoreContextMut::run_concurrent`
                    // event loop will be unable to make progress until that call
                    // finishes.  Similarly, if the task loops indefinitely, subject
                    // only to epoch interruption, the event loop will also be
                    // stuck.  Either way, any request expirations created inside
                    // the `AsyncFnOnce` we passed to `run_concurrent` won't have a
                    // chance to trigger.  Consequently, we poll for instance
                    // expiration here, outside the event loop, based on the most
                    // recently recorded state of the worker.

                    let (status, start) = *status.try_lock().unwrap();

                    if let Poll::Ready(()) = expiration.as_mut().poll(cx, status, start) {
                        return Poll::Ready(match status {
                            WorkerStatus::Requests | WorkerStatus::PostReturn => {
                                Err(format_err!("guest timed out"))
                            }
                            WorkerStatus::Idle => Ok(()),
                        });
                    }

                    // Otherwise, if the instance has not yet expired, we set
                    // `accept_concurrent` to true and, if it wasn't already true
                    // before, poll the future one more time so it can ask for
                    // another request if appropriate.
                    if !accept_concurrent.swap(true, Relaxed) {
                        return future.as_mut().poll(cx);
                    }
                }

                poll
            })
            .await
        };

        dropper.state.drop(dropper.store.take().unwrap(), result);
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
/// `S::WorkerState::should_accept_request` returns [`ShouldAccept::Yes`] more
/// than once for a given instance.  See [`WorkerState`] for details.
pub struct ProxyHandler<S: HandlerState>(Arc<ProxyHandlerInner<S>>);

impl<S: HandlerState> Clone for ProxyHandler<S> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

/// This error is returned if, when handling the request, a new worker and
/// associated instance needed to be created, but instantiation failed, e.g. due
/// to reaching a pooling allocator limit or running out of memory.  In this
/// case, the caller may be able to recover and retry (e.g. after waiting for
/// existing instances to be dropped and/or freeing memory used by caches,
/// etc.).  Otherwise, it will probably need to return an HTTP 500 error.
pub struct InstantiationError<T> {
    /// The ID of the request which was originally configured,
    pub request_id: T,
    /// The original request passed to `ProxyHandler::handle`.
    ///
    /// This is wrapped in a `Mutex` to satisfy the `Send + Sync` bounds
    /// required by `wasmtime::Error`.
    pub request: Mutex<Request>,
    /// The original instantiation error.
    ///
    /// This is wrapped in an `Arc` because a single instantiation error may
    /// affect multiple requests, and each caller will be given a clone.
    pub error: Arc<wasmtime::Error>,
}

impl<T> fmt::Display for InstantiationError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "instantiation error: {}", self.error)
    }
}

impl<T> fmt::Debug for InstantiationError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "instantiation error: {:?}", self.error)
    }
}

impl<T> error::Error for InstantiationError<T> {}

/// Returned when the guest failed to produce a response before the expiration
/// returned by `HandlerState::on_request_start` elapsed.
pub struct ExpirationError;

impl fmt::Display for ExpirationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        fmt::Debug::fmt(self, f)
    }
}

impl fmt::Debug for ExpirationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "guest timed out")
    }
}

impl error::Error for ExpirationError {}

/// A worker trapped or panicked and failed to produce a result.
pub struct TrapOrPanicError;

impl fmt::Display for TrapOrPanicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        fmt::Debug::fmt(self, f)
    }
}

impl fmt::Debug for TrapOrPanicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "worker trapped or panicked")
    }
}

impl error::Error for TrapOrPanicError {}

impl<S> ProxyHandler<S>
where
    S: HandlerState,
{
    /// Create a new `ProxyHandler` with the specified application state and
    /// pre-instance.
    pub fn new(state: S) -> Self {
        Self(Arc::new(ProxyHandlerInner {
            state,
            request_queue: Default::default(),
            worker_count: AtomicUsize::from(0),
        }))
    }

    /// Handle the specified request, returning a response on success or the
    /// tuple of the request and error on failure.
    ///
    /// This function will return a `wasmtime::Error` on failure, which may be
    /// downcast to a more specific type in certain scenarios:
    ///
    /// - [`InstantiationError`] if a new worker was created to handle the
    /// request but could not instantiate the guest component.
    ///
    /// - [`ExpirationError`] if the request expired before it produced a
    /// response.  See [`WorkerState::on_request_start`] for details.
    ///
    /// - [`TrapOrPanicError`] if the worker responsible for handling the
    /// request trapped or panicked before it produced a response.  This may be
    /// used when a trap occurs but cannot be traced to a specific request,
    /// e.g. during concurrent request handling.
    ///
    /// In other failure cases (e.g. `wasi:http/types#error-code` return values
    /// and/or traps when executing synchronous WASIp2 handler functions), the
    /// original error returned by the handler will be returned.
    ///
    /// # Backpressure
    ///
    /// Note that this API does not implement any form of backpressure to limit
    /// the number of in-flight `Request`s being processed. This function
    /// may spawn new tokio tasks, instantiate new modules under new stores, and
    /// queue up pending `Request`s while waiting for previous instances. In all
    /// of these situations invoking this function will consume some host-side
    /// resources until the request is done.
    ///
    /// Embedders using this API must ensure to take this into account. If an
    /// infinite number of requests can be fed into this function then it's
    /// recommended to take a semaphore, for example, around this function call
    /// to limit the number of concurrent requests that are being processed.
    pub async fn handle(
        &self,
        id: <S::WorkerState as WorkerState>::RequestId,
        request: Request,
    ) -> Result<Response, wasmtime::Error> {
        let (tx, rx) = oneshot::channel();
        let req = (id, request, tx);
        if self.0.worker_count.load(Relaxed) == 0 {
            // There are no available workers; skip the queue and pass
            // the request directly to the worker, which improves
            // performance as measured by `wasmtime-server-rps.sh` by
            // about 15%.
            self.start_worker(Some(req));
        } else {
            let mut queue = self.0.request_queue.queue.lock().unwrap();
            queue.push_back(req);

            // Start a new worker to handle the request if the last worker just
            // went unavailable.  See also `Worker::set_available` for what
            // happens if the available worker count goes to zero right after we
            // check it here, and note that we only check the count _after_
            // we've pushed the request to the queue.
            //
            // The upshot is that at least one (or more) of the
            // following will happen:
            //
            // - An existing worker will accept the request
            // - We'll start a new worker here to accept the request
            // - `Worker::set_available` will start a new worker to accept the request
            //
            // I.e. it should not be possible for the request to be orphaned
            // indefinitely in the queue without being accepted except in the
            // case of a panic or an instantiation error.  In the case of an
            // instantiation error, we'll give the request back to the caller in
            // an `Err(_)`, allowing the application to decide what to do next.
            if self.0.worker_count.load(Relaxed) == 0 {
                let req = queue.pop_back().unwrap();
                drop(queue);
                self.start_worker(Some(req));
            } else {
                drop(queue);
                self.0.request_queue.notify_push.notify_one();
            }
        }

        rx.await.map_err(|_| TrapOrPanicError)?
    }

    /// Return a reference to the application state.
    pub fn state(&self) -> &S {
        &self.0.state
    }

    fn start_worker(&self, request: Option<WorkerRequest<S>>) {
        tokio::spawn(
            Worker {
                handler: self.clone(),
                available: false,
            }
            .run(request),
        );
    }
}

/// Representation of a "prepared" call for a guest, used to extract the
/// `GuestTaskId` before actually executing any handlers.
///
/// Right now this is a bit gross since it has to type out a bunch of types by
/// hand.
pub enum Prepared<'a, T: 'static> {
    #[doc(hidden)]
    #[cfg(feature = "p2")]
    P2 {
        guest: &'a p2::bindings::Proxy,
        call: TypedFuncCallConcurrent<
            T,
            (
                Resource<p2_types::IncomingRequest>,
                Resource<p2_types::ResponseOutparam>,
            ),
            (),
        >,
        tx: Arc<Mutex<Option<oneshot::Sender<Result<Response, wasmtime::Error>>>>>,
    },
    #[doc(hidden)]
    #[cfg(feature = "p3")]
    P3 {
        guest: &'a p3::bindings::Service,
        call: TypedFuncCallConcurrent<
            T,
            (Resource<p3_types::Request>,),
            (Result<Resource<p3_types::Response>, p3_types::ErrorCode>,),
        >,
        tx: oneshot::Sender<Result<Response, wasmtime::Error>>,
        request_io_result: Pin<Box<dyn Future<Output = Result<(), p3_types::ErrorCode>> + Send>>,
        view: fn(&mut T) -> p3::WasiHttpCtxView,
    },
}

impl<'a, T: Send> Prepared<'a, T> {
    /// Creates a new prepared request.
    pub fn new(
        mut store: StoreContextMut<'_, T>,
        proxy: &'a Proxy,
        request: Request,
        view: ViewFn<T>,
        tx: oneshot::Sender<Result<Response, wasmtime::Error>>,
    ) -> Result<Prepared<'a, T>> {
        match (proxy, view) {
            #[cfg(feature = "p3")]
            (Proxy::P3(guest), ViewFn::P3(view)) => {
                let (request, body) = request.into_parts();
                let body = body.map_err(p3_types::ErrorCode::from);
                let request = http::Request::from_parts(request, body);
                let (request, request_io_result) = p3::Request::from_http(request);
                let request = view(store.data_mut()).table.push(request)?;

                Ok(Prepared::P3 {
                    tx,
                    request_io_result: Box::pin(request_io_result),
                    guest,
                    view,
                    call: guest
                        .wasi_http_handler()
                        .func_handle()
                        .start_call_concurrent(store, (request,))?,
                })
            }
            #[cfg(feature = "p2")]
            (Proxy::P2(guest), ViewFn::P2(view)) => {
                // Here we wrap the sender in an `Arc<Mutex<Option<_>>>`, with one
                // clone used in the `response-outparam` and the other used to send
                // an error if the request expires or the handler returns without
                // producing a response.
                let tx = Arc::new(Mutex::new(Some(tx)));

                let request =
                    view(store.data_mut()).new_incoming_request(p2_types::Scheme::Http, request)?;

                let out = view(store.data_mut()).new_response_outparam_from_callback({
                    let tx = tx.clone();
                    move |value| {
                        if let Some(tx) = tx.lock().unwrap().take() {
                            _ = tx.send(
                                value
                                    .map(|v| {
                                        v.map(move |body| {
                                            body.map_err(wasmtime::Error::from).boxed_unsync()
                                        })
                                    })
                                    .map_err(wasmtime::Error::from),
                            );
                        }
                    }
                })?;

                Ok(Prepared::P2 {
                    guest,
                    tx,
                    call: guest
                        .wasi_http_incoming_handler()
                        .func_handle()
                        .start_call_concurrent(store, (request, out))?,
                })
            }
            #[cfg(all(feature = "p2", feature = "p3"))]
            _ => unreachable!(),
        }
    }

    fn task(&self) -> GuestTaskId {
        match self {
            #[cfg(feature = "p3")]
            Prepared::P3 { call, .. } => call.task(),
            #[cfg(feature = "p2")]
            Prepared::P2 { call, .. } => call.task(),
        }
    }

    /// Executes this request to completion.
    pub async fn run(
        self,
        accessor: &Accessor<T>,
        expiration: impl Future<Output = ()>,
    ) -> Result<bool> {
        let expiration = pin!(expiration);

        match self {
            #[cfg(feature = "p3")]
            Prepared::P3 {
                guest,
                call,
                tx,
                request_io_result,
                view,
            } => {
                let handle =
                    pin!(async move {
                        let response = guest
                            .wasi_http_handler()
                            .func_handle()
                            .finish_call_concurrent(accessor, call)
                            .await?
                            .0?;

                        let response = accessor.with(|mut store| {
                            let response = view(store.get()).table.delete(response)?;
                            Ok::<_, wasmtime::Error>(response.into_http_with_getter(
                                &mut store,
                                request_io_result,
                                view,
                            )?)
                        })?;

                        Ok(response
                            .map(move |body| body.map_err(wasmtime::Error::from).boxed_unsync()))
                    });

                // TODO: We should also use `oneshot::Sender::poll_close` to be
                // notified when the receiver is dropped, in which case we should
                // expire the request since the response is no longer of interest to
                // the original `ProxyHandler::handle` caller.
                let (result, sent) = match futures::future::select(handle, expiration).await {
                    Either::Left((result, _)) => (result, true),
                    // TODO: We should also send a cancel request to the expired
                    // task to give it a chance to shut down gracefully, but as of
                    // this writing Wasmtime does not yet provide an API for doing
                    // that.  See issue #11833.  Instead, we let it continue running
                    // as a background task until it either returns a response
                    // (which we'll ignore) or the instance itself has expired.
                    Either::Right(((), _)) => (Err(ExpirationError.into()), false),
                };

                _ = tx.send(result);

                Ok(sent)
            }
            #[cfg(feature = "p2")]
            Prepared::P2 { guest, call, tx } => {
                let handle = pin!(
                    guest
                        .wasi_http_incoming_handler()
                        .func_handle()
                        .finish_call_concurrent(accessor, call)
                );

                const MESSAGE: &str = "guest never invoked `response-outparam::set` method";

                struct Dropper(
                    Arc<Mutex<Option<oneshot::Sender<Result<Response, wasmtime::Error>>>>>,
                );

                impl Drop for Dropper {
                    fn drop(&mut self) {
                        if let Some(tx) = self.0.lock().unwrap().take() {
                            _ = tx.send(Err(format_err!("{MESSAGE}")));
                        }
                    }
                }

                let tx = Dropper(tx);

                // See corresponding TODO comment for the p3 case above.
                let (result, sent) = match futures::future::select(handle, expiration).await {
                    Either::Left((result, _)) => (result.context(MESSAGE), true),
                    // See corresponding TODO comment for the p3 case above.
                    Either::Right(((), _)) => (Err(ExpirationError.into()), false),
                };

                if let Some(tx) = tx.0.lock().unwrap().take() {
                    _ = tx.send(result.and_then(|()| Err(format_err!("{MESSAGE}"))));
                }

                Ok(sent)
            }
        }
    }
}
