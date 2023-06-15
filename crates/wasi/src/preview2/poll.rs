use crate::preview2::{
    wasi::poll::poll::{self, Pollable},
    Table, TableError, WasiView,
};
use anyhow::{anyhow, Result};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

type ReadynessFuture = Pin<Box<dyn Future<Output = Result<()>> + Send + Sync>>;

pub struct HostPollable(Box<dyn Fn() -> ReadynessFuture + Send + Sync>);

impl HostPollable {
    pub fn new(mkfuture: impl Fn() -> ReadynessFuture + Send + Sync + 'static) -> HostPollable {
        HostPollable(Box::new(mkfuture))
    }
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
        struct PollOneoff {
            elems: Vec<(u32, ReadynessFuture)>,
        }
        impl Future for PollOneoff {
            type Output = Result<Vec<u8>>;

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                let mut any_ready = false;
                let mut results = vec![0; self.elems.len()];
                for (ix, (pollable, f)) in self.elems.iter_mut().enumerate() {
                    // NOTE: we don't need to guard against polling any of the elems more than
                    // once, as a single one becoming ready will cause the whole PollOneoff future
                    // to become ready.
                    match f.as_mut().poll(cx) {
                        Poll::Ready(Ok(())) => {
                            results[ix] = 1;
                            any_ready = true;
                        }
                        Poll::Ready(Err(e)) => {
                            return Poll::Ready(Err(
                                e.context(format!("poll_oneoff[{ix}]: {pollable}"))
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
            elems: pollables
                .iter()
                .enumerate()
                .map(
                    |(ix, pollable)| match self.table_mut().get_host_pollable_mut(*pollable) {
                        Ok(mkf) => Ok((*pollable, mkf.0())),
                        Err(e) => Err(anyhow!(e).context(format!("poll_oneoff[{ix}]: {pollable}"))),
                    },
                )
                .collect::<Result<Vec<_>>>()?,
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
