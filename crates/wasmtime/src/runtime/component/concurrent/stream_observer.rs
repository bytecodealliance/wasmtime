//! Watching what streams carry from one component to another: see [`StreamObserver`].

use crate::component::Val;
use crate::{AsContextMut, Store, StoreContextMut};
use alloc::boxed::Box;

/// A stream, as a [`StreamObserver`] is told of it: one for both its ends, for its life.
///
/// [`StreamAny::id`] gives a stream's, for the embedder to know which it is told of.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct StreamId(pub(crate) u32);

/// Told of the items streams carry from one component instance's writer to another's reader,
/// as the runtime copies them, for an embedder to watch them: a debugger previewing a stream,
/// say. The reader gets them as it would have.
///
/// The runtime copies items of a "flat" payload (no pointers or handles) as bytes; it lifts
/// them into [`Val`]s only for the streams [`observes`](Self::observes) says yes to, so a
/// stream nobody watches costs nothing more.
pub trait StreamObserver: Send + Sync + 'static {
    /// Whether to be told of `stream`'s items.
    fn observes(&mut self, stream: StreamId) -> bool;
    /// `stream` carried `items`, in order, to its reader.
    fn items(&mut self, stream: StreamId, items: &[Val]);
}

impl<T> Store<T> {
    /// Convenience wrapper for [`StoreContextMut::stream_observer`].
    pub fn stream_observer(&mut self, observer: impl StreamObserver) {
        self.as_context_mut().stream_observer(observer);
    }
}

impl<T> StoreContextMut<'_, T> {
    /// Set a [`StreamObserver`] for this store, overwriting any set before.
    pub fn stream_observer(self, observer: impl StreamObserver) {
        self.0
            .concurrent_state_mut_without_forcing_current_thread()
            .stream_observer = Some(Box::new(observer));
    }
}
