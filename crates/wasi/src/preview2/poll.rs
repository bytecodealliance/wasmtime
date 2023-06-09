use crate::preview2::{
    wasi::poll::poll::{self, Pollable},
    Table, TableError, WasiView,
};
use anyhow::{anyhow, Result};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct HostPollable(
    Box<dyn Fn() -> Pin<Box<dyn Future<Output = Result<()>> + Send + Sync>> + Send + Sync>,
);

impl HostPollable {
    pub fn new(
        mkfuture: impl Fn() -> Pin<Box<dyn Future<Output = Result<()>> + Send + Sync + 'static>>
            + Send
            + Sync
            + 'static,
    ) -> HostPollable {
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
        struct PollOneoff<'a> {
            table: &'a mut Table,
            elems: &'a [Pollable],
        }
        impl<'a> Future for PollOneoff<'a> {
            type Output = Result<Vec<bool>>;
            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                let mut any_ready = false;
                let mut results = vec![false; self.elems.len()];
                for (ix, pollable) in self.elems.iter().enumerate() {
                    match self.table.get_host_pollable_mut(*pollable) {
                        Ok(mkf) => {
                            let mut f = mkf.0();
                            match Pin::new(&mut f).poll(cx) {
                                Poll::Ready(Ok(())) => {
                                    results[ix] = true;
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
                        Err(e) => {
                            return Poll::Ready(Err(
                                anyhow!(e).context(format!("poll_oneoff[{ix}]: {pollable}"))
                            ))
                        }
                    }
                }
                if any_ready {
                    Poll::Ready(Ok(results))
                } else {
                    Poll::Pending
                }
            }
        }

        let bs = Pin::new(&mut PollOneoff {
            table: self.table_mut(),
            elems: &pollables,
        })
        .await?;
        Ok(bs.into_iter().map(|b| if b { 1 } else { 0 }).collect())
    }
}
