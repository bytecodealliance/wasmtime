use std::mem::{self, ManuallyDrop};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

/// Handle to a task which may be used to join on the result of executing it.
///
/// This represents a handle to a running task which can be cancelled with
/// [`JoinHandle::abort`]. The final result and drop of the task can be
/// determined by `await`-ing this handle.
///
/// Note that dropping this handle does not affect the running task it's
/// connected to. A manual invocation of [`JoinHandle::abort`] is required to
/// affect the task.
pub struct JoinHandle {
    state: Arc<Mutex<JoinState>>,
}

enum JoinState {
    /// The task this is connected to is still running and has not completed or
    /// been dropped.
    Running {
        /// The waker that the running task has registered which is signaled
        /// upon abort.
        waiting_for_abort_signal: Option<Waker>,

        /// The waker that the `JoinHandle` has registered to await
        /// destruction of the running task itself.
        waiting_for_abort_to_complete: Option<Waker>,
    },

    /// An abort as been requested through an `JoinHandle`. The task specified
    /// here is used for `Future for JoinHandle`.
    AbortRequested {
        waiting_for_abort_to_complete: Option<Waker>,
    },

    /// The running task has completed, so no need to abort it and nothing else
    /// needs to wait.
    Complete,
}

impl JoinHandle {
    /// Abort the task.
    ///
    /// This flags the connected task should abort in the near future, but note
    /// that if this is called while the future is being polled then that call
    /// will still complete.
    ///
    /// Note that this `JoinHandle` is itself a `Future` and can be used to
    /// await the result and destruction of the task that this is associated
    /// with.
    pub fn abort(&self) {
        let mut state = self.state.lock().unwrap();

        match &mut *state {
            // If this task is still running, then fall through to below to
            // transition it into the `AbortRequested` state. If present the
            // waker for the running task is notified to indicate that an abort
            // signal has been received.
            JoinState::Running {
                waiting_for_abort_signal,
                waiting_for_abort_to_complete,
            } => {
                if let Some(task) = waiting_for_abort_signal.take() {
                    task.wake();
                }

                *state = JoinState::AbortRequested {
                    waiting_for_abort_to_complete: waiting_for_abort_to_complete.take(),
                };
            }

            // If this task has already been aborted or has completed, nothing
            // is left to do.
            JoinState::AbortRequested { .. } | JoinState::Complete => {}
        }
    }

    /// Wraps the `future` provided in a new future which is "abortable" where
    /// if the returned `JoinHandle` is flagged then the future will resolve
    /// ASAP with `None` and drop the provided `future`.
    pub(crate) fn run<F>(future: F) -> (JoinHandle, impl Future<Output = Option<F::Output>>)
    where
        F: Future,
    {
        let handle = JoinHandle {
            state: Arc::new(Mutex::new(JoinState::Running {
                waiting_for_abort_signal: None,
                waiting_for_abort_to_complete: None,
            })),
        };
        let future = JoinHandleFuture {
            future: ManuallyDrop::new(future),
            state: handle.state.clone(),
        };
        (handle, future)
    }
}

impl Future for JoinHandle {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.state.lock().unwrap();
        match &mut *state {
            // If this task is running or still only has requested an abort,
            // wait further for the task to get dropped.
            JoinState::Running {
                waiting_for_abort_to_complete,
                ..
            }
            | JoinState::AbortRequested {
                waiting_for_abort_to_complete,
            } => {
                *waiting_for_abort_to_complete = Some(cx.waker().clone());
                Poll::Pending
            }

            // The task is dropped, done!
            JoinState::Complete => Poll::Ready(()),
        }
    }
}

struct JoinHandleFuture<F> {
    future: ManuallyDrop<F>,
    state: Arc<Mutex<JoinState>>,
}

impl<F> Future for JoinHandleFuture<F>
where
    F: Future,
{
    type Output = Option<F::Output>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // SAFETY: this is a pin-projection from `Self` to the state and `Pin`
        // of the internal future. This is the exclusive access of these fields
        // apart from the destructor and should be safe.
        let (state, future) = unsafe {
            let me = self.get_unchecked_mut();
            (&me.state, Pin::new_unchecked(&mut *me.future))
        };

        // First, before polling the future, check to see if we've been
        // aborted. If not register our task as awaiting such an abort.
        {
            let mut state = state.lock().unwrap();
            match &mut *state {
                JoinState::Running {
                    waiting_for_abort_signal,
                    ..
                } => {
                    *waiting_for_abort_signal = Some(cx.waker().clone());
                }
                JoinState::AbortRequested { .. } | JoinState::Complete => {
                    return Poll::Ready(None);
                }
            }
        }

        future.poll(cx).map(Some)
    }
}

impl<F> Drop for JoinHandleFuture<F> {
    fn drop(&mut self) {
        // SAFETY: this is the exclusive owner of this future and it's safe to
        // drop here during the owning destructor.
        //
        // Note that this explicitly happens before notifying the abort handle
        // that the task completed so that when the notification goes through
        // it's guaranteed that the future has been destroyed.
        unsafe {
            ManuallyDrop::drop(&mut self.future);
        }

        // After the future dropped see if there was a task awaiting its
        // destruction. Simultaneously flag this state as complete.
        let prev = mem::replace(&mut *self.state.lock().unwrap(), JoinState::Complete);
        let task = match prev {
            JoinState::Running {
                waiting_for_abort_to_complete,
                ..
            }
            | JoinState::AbortRequested {
                waiting_for_abort_to_complete,
            } => waiting_for_abort_to_complete,
            JoinState::Complete => None,
        };
        if let Some(task) = task {
            task.wake();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::JoinHandle;
    use std::pin::{Pin, pin};
    use std::task::{Context, Poll, Waker};
    use tokio::sync::oneshot;

    fn is_ready<F>(future: Pin<&mut F>) -> bool
    where
        F: Future,
    {
        match future.poll(&mut Context::from_waker(Waker::noop())) {
            Poll::Ready(_) => true,
            Poll::Pending => false,
        }
    }

    #[tokio::test]
    async fn abort_in_progress() {
        let (tx, rx) = oneshot::channel::<()>();
        let (mut handle, future) = JoinHandle::run(rx);
        let mut handle = Pin::new(&mut handle);
        {
            let mut future = pin!(future);
            assert!(!is_ready(future.as_mut()));
            assert!(!is_ready(handle.as_mut()));
            handle.abort();
            assert!(is_ready(future.as_mut()));
            assert!(!is_ready(handle.as_mut()));
            assert!(!tx.is_closed());
        }
        assert!(is_ready(handle.as_mut()));
        assert!(tx.is_closed());
    }

    #[tokio::test]
    async fn abort_complete() {
        let (tx, rx) = oneshot::channel::<()>();
        let (mut handle, future) = JoinHandle::run(rx);
        let mut handle = Pin::new(&mut handle);
        tx.send(()).unwrap();
        assert!(!is_ready(handle.as_mut()));
        {
            let mut future = pin!(future);
            assert!(is_ready(future.as_mut()));
            assert!(!is_ready(handle.as_mut()));
        }
        assert!(is_ready(handle.as_mut()));
        handle.abort();
        assert!(is_ready(handle.as_mut()));
    }

    #[tokio::test]
    async fn abort_dropped() {
        let (tx, rx) = oneshot::channel::<()>();
        let (mut handle, future) = JoinHandle::run(rx);
        let mut handle = Pin::new(&mut handle);
        drop(future);
        assert!(is_ready(handle.as_mut()));
        handle.abort();
        assert!(is_ready(handle.as_mut()));
        assert!(tx.is_closed());
    }

    #[tokio::test]
    async fn await_completion() {
        let (tx, rx) = oneshot::channel::<()>();
        tx.send(()).unwrap();
        let (handle, future) = JoinHandle::run(rx);
        let task = tokio::task::spawn(future);
        handle.await;
        task.await.unwrap();
    }

    #[tokio::test]
    async fn await_abort() {
        let (tx, rx) = oneshot::channel::<()>();
        tx.send(()).unwrap();
        let (handle, future) = JoinHandle::run(rx);
        handle.abort();
        let task = tokio::task::spawn(future);
        handle.await;
        task.await.unwrap();
    }
}
