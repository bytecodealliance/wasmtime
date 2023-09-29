use crate::preview2::{
    bindings::io::poll::{self, Pollable},
    Table, TableError, WasiView,
};
use anyhow::Result;
use std::any::Any;
use std::collections::{hash_map::Entry, HashMap};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use wasmtime::component::Resource;

pub type PollableFuture<'a> = Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;
pub type MakeFuture = for<'a> fn(&'a mut dyn Any) -> PollableFuture<'a>;
pub type ClosureFuture = Box<dyn Fn() -> PollableFuture<'static> + Send + Sync + 'static>;

/// A host representation of the `wasi:io/poll.pollable` resource.
///
/// A pollable is not the same thing as a Rust Future: the same pollable may be used to
/// repeatedly check for readiness of a given condition, e.g. if a stream is readable
/// or writable. So, rather than containing a Future, which can only become Ready once, a
/// HostPollable contains a way to create a Future in each call to `poll_list`.
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
    fn push_host_pollable(&mut self, p: HostPollable) -> Result<Resource<Pollable>, TableError>;
    fn get_host_pollable_mut(
        &mut self,
        fd: &Resource<Pollable>,
    ) -> Result<&mut HostPollable, TableError>;
    fn delete_host_pollable(&mut self, fd: Resource<Pollable>) -> Result<HostPollable, TableError>;
}

impl TablePollableExt for Table {
    fn push_host_pollable(&mut self, p: HostPollable) -> Result<Resource<Pollable>, TableError> {
        Ok(Resource::new_own(match p {
            HostPollable::TableEntry { index, .. } => self.push_child(Box::new(p), index)?,
            HostPollable::Closure { .. } => self.push(Box::new(p))?,
        }))
    }
    fn get_host_pollable_mut(
        &mut self,
        fd: &Resource<Pollable>,
    ) -> Result<&mut HostPollable, TableError> {
        self.get_mut::<HostPollable>(fd.rep())
    }
    fn delete_host_pollable(&mut self, fd: Resource<Pollable>) -> Result<HostPollable, TableError> {
        self.delete::<HostPollable>(fd.rep())
    }
}

#[async_trait::async_trait]
impl<T: WasiView> poll::Host for T {
    async fn poll_list(&mut self, pollables: Vec<Resource<Pollable>>) -> Result<Vec<u32>> {
        type ReadylistIndex = u32;

        let table = self.table_mut();

        let mut table_futures: HashMap<u32, (MakeFuture, Vec<ReadylistIndex>)> = HashMap::new();
        let mut closure_futures: Vec<(PollableFuture<'_>, Vec<ReadylistIndex>)> = Vec::new();

        for (ix, p) in pollables.iter().enumerate() {
            let ix: u32 = ix.try_into()?;
            match table.get_host_pollable_mut(&p)? {
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

        struct PollList<'a> {
            elems: Vec<(PollableFuture<'a>, Vec<ReadylistIndex>)>,
        }
        impl<'a> Future for PollList<'a> {
            type Output = Result<Vec<u32>>;

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                let mut any_ready = false;
                let mut results = Vec::new();
                for (fut, readylist_indicies) in self.elems.iter_mut() {
                    match fut.as_mut().poll(cx) {
                        Poll::Ready(Ok(())) => {
                            results.extend_from_slice(readylist_indicies);
                            any_ready = true;
                        }
                        Poll::Ready(Err(e)) => {
                            return Poll::Ready(Err(
                                e.context(format!("poll_list {readylist_indicies:?}"))
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

        Ok(PollList {
            elems: closure_futures,
        }
        .await?)
    }

    async fn poll_one(&mut self, pollable: Resource<Pollable>) -> Result<()> {
        use anyhow::Context;

        let table = self.table_mut();

        let closure_future = match table.get_host_pollable_mut(&pollable)? {
            HostPollable::Closure(f) => f(),
            HostPollable::TableEntry { index, make_future } => {
                let index = *index;
                let make_future = *make_future;
                make_future(table.get_as_any_mut(index)?)
            }
        };

        closure_future.await.context("poll_one")
    }
}

#[async_trait::async_trait]
impl<T: WasiView> crate::preview2::bindings::io::poll::HostPollable for T {
    fn drop(&mut self, pollable: Resource<Pollable>) -> Result<()> {
        self.table_mut().delete_host_pollable(pollable)?;
        Ok(())
    }
}

pub mod sync {
    use crate::preview2::{
        bindings::io::poll::{Host as AsyncHost, HostPollable as AsyncHostPollable},
        bindings::sync_io::io::poll::{self, Pollable},
        in_tokio, WasiView,
    };
    use anyhow::Result;
    use wasmtime::component::Resource;

    impl<T: WasiView> poll::Host for T {
        fn poll_list(&mut self, pollables: Vec<Resource<Pollable>>) -> Result<Vec<u32>> {
            in_tokio(async { AsyncHost::poll_list(self, pollables).await })
        }

        fn poll_one(&mut self, pollable: Resource<Pollable>) -> Result<()> {
            in_tokio(async { AsyncHost::poll_one(self, pollable).await })
        }
    }

    impl<T: WasiView> crate::preview2::bindings::sync_io::io::poll::HostPollable for T {
        fn drop(&mut self, pollable: Resource<Pollable>) -> Result<()> {
            AsyncHostPollable::drop(self, pollable)
        }
    }
}
