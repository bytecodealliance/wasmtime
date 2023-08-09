use crate::preview2::{
    bindings::poll::poll::{self, Pollable},
    Table, TableError, WasiView,
};
use anyhow::Result;
use std::any::Any;
use std::collections::{hash_map::Entry, HashMap};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub type PollableFuture<'a> = Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;
pub type MakeFuture = for<'a> fn(&'a mut dyn Any) -> PollableFuture<'a>;
pub type ClosureFuture = Box<dyn Fn() -> PollableFuture<'static> + Send + Sync + 'static>;

/// A host representation of the `wasi:poll/poll.pollable` resource.
///
/// A pollable is not the same thing as a Rust Future: the same pollable may be used to
/// repeatedly check for readiness of a given condition, e.g. if a stream is readable
/// or writable. So, rather than containing a Future, which can only become Ready once, a
/// HostPollable contains a way to create a Future in each call to poll_oneoff.
pub enum HostPollable {
    /// Create a Future by calling a fn on another resource in the table. This
    /// indirection means the created Future can use a mut borrow of another
    /// resource in the Table (e.g. a stream)
    TableEntry { index: u32, make_future: MakeFuture },
    /// Create a future by calling an owned, static closure. This is used for
    /// pollables which do not share state with another resource in the Table
    /// (e.g. a timer)
    Closure(ClosureFuture),
}

pub trait TablePollableExt {
    fn push_host_pollable(&mut self, p: HostPollable) -> Result<u32, TableError>;
    fn get_host_pollable_mut(&mut self, fd: u32) -> Result<&mut HostPollable, TableError>;
    fn delete_host_pollable(&mut self, fd: u32) -> Result<HostPollable, TableError>;
}

impl TablePollableExt for Table {
    fn push_host_pollable(&mut self, p: HostPollable) -> Result<u32, TableError> {
        match p {
            HostPollable::TableEntry { index, .. } => self.push_child(Box::new(p), index),
            HostPollable::Closure { .. } => self.push(Box::new(p)),
        }
    }
    fn get_host_pollable_mut(&mut self, fd: u32) -> Result<&mut HostPollable, TableError> {
        self.get_mut::<HostPollable>(fd)
    }
    fn delete_host_pollable(&mut self, fd: u32) -> Result<HostPollable, TableError> {
        self.delete::<HostPollable>(fd)
    }
}

#[async_trait::async_trait]
impl<T: WasiView> poll::Host for T {
    async fn drop_pollable(&mut self, pollable: Pollable) -> Result<()> {
        self.table_mut().delete_host_pollable(pollable)?;
        Ok(())
    }

    async fn poll_oneoff(&mut self, pollables: Vec<Pollable>) -> Result<Vec<bool>> {
        type ReadylistIndex = usize;

        let table = self.table_mut();

        let mut table_futures: HashMap<u32, (MakeFuture, Vec<ReadylistIndex>)> = HashMap::new();
        let mut closure_futures: Vec<(PollableFuture<'_>, Vec<ReadylistIndex>)> = Vec::new();

        for (ix, p) in pollables.iter().enumerate() {
            match table.get_host_pollable_mut(*p)? {
                HostPollable::Closure(f) => closure_futures.push((f(), vec![ix])),
                HostPollable::TableEntry { index, make_future } => {
                    match table_futures.entry(*index) {
                        Entry::Vacant(v) => {
                            v.insert((*make_future, vec![ix]));
                        }
                        Entry::Occupied(mut o) => {
                            let (_, v) = o.get_mut();
                            v.push(ix);
                        }
                    }
                }
            }
        }

        for (entry, (make_future, readylist_indices)) in table.iter_entries(table_futures) {
            let entry = entry?;
            closure_futures.push((make_future(entry), readylist_indices));
        }

        struct PollOneoff<'a> {
            elems: Vec<(PollableFuture<'a>, Vec<ReadylistIndex>)>,
        }
        impl<'a> Future for PollOneoff<'a> {
            type Output = Result<Vec<bool>>;

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                let mut any_ready = false;
                let mut results = vec![false; self.elems.len()];
                for (fut, readylist_indicies) in self.elems.iter_mut() {
                    match fut.as_mut().poll(cx) {
                        Poll::Ready(Ok(())) => {
                            for r in readylist_indicies {
                                results[*r] = true;
                            }
                            any_ready = true;
                        }
                        Poll::Ready(Err(e)) => {
                            return Poll::Ready(Err(
                                e.context(format!("poll_oneoff {readylist_indicies:?}"))
                            ));
                        }
                        Poll::Pending => {}
                    }
                }
                if any_ready {
                    Poll::Ready(Ok(results))
                } else {
                    Poll::Pending
                }
            }
        }

        Ok(PollOneoff {
            elems: closure_futures,
        }
        .await?)
    }
}

pub mod sync {
    use crate::preview2::{
        bindings::poll::poll::Host as AsyncHost,
        bindings::sync_io::poll::poll::{self, Pollable},
        in_tokio, WasiView,
    };
    use anyhow::Result;

    impl<T: WasiView> poll::Host for T {
        fn drop_pollable(&mut self, pollable: Pollable) -> Result<()> {
            in_tokio(async { AsyncHost::drop_pollable(self, pollable).await })
        }

        fn poll_oneoff(&mut self, pollables: Vec<Pollable>) -> Result<Vec<bool>> {
            in_tokio(async { AsyncHost::poll_oneoff(self, pollables).await })
        }
    }
}
