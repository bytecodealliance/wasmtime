use crate::preview2::{
    wasi::poll::poll::{self, Pollable},
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

pub enum HostPollable {
    TableEntry { index: u32, make_future: MakeFuture },
    Closure(ClosureFuture),
}

pub trait TablePollableExt {
    fn push_host_pollable(&mut self, p: HostPollable) -> Result<u32, TableError>;
    fn get_host_pollable_mut(&mut self, fd: u32) -> Result<&mut HostPollable, TableError>;
    fn delete_host_pollable(&mut self, fd: u32) -> Result<HostPollable, TableError>;
}

impl TablePollableExt for Table {
    fn push_host_pollable(&mut self, p: HostPollable) -> Result<u32, TableError> {
        self.push(Box::new(p))
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

    async fn poll_oneoff(&mut self, pollables: Vec<Pollable>) -> Result<Vec<u8>> {
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
            type Output = Result<Vec<u8>>;

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                let mut any_ready = false;
                let mut results = vec![0; self.elems.len()];
                for (fut, readylist_indicies) in self.elems.iter_mut() {
                    match fut.as_mut().poll(cx) {
                        Poll::Ready(Ok(())) => {
                            for r in readylist_indicies {
                                results[*r] = 1;
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
        wasi::poll::poll::Host as AsyncHost,
        wasi::sync_io::poll::poll::{self, Pollable},
        WasiView,
    };
    use anyhow::Result;
    use std::future::Future;
    use tokio::runtime::{Builder, Handle, Runtime};

    pub fn block_on<F: Future>(f: F) -> F::Output {
        match Handle::try_current() {
            Ok(h) => h.block_on(f),
            Err(_) => {
                use once_cell::sync::Lazy;
                static RUNTIME: Lazy<Runtime> =
                    Lazy::new(|| Builder::new_current_thread().enable_time().build().unwrap());
                let _enter = RUNTIME.enter();
                RUNTIME.block_on(f)
            }
        }
    }

    impl<T: WasiView> poll::Host for T {
        fn drop_pollable(&mut self, pollable: Pollable) -> Result<()> {
            block_on(async { AsyncHost::drop_pollable(self, pollable).await })
        }

        fn poll_oneoff(&mut self, pollables: Vec<Pollable>) -> Result<Vec<u8>> {
            block_on(async { AsyncHost::poll_oneoff(self, pollables).await })
        }
    }
}
