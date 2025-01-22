use {
    crate::AsContextMut,
    anyhow::Result,
    futures::{stream::FuturesUnordered, FutureExt},
    std::{boxed::Box, future::Future, pin::Pin},
};

pub use futures_and_streams::{ErrorContext, FutureReader, StreamReader};

mod futures_and_streams;

/// Represents the result of a concurrent operation.
///
/// This is similar to a [`std::future::Future`] except that it represents an
/// operation which requires exclusive access to a store in order to make
/// progress -- without monopolizing that store for the lifetime of the
/// operation.
pub struct Promise<T>(Pin<Box<dyn Future<Output = T> + Send + Sync + 'static>>);

impl<T: 'static> Promise<T> {
    /// Map the result of this `Promise` from one value to another.
    pub fn map<U>(self, fun: impl FnOnce(T) -> U + Send + Sync + 'static) -> Promise<U> {
        Promise(Box::pin(self.0.map(fun)))
    }

    /// Convert this `Promise` to a future which may be `await`ed for its
    /// result.
    ///
    /// The returned future will require exclusive use of the store until it
    /// completes.  If you need to await more than one `Promise` concurrently,
    /// use [`PromisesUnordered`].
    pub async fn get<U: Send>(self, store: impl AsContextMut<Data = U>) -> Result<T> {
        _ = store;
        todo!()
    }

    /// Convert this `Promise` to a future which may be `await`ed for its
    /// result.
    ///
    /// Unlike [`Self::get`], this does _not_ take a store parameter, meaning
    /// the returned future will not make progress until and unless the event
    /// loop for the store it came from is polled.  Thus, this method should
    /// only be used from within host functions and not from top-level embedder
    /// code.
    pub fn into_future(self) -> Pin<Box<dyn Future<Output = T> + Send + Sync + 'static>> {
        self.0
    }
}

/// Represents a collection of zero or more concurrent operations.
///
/// Similar to [`futures::stream::FuturesUnordered`], this type supports
/// `await`ing more than one [`Promise`]s concurrently.
pub struct PromisesUnordered<T>(
    FuturesUnordered<Pin<Box<dyn Future<Output = T> + Send + Sync + 'static>>>,
);

impl<T: 'static> PromisesUnordered<T> {
    /// Create a new `PromisesUnordered` with no entries.
    pub fn new() -> Self {
        Self(FuturesUnordered::new())
    }

    /// Add the specified [`Promise`] to this collection.
    pub fn push(&mut self, promise: Promise<T>) {
        self.0.push(promise.0)
    }

    /// Get the next result from this collection, if any.
    pub async fn next<U: Send>(&mut self, store: impl AsContextMut<Data = U>) -> Result<Option<T>> {
        _ = store;
        todo!()
    }
}
