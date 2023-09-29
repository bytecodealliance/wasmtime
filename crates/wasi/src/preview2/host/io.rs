use crate::preview2::{
    bindings::io::poll::Pollable,
    bindings::io::streams::{self, InputStream, OutputStream},
    filesystem::FileInputStream,
    poll::PollableFuture,
    stream::{
        HostInputStream, HostOutputStream, InternalInputStream, InternalTableStreamExt,
        StreamError, TableStreamExt,
    },
    HostPollable, TableError, TablePollableExt, WasiView,
};
use std::any::Any;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use wasmtime::component::Resource;

impl From<TableError> for streams::Error {
    fn from(e: TableError) -> streams::Error {
        streams::Error::trap(e.into())
    }
}
impl From<StreamError> for streams::Error {
    fn from(e: StreamError) -> streams::Error {
        match e {
            StreamError::Closed => streams::StreamError::Closed.into(),
            StreamError::LastOperationFailed(e) => {
                tracing::debug!("streams::StreamError::LastOperationFailed: {e:?}");
                streams::StreamError::LastOperationFailed.into()
            }
            StreamError::Trap(e) => streams::Error::trap(e),
        }
    }
}

#[async_trait::async_trait]
impl<T: WasiView> streams::Host for T {}

#[async_trait::async_trait]
impl<T: WasiView> streams::HostOutputStream for T {
    fn drop(&mut self, stream: Resource<OutputStream>) -> anyhow::Result<()> {
        self.table_mut().delete_output_stream(stream)?;
        Ok(())
    }

    fn check_write(&mut self, stream: Resource<OutputStream>) -> Result<u64, streams::Error> {
        let s = self.table_mut().get_output_stream_mut(&stream)?;
        let mut ready = s.write_ready();
        let mut task = Context::from_waker(futures::task::noop_waker_ref());
        match Pin::new(&mut ready).poll(&mut task) {
            Poll::Ready(Ok(permit)) => Ok(permit as u64),
            Poll::Ready(Err(e)) => Err(e.into()),
            Poll::Pending => Ok(0),
        }
    }

    fn write(
        &mut self,
        stream: Resource<OutputStream>,
        bytes: Vec<u8>,
    ) -> Result<(), streams::Error> {
        let s = self.table_mut().get_output_stream_mut(&stream)?;
        HostOutputStream::write(s, bytes.into())?;
        Ok(())
    }

    fn subscribe(&mut self, stream: Resource<OutputStream>) -> anyhow::Result<Resource<Pollable>> {
        // Ensure that table element is an output-stream:
        let _ = self.table_mut().get_output_stream_mut(&stream)?;

        fn output_stream_ready<'a>(stream: &'a mut dyn Any) -> PollableFuture<'a> {
            let stream = stream
                .downcast_mut::<Box<dyn HostOutputStream>>()
                .expect("downcast to HostOutputStream failed");
            Box::pin(async move {
                let _ = stream.write_ready().await?;
                Ok(())
            })
        }

        Ok(self
            .table_mut()
            .push_host_pollable(HostPollable::TableEntry {
                index: stream.rep(),
                make_future: output_stream_ready,
            })?)
    }

    async fn blocking_write_and_flush(
        &mut self,
        stream: Resource<OutputStream>,
        bytes: Vec<u8>,
    ) -> Result<(), streams::Error> {
        let s = self.table_mut().get_output_stream_mut(&stream)?;

        if bytes.len() > 4096 {
            return Err(streams::Error::trap(anyhow::anyhow!(
                "Buffer too large for blocking-write-and-flush (expected at most 4096)"
            )));
        }

        let mut bytes = bytes::Bytes::from(bytes);
        while !bytes.is_empty() {
            let permit = s.write_ready().await?;
            let len = bytes.len().min(permit);
            let chunk = bytes.split_to(len);
            HostOutputStream::write(s, chunk)?;
        }

        HostOutputStream::flush(s)?;
        let _ = s.write_ready().await?;

        Ok(())
    }

    async fn blocking_write_zeroes_and_flush(
        &mut self,
        stream: Resource<OutputStream>,
        len: u64,
    ) -> Result<(), streams::Error> {
        let s = self.table_mut().get_output_stream_mut(&stream)?;

        if len > 4096 {
            return Err(streams::Error::trap(anyhow::anyhow!(
                "Buffer too large for blocking-write-zeroes-and-flush (expected at most 4096)"
            )));
        }

        let mut len = len;
        while len > 0 {
            let permit = s.write_ready().await?;
            let this_len = len.min(permit as u64);
            HostOutputStream::write_zeroes(s, this_len as usize)?;
            len -= this_len;
        }

        HostOutputStream::flush(s)?;
        let _ = s.write_ready().await?;

        Ok(())
    }

    fn write_zeroes(
        &mut self,
        stream: Resource<OutputStream>,
        len: u64,
    ) -> Result<(), streams::Error> {
        let s = self.table_mut().get_output_stream_mut(&stream)?;
        HostOutputStream::write_zeroes(s, len as usize)?;
        Ok(())
    }

    fn flush(&mut self, stream: Resource<OutputStream>) -> Result<(), streams::Error> {
        let s = self.table_mut().get_output_stream_mut(&stream)?;
        HostOutputStream::flush(s)?;
        Ok(())
    }

    async fn blocking_flush(
        &mut self,
        stream: Resource<OutputStream>,
    ) -> Result<(), streams::Error> {
        let s = self.table_mut().get_output_stream_mut(&stream)?;
        HostOutputStream::flush(s)?;
        let _ = s.write_ready().await?;
        Ok(())
    }

    async fn splice(
        &mut self,
        _dst: Resource<OutputStream>,
        _src: Resource<InputStream>,
        _len: u64,
    ) -> Result<u64, streams::Error> {
        // TODO: We can't get two streams at the same time because they both
        // carry the exclusive lifetime of `ctx`. When [`get_many_mut`] is
        // stabilized, that could allow us to add a `get_many_stream_mut` or
        // so which lets us do this.
        //
        // [`get_many_mut`]: https://doc.rust-lang.org/stable/std/collections/hash_map/struct.HashMap.html#method.get_many_mut
        /*
        let s: &mut Box<dyn crate::InputStream> = ctx
            .table_mut()
            .get_input_stream_mut(src)
            ?;
        let d: &mut Box<dyn crate::OutputStream> = ctx
            .table_mut()
            .get_output_stream_mut(dst)
            ?;

        let bytes_spliced: u64 = s.splice(&mut **d, len).await?;

        Ok(bytes_spliced)
        */
        todo!("stream splice is not implemented")
    }

    async fn blocking_splice(
        &mut self,
        _dst: Resource<OutputStream>,
        _src: Resource<InputStream>,
        _len: u64,
    ) -> Result<u64, streams::Error> {
        // TODO: once splice is implemented, figure out what the blocking semantics are for waiting
        // on src and dest here.
        todo!("stream splice is not implemented")
    }

    async fn forward(
        &mut self,
        _dst: Resource<OutputStream>,
        _src: Resource<InputStream>,
    ) -> Result<u64, streams::Error> {
        // TODO: We can't get two streams at the same time because they both
        // carry the exclusive lifetime of `ctx`. When [`get_many_mut`] is
        // stabilized, that could allow us to add a `get_many_stream_mut` or
        // so which lets us do this.
        //
        // [`get_many_mut`]: https://doc.rust-lang.org/stable/std/collections/hash_map/struct.HashMap.html#method.get_many_mut
        /*
        let s: &mut Box<dyn crate::InputStream> = ctx
            .table_mut()
            .get_input_stream_mut(src)
            ?;
        let d: &mut Box<dyn crate::OutputStream> = ctx
            .table_mut()
            .get_output_stream_mut(dst)
            ?;

        let bytes_spliced: u64 = s.splice(&mut **d, len).await?;

        Ok(bytes_spliced)
        */

        todo!("stream forward is not implemented")
    }
}

impl From<std::num::TryFromIntError> for streams::Error {
    fn from(e: std::num::TryFromIntError) -> Self {
        streams::Error::trap(anyhow::anyhow!("length overflow: {e:?}"))
    }
}
#[async_trait::async_trait]
impl<T: WasiView> streams::HostInputStream for T {
    fn drop(&mut self, stream: Resource<InputStream>) -> anyhow::Result<()> {
        self.table_mut().delete_internal_input_stream(stream)?;
        Ok(())
    }

    async fn read(
        &mut self,
        stream: Resource<InputStream>,
        len: u64,
    ) -> Result<Vec<u8>, streams::Error> {
        let len = len.try_into()?;
        let bytes = match self.table_mut().get_internal_input_stream_mut(&stream)? {
            InternalInputStream::Host(s) => HostInputStream::read(s.as_mut(), len)?,
            InternalInputStream::File(s) => FileInputStream::read(s, len).await?,
        };
        debug_assert!(bytes.len() <= len as usize);
        Ok(bytes.into())
    }

    async fn blocking_read(
        &mut self,
        stream: Resource<InputStream>,
        len: u64,
    ) -> Result<Vec<u8>, streams::Error> {
        let len = len.try_into()?;
        let bytes = match self.table_mut().get_internal_input_stream_mut(&stream)? {
            InternalInputStream::Host(s) => {
                s.ready().await.map_err(streams::Error::trap)?;
                HostInputStream::read(s.as_mut(), len)?
            }
            InternalInputStream::File(s) => FileInputStream::read(s, len).await?,
        };
        debug_assert!(bytes.len() <= len as usize);
        Ok(bytes.into())
    }

    async fn skip(
        &mut self,
        stream: Resource<InputStream>,
        len: u64,
    ) -> Result<u64, streams::Error> {
        let len = len.try_into()?;
        let written = match self.table_mut().get_internal_input_stream_mut(&stream)? {
            InternalInputStream::Host(s) => HostInputStream::skip(s.as_mut(), len)?,
            InternalInputStream::File(s) => FileInputStream::skip(s, len).await?,
        };
        Ok(written.try_into().expect("usize always fits in u64"))
    }

    async fn blocking_skip(
        &mut self,
        stream: Resource<InputStream>,
        len: u64,
    ) -> Result<u64, streams::Error> {
        let len = len.try_into()?;
        let written = match self.table_mut().get_internal_input_stream_mut(&stream)? {
            InternalInputStream::Host(s) => {
                s.ready().await.map_err(streams::Error::trap)?;
                HostInputStream::skip(s.as_mut(), len)?
            }
            InternalInputStream::File(s) => FileInputStream::skip(s, len).await?,
        };
        Ok(written.try_into().expect("usize always fits in u64"))
    }

    fn subscribe(&mut self, stream: Resource<InputStream>) -> anyhow::Result<Resource<Pollable>> {
        // Ensure that table element is an input-stream:
        let pollable = match self.table_mut().get_internal_input_stream_mut(&stream)? {
            InternalInputStream::Host(_) => {
                fn input_stream_ready<'a>(stream: &'a mut dyn Any) -> PollableFuture<'a> {
                    let stream = stream
                        .downcast_mut::<InternalInputStream>()
                        .expect("downcast to InternalInputStream failed");
                    match *stream {
                        InternalInputStream::Host(ref mut hs) => hs.ready(),
                        _ => unreachable!(),
                    }
                }

                HostPollable::TableEntry {
                    index: stream.rep(),
                    make_future: input_stream_ready,
                }
            }
            // Files are always "ready" immediately (because we have no way to actually wait on
            // readiness in epoll)
            InternalInputStream::File(_) => {
                HostPollable::Closure(Box::new(|| Box::pin(futures::future::ready(Ok(())))))
            }
        };
        Ok(self.table_mut().push_host_pollable(pollable)?)
    }
}

pub mod sync {
    use crate::preview2::{
        bindings::io::streams::{
            self as async_streams, HostInputStream as AsyncHostInputStream,
            HostOutputStream as AsyncHostOutputStream,
        },
        bindings::sync_io::io::poll::Pollable,
        bindings::sync_io::io::streams::{self, InputStream, OutputStream},
        in_tokio, WasiView,
    };
    use wasmtime::component::Resource;

    impl From<async_streams::StreamError> for streams::StreamError {
        fn from(other: async_streams::StreamError) -> Self {
            match other {
                async_streams::StreamError::LastOperationFailed => Self::LastOperationFailed,
                async_streams::StreamError::Closed => Self::Closed,
            }
        }
    }
    impl From<async_streams::Error> for streams::Error {
        fn from(other: async_streams::Error) -> Self {
            match other.downcast() {
                Ok(write_error) => streams::Error::from(streams::StreamError::from(write_error)),
                Err(e) => streams::Error::trap(e),
            }
        }
    }

    impl<T: WasiView> streams::Host for T {}

    impl<T: WasiView> streams::HostOutputStream for T {
        fn drop(&mut self, stream: Resource<OutputStream>) -> anyhow::Result<()> {
            AsyncHostOutputStream::drop(self, stream)
        }

        fn check_write(&mut self, stream: Resource<OutputStream>) -> Result<u64, streams::Error> {
            Ok(AsyncHostOutputStream::check_write(self, stream)?)
        }

        fn write(
            &mut self,
            stream: Resource<OutputStream>,
            bytes: Vec<u8>,
        ) -> Result<(), streams::Error> {
            Ok(AsyncHostOutputStream::write(self, stream, bytes)?)
        }

        fn blocking_write_and_flush(
            &mut self,
            stream: Resource<OutputStream>,
            bytes: Vec<u8>,
        ) -> Result<(), streams::Error> {
            Ok(in_tokio(async {
                AsyncHostOutputStream::blocking_write_and_flush(self, stream, bytes).await
            })?)
        }

        fn blocking_write_zeroes_and_flush(
            &mut self,
            stream: Resource<OutputStream>,
            len: u64,
        ) -> Result<(), streams::Error> {
            Ok(in_tokio(async {
                AsyncHostOutputStream::blocking_write_zeroes_and_flush(self, stream, len).await
            })?)
        }

        fn subscribe(
            &mut self,
            stream: Resource<OutputStream>,
        ) -> anyhow::Result<Resource<Pollable>> {
            Ok(AsyncHostOutputStream::subscribe(self, stream)?)
        }

        fn write_zeroes(
            &mut self,
            stream: Resource<OutputStream>,
            len: u64,
        ) -> Result<(), streams::Error> {
            Ok(AsyncHostOutputStream::write_zeroes(self, stream, len)?)
        }

        fn flush(&mut self, stream: Resource<OutputStream>) -> Result<(), streams::Error> {
            Ok(AsyncHostOutputStream::flush(
                self,
                Resource::new_borrow(stream.rep()),
            )?)
        }

        fn blocking_flush(&mut self, stream: Resource<OutputStream>) -> Result<(), streams::Error> {
            Ok(in_tokio(async {
                AsyncHostOutputStream::blocking_flush(self, Resource::new_borrow(stream.rep()))
                    .await
            })?)
        }

        fn splice(
            &mut self,
            dst: Resource<OutputStream>,
            src: Resource<InputStream>,
            len: u64,
        ) -> Result<u64, streams::Error> {
            Ok(in_tokio(async {
                AsyncHostOutputStream::splice(self, dst, src, len).await
            })?)
        }

        fn blocking_splice(
            &mut self,
            dst: Resource<OutputStream>,
            src: Resource<InputStream>,
            len: u64,
        ) -> Result<u64, streams::Error> {
            Ok(in_tokio(async {
                AsyncHostOutputStream::blocking_splice(self, dst, src, len).await
            })?)
        }

        fn forward(
            &mut self,
            dst: Resource<OutputStream>,
            src: Resource<InputStream>,
        ) -> Result<u64, streams::Error> {
            Ok(in_tokio(async {
                AsyncHostOutputStream::forward(self, dst, src).await
            })?)
        }
    }

    impl<T: WasiView> streams::HostInputStream for T {
        fn drop(&mut self, stream: Resource<InputStream>) -> anyhow::Result<()> {
            AsyncHostInputStream::drop(self, stream)
        }

        fn read(
            &mut self,
            stream: Resource<InputStream>,
            len: u64,
        ) -> Result<Vec<u8>, streams::Error> {
            Ok(in_tokio(async {
                AsyncHostInputStream::read(self, stream, len).await
            })?)
        }

        fn blocking_read(
            &mut self,
            stream: Resource<InputStream>,
            len: u64,
        ) -> Result<Vec<u8>, streams::Error> {
            Ok(in_tokio(async {
                AsyncHostInputStream::blocking_read(self, stream, len).await
            })?)
        }

        fn skip(&mut self, stream: Resource<InputStream>, len: u64) -> Result<u64, streams::Error> {
            Ok(in_tokio(async {
                AsyncHostInputStream::skip(self, stream, len).await
            })?)
        }

        fn blocking_skip(
            &mut self,
            stream: Resource<InputStream>,
            len: u64,
        ) -> Result<u64, streams::Error> {
            Ok(in_tokio(async {
                AsyncHostInputStream::blocking_skip(self, stream, len).await
            })?)
        }

        fn subscribe(
            &mut self,
            stream: Resource<InputStream>,
        ) -> anyhow::Result<Resource<Pollable>> {
            AsyncHostInputStream::subscribe(self, stream)
        }
    }
}
