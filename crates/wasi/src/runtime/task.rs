use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
/// Exactly like a [`tokio::task::JoinHandle`], except that it aborts the task when
/// the handle is dropped.
///
/// This behavior makes it easier to tie a worker task to the lifetime of a Resource
/// by keeping this handle owned by the Resource.
#[derive(Debug)]
pub struct AbortOnDropJoinHandle<T>(tokio::task::JoinHandle<T>);
impl<T> AbortOnDropJoinHandle<T> {
    /// Abort the task and wait for it to finish. Optionally returns the result
    /// of the task if it ran to completion prior to being aborted.
    pub(crate) async fn cancel(mut self) -> Option<T> {
        self.0.abort();

        match (&mut self.0).await {
            Ok(value) => Some(value),
            Err(err) if err.is_cancelled() => None,
            Err(err) => std::panic::resume_unwind(err.into_panic()),
        }
    }
}
impl<T> Drop for AbortOnDropJoinHandle<T> {
    fn drop(&mut self) {
        self.0.abort()
    }
}
impl<T> std::ops::Deref for AbortOnDropJoinHandle<T> {
    type Target = tokio::task::JoinHandle<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T> std::ops::DerefMut for AbortOnDropJoinHandle<T> {
    fn deref_mut(&mut self) -> &mut tokio::task::JoinHandle<T> {
        &mut self.0
    }
}
impl<T> From<tokio::task::JoinHandle<T>> for AbortOnDropJoinHandle<T> {
    fn from(jh: tokio::task::JoinHandle<T>) -> Self {
        AbortOnDropJoinHandle(jh)
    }
}
impl<T> Future for AbortOnDropJoinHandle<T> {
    type Output = T;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.as_mut().0).poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(r) => Poll::Ready(r.expect("child task panicked")),
        }
    }
}
