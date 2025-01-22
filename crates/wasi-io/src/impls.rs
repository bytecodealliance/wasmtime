use crate::bindings::wasi::io::{error, poll, streams};
use crate::poll::{subscribe, DynFuture, DynPollable, MakeFuture};
use crate::streams::{DynInputStream, DynOutputStream, StreamError, StreamResult};
use crate::{IoImpl, IoView};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use anyhow::{anyhow, Result};
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use wasmtime::component::Resource;

impl<T: IoView> poll::Host for IoImpl<T> {
    async fn poll(&mut self, pollables: Vec<Resource<DynPollable>>) -> Result<Vec<u32>> {
        type ReadylistIndex = u32;

        if pollables.is_empty() {
            return Err(anyhow!("empty poll list"));
        }

        let table = self.table();

        let mut table_futures: BTreeMap<u32, (MakeFuture, Vec<ReadylistIndex>)> = BTreeMap::new();

        for (ix, p) in pollables.iter().enumerate() {
            let ix: u32 = ix.try_into()?;

            let pollable = table.get(p)?;
            let (_, list) = table_futures
                .entry(pollable.index)
                .or_insert((pollable.make_future, Vec::new()));
            list.push(ix);
        }

        let mut futures: Vec<(DynFuture<'_>, Vec<ReadylistIndex>)> = Vec::new();
        for (entry, (make_future, readylist_indices)) in table.iter_entries(table_futures) {
            let entry = entry?;
            futures.push((make_future(entry), readylist_indices));
        }

        struct PollList<'a> {
            futures: Vec<(DynFuture<'a>, Vec<ReadylistIndex>)>,
        }
        impl<'a> Future for PollList<'a> {
            type Output = Vec<u32>;

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                let mut any_ready = false;
                let mut results = Vec::new();
                for (fut, readylist_indicies) in self.futures.iter_mut() {
                    match fut.as_mut().poll(cx) {
                        Poll::Ready(()) => {
                            results.extend_from_slice(readylist_indicies);
                            any_ready = true;
                        }
                        Poll::Pending => {}
                    }
                }
                if any_ready {
                    Poll::Ready(results)
                } else {
                    Poll::Pending
                }
            }
        }

        Ok(PollList { futures }.await)
    }
}

impl<T: IoView> crate::bindings::wasi::io::poll::HostPollable for IoImpl<T> {
    async fn block(&mut self, pollable: Resource<DynPollable>) -> Result<()> {
        let table = self.table();
        let pollable = table.get(&pollable)?;
        let ready = (pollable.make_future)(table.get_any_mut(pollable.index)?);
        ready.await;
        Ok(())
    }
    async fn ready(&mut self, pollable: Resource<DynPollable>) -> Result<bool> {
        let table = self.table();
        let pollable = table.get(&pollable)?;
        let ready = (pollable.make_future)(table.get_any_mut(pollable.index)?);
        futures::pin_mut!(ready);
        Ok(matches!(
            futures::future::poll_immediate(ready).await,
            Some(())
        ))
    }
    fn drop(&mut self, pollable: Resource<DynPollable>) -> Result<()> {
        let pollable = self.table().delete(pollable)?;
        if let Some(delete) = pollable.remove_index_on_delete {
            delete(self.table(), pollable.index)?;
        }
        Ok(())
    }
}

impl<T: IoView> error::Host for IoImpl<T> {}

impl<T: IoView> streams::Host for IoImpl<T> {
    fn convert_stream_error(&mut self, err: StreamError) -> Result<streams::StreamError> {
        match err {
            StreamError::Closed => Ok(streams::StreamError::Closed),
            StreamError::LastOperationFailed(e) => Ok(streams::StreamError::LastOperationFailed(
                self.table().push(e)?,
            )),
            StreamError::Trap(e) => Err(e),
        }
    }
}

impl<T: IoView> error::HostError for IoImpl<T> {
    fn drop(&mut self, err: Resource<streams::Error>) -> Result<()> {
        self.table().delete(err)?;
        Ok(())
    }

    fn to_debug_string(&mut self, err: Resource<streams::Error>) -> Result<String> {
        Ok(alloc::format!("{:?}", self.table().get(&err)?))
    }
}

impl<T: IoView> streams::HostOutputStream for IoImpl<T> {
    async fn drop(&mut self, stream: Resource<DynOutputStream>) -> Result<()> {
        self.table().delete(stream)?.cancel().await;
        Ok(())
    }

    fn check_write(&mut self, stream: Resource<DynOutputStream>) -> StreamResult<u64> {
        let bytes = self.table().get_mut(&stream)?.check_write()?;
        Ok(bytes as u64)
    }

    fn write(&mut self, stream: Resource<DynOutputStream>, bytes: Vec<u8>) -> StreamResult<()> {
        self.table().get_mut(&stream)?.write(bytes.into())?;
        Ok(())
    }

    fn subscribe(&mut self, stream: Resource<DynOutputStream>) -> Result<Resource<DynPollable>> {
        subscribe(self.table(), stream)
    }

    async fn blocking_write_and_flush(
        &mut self,
        stream: Resource<DynOutputStream>,
        bytes: Vec<u8>,
    ) -> StreamResult<()> {
        if bytes.len() > 4096 {
            return Err(StreamError::trap(
                "Buffer too large for blocking-write-and-flush (expected at most 4096)",
            ));
        }

        self.table()
            .get_mut(&stream)?
            .blocking_write_and_flush(bytes.into())
            .await
    }

    async fn blocking_write_zeroes_and_flush(
        &mut self,
        stream: Resource<DynOutputStream>,
        len: u64,
    ) -> StreamResult<()> {
        if len > 4096 {
            return Err(StreamError::trap(
                "Buffer too large for blocking-write-zeroes-and-flush (expected at most 4096)",
            ));
        }

        self.table()
            .get_mut(&stream)?
            .blocking_write_zeroes_and_flush(len as usize)
            .await
    }

    fn write_zeroes(&mut self, stream: Resource<DynOutputStream>, len: u64) -> StreamResult<()> {
        self.table().get_mut(&stream)?.write_zeroes(len as usize)?;
        Ok(())
    }

    fn flush(&mut self, stream: Resource<DynOutputStream>) -> StreamResult<()> {
        self.table().get_mut(&stream)?.flush()?;
        Ok(())
    }

    async fn blocking_flush(&mut self, stream: Resource<DynOutputStream>) -> StreamResult<()> {
        let s = self.table().get_mut(&stream)?;
        s.flush()?;
        s.write_ready().await?;
        Ok(())
    }

    fn splice(
        &mut self,
        dest: Resource<DynOutputStream>,
        src: Resource<DynInputStream>,
        len: u64,
    ) -> StreamResult<u64> {
        let len = len.try_into().unwrap_or(usize::MAX);

        let permit = {
            let output = self.table().get_mut(&dest)?;
            output.check_write()?
        };
        let len = len.min(permit);
        if len == 0 {
            return Ok(0);
        }

        let contents = self.table().get_mut(&src)?.read(len)?;

        let len = contents.len();
        if len == 0 {
            return Ok(0);
        }

        let output = self.table().get_mut(&dest)?;
        output.write(contents)?;
        Ok(len.try_into().expect("usize can fit in u64"))
    }

    async fn blocking_splice(
        &mut self,
        dest: Resource<DynOutputStream>,
        src: Resource<DynInputStream>,
        len: u64,
    ) -> StreamResult<u64> {
        let len = len.try_into().unwrap_or(usize::MAX);

        let permit = {
            let output = self.table().get_mut(&dest)?;
            output.write_ready().await?
        };
        let len = len.min(permit);
        if len == 0 {
            return Ok(0);
        }

        let contents = self.table().get_mut(&src)?.blocking_read(len).await?;

        let len = contents.len();
        if len == 0 {
            return Ok(0);
        }

        let output = self.table().get_mut(&dest)?;
        output.blocking_write_and_flush(contents).await?;
        Ok(len.try_into().expect("usize can fit in u64"))
    }
}

impl<T: IoView> streams::HostInputStream for IoImpl<T> {
    async fn drop(&mut self, stream: Resource<DynInputStream>) -> Result<()> {
        self.table().delete(stream)?.cancel().await;
        Ok(())
    }

    fn read(&mut self, stream: Resource<DynInputStream>, len: u64) -> StreamResult<Vec<u8>> {
        let len = len.try_into().unwrap_or(usize::MAX);
        let bytes = self.table().get_mut(&stream)?.read(len)?;
        debug_assert!(bytes.len() <= len);
        Ok(bytes.into())
    }

    async fn blocking_read(
        &mut self,
        stream: Resource<DynInputStream>,
        len: u64,
    ) -> StreamResult<Vec<u8>> {
        let len = len.try_into().unwrap_or(usize::MAX);
        let bytes = self.table().get_mut(&stream)?.blocking_read(len).await?;
        debug_assert!(bytes.len() <= len);
        Ok(bytes.into())
    }

    fn skip(&mut self, stream: Resource<DynInputStream>, len: u64) -> StreamResult<u64> {
        let len = len.try_into().unwrap_or(usize::MAX);
        let written = self.table().get_mut(&stream)?.skip(len)?;
        Ok(written.try_into().expect("usize always fits in u64"))
    }

    async fn blocking_skip(
        &mut self,
        stream: Resource<DynInputStream>,
        len: u64,
    ) -> StreamResult<u64> {
        let len = len.try_into().unwrap_or(usize::MAX);
        let written = self.table().get_mut(&stream)?.blocking_skip(len).await?;
        Ok(written.try_into().expect("usize always fits in u64"))
    }

    fn subscribe(&mut self, stream: Resource<DynInputStream>) -> Result<Resource<DynPollable>> {
        crate::poll::subscribe(self.table(), stream)
    }
}
