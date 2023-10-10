use crate::preview2::{
    bindings::io::streams::{self, InputStream, OutputStream},
    poll::subscribe,
    Pollable, StreamError, StreamResult, WasiView,
};
use wasmtime::component::Resource;

impl<T: WasiView> streams::Host for T {
    fn convert_stream_error(&mut self, err: StreamError) -> anyhow::Result<streams::StreamError> {
        match err {
            StreamError::Closed => Ok(streams::StreamError::Closed),
            StreamError::LastOperationFailed(e) => Ok(streams::StreamError::LastOperationFailed(
                self.table_mut().push_resource(e)?,
            )),
            StreamError::Trap(e) => Err(e),
        }
    }
}

impl<T: WasiView> streams::HostError for T {
    fn drop(&mut self, err: Resource<streams::Error>) -> anyhow::Result<()> {
        self.table_mut().delete_resource(err)?;
        Ok(())
    }

    fn to_debug_string(&mut self, err: Resource<streams::Error>) -> anyhow::Result<String> {
        Ok(format!("{:?}", self.table_mut().get_resource(&err)?))
    }
}

#[async_trait::async_trait]
impl<T: WasiView> streams::HostOutputStream for T {
    fn drop(&mut self, stream: Resource<OutputStream>) -> anyhow::Result<()> {
        self.table_mut().delete_resource(stream)?;
        Ok(())
    }

    fn check_write(&mut self, stream: Resource<OutputStream>) -> StreamResult<u64> {
        let bytes = self.table_mut().get_resource_mut(&stream)?.check_write()?;
        Ok(bytes as u64)
    }

    fn write(&mut self, stream: Resource<OutputStream>, bytes: Vec<u8>) -> StreamResult<()> {
        self.table_mut()
            .get_resource_mut(&stream)?
            .write(bytes.into())?;
        Ok(())
    }

    fn subscribe(&mut self, stream: Resource<OutputStream>) -> anyhow::Result<Resource<Pollable>> {
        subscribe(self.table_mut(), stream)
    }

    async fn blocking_write_and_flush(
        &mut self,
        stream: Resource<OutputStream>,
        bytes: Vec<u8>,
    ) -> StreamResult<()> {
        let s = self.table_mut().get_resource_mut(&stream)?;

        if bytes.len() > 4096 {
            return Err(StreamError::trap(
                "Buffer too large for blocking-write-and-flush (expected at most 4096)",
            ));
        }

        let mut bytes = bytes::Bytes::from(bytes);
        while !bytes.is_empty() {
            let permit = s.write_ready().await?;
            let len = bytes.len().min(permit);
            let chunk = bytes.split_to(len);
            s.write(chunk)?;
        }

        s.flush()?;
        s.write_ready().await?;

        Ok(())
    }

    async fn blocking_write_zeroes_and_flush(
        &mut self,
        stream: Resource<OutputStream>,
        len: u64,
    ) -> StreamResult<()> {
        let s = self.table_mut().get_resource_mut(&stream)?;

        if len > 4096 {
            return Err(StreamError::trap(
                "Buffer too large for blocking-write-zeroes-and-flush (expected at most 4096)",
            ));
        }

        let mut len = len;
        while len > 0 {
            let permit = s.write_ready().await?;
            let this_len = len.min(permit as u64);
            s.write_zeroes(this_len as usize)?;
            len -= this_len;
        }

        s.flush()?;
        s.write_ready().await?;

        Ok(())
    }

    fn write_zeroes(&mut self, stream: Resource<OutputStream>, len: u64) -> StreamResult<()> {
        self.table_mut()
            .get_resource_mut(&stream)?
            .write_zeroes(len as usize)?;
        Ok(())
    }

    fn flush(&mut self, stream: Resource<OutputStream>) -> StreamResult<()> {
        self.table_mut().get_resource_mut(&stream)?.flush()?;
        Ok(())
    }

    async fn blocking_flush(&mut self, stream: Resource<OutputStream>) -> StreamResult<()> {
        let s = self.table_mut().get_resource_mut(&stream)?;
        s.flush()?;
        s.write_ready().await?;
        Ok(())
    }

    async fn splice(
        &mut self,
        _dst: Resource<OutputStream>,
        _src: Resource<InputStream>,
        _len: u64,
    ) -> StreamResult<u64> {
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
            .get_resource_mut(dst)
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
    ) -> StreamResult<u64> {
        // TODO: once splice is implemented, figure out what the blocking semantics are for waiting
        // on src and dest here.
        todo!("stream splice is not implemented")
    }

    async fn forward(
        &mut self,
        _dst: Resource<OutputStream>,
        _src: Resource<InputStream>,
    ) -> StreamResult<u64> {
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
            .get_resource_mut(dst)
            ?;

        let bytes_spliced: u64 = s.splice(&mut **d, len).await?;

        Ok(bytes_spliced)
        */

        todo!("stream forward is not implemented")
    }
}

#[async_trait::async_trait]
impl<T: WasiView> streams::HostInputStream for T {
    fn drop(&mut self, stream: Resource<InputStream>) -> anyhow::Result<()> {
        self.table_mut().delete_resource(stream)?;
        Ok(())
    }

    async fn read(&mut self, stream: Resource<InputStream>, len: u64) -> StreamResult<Vec<u8>> {
        let len = len.try_into().unwrap_or(usize::MAX);
        let bytes = match self.table_mut().get_resource_mut(&stream)? {
            InputStream::Host(s) => s.read(len)?,
            InputStream::File(s) => s.read(len).await?,
        };
        debug_assert!(bytes.len() <= len as usize);
        Ok(bytes.into())
    }

    async fn blocking_read(
        &mut self,
        stream: Resource<InputStream>,
        len: u64,
    ) -> StreamResult<Vec<u8>> {
        if let InputStream::Host(s) = self.table_mut().get_resource_mut(&stream)? {
            s.ready().await;
        }
        self.read(stream, len).await
    }

    async fn skip(&mut self, stream: Resource<InputStream>, len: u64) -> StreamResult<u64> {
        let len = len.try_into().unwrap_or(usize::MAX);
        let written = match self.table_mut().get_resource_mut(&stream)? {
            InputStream::Host(s) => s.skip(len)?,
            InputStream::File(s) => s.skip(len).await?,
        };
        Ok(written.try_into().expect("usize always fits in u64"))
    }

    async fn blocking_skip(
        &mut self,
        stream: Resource<InputStream>,
        len: u64,
    ) -> StreamResult<u64> {
        if let InputStream::Host(s) = self.table_mut().get_resource_mut(&stream)? {
            s.ready().await;
        }
        self.skip(stream, len).await
    }

    fn subscribe(&mut self, stream: Resource<InputStream>) -> anyhow::Result<Resource<Pollable>> {
        crate::preview2::poll::subscribe(self.table_mut(), stream)
    }
}

pub mod sync {
    use crate::preview2::{
        bindings::io::streams::{
            self as async_streams, Host as AsyncHost, HostError as AsyncHostError,
            HostInputStream as AsyncHostInputStream, HostOutputStream as AsyncHostOutputStream,
        },
        bindings::sync_io::io::poll::Pollable,
        bindings::sync_io::io::streams::{self, InputStream, OutputStream},
        in_tokio, StreamError, StreamResult, WasiView,
    };
    use wasmtime::component::Resource;

    impl From<async_streams::StreamError> for streams::StreamError {
        fn from(other: async_streams::StreamError) -> Self {
            match other {
                async_streams::StreamError::LastOperationFailed(e) => Self::LastOperationFailed(e),
                async_streams::StreamError::Closed => Self::Closed,
            }
        }
    }

    impl<T: WasiView> streams::Host for T {
        fn convert_stream_error(
            &mut self,
            err: StreamError,
        ) -> anyhow::Result<streams::StreamError> {
            Ok(AsyncHost::convert_stream_error(self, err)?.into())
        }
    }

    impl<T: WasiView> streams::HostError for T {
        fn drop(&mut self, err: Resource<streams::Error>) -> anyhow::Result<()> {
            AsyncHostError::drop(self, err)
        }

        fn to_debug_string(&mut self, err: Resource<streams::Error>) -> anyhow::Result<String> {
            AsyncHostError::to_debug_string(self, err)
        }
    }

    impl<T: WasiView> streams::HostOutputStream for T {
        fn drop(&mut self, stream: Resource<OutputStream>) -> anyhow::Result<()> {
            AsyncHostOutputStream::drop(self, stream)
        }

        fn check_write(&mut self, stream: Resource<OutputStream>) -> StreamResult<u64> {
            Ok(AsyncHostOutputStream::check_write(self, stream)?)
        }

        fn write(&mut self, stream: Resource<OutputStream>, bytes: Vec<u8>) -> StreamResult<()> {
            Ok(AsyncHostOutputStream::write(self, stream, bytes)?)
        }

        fn blocking_write_and_flush(
            &mut self,
            stream: Resource<OutputStream>,
            bytes: Vec<u8>,
        ) -> StreamResult<()> {
            in_tokio(async {
                AsyncHostOutputStream::blocking_write_and_flush(self, stream, bytes).await
            })
        }

        fn blocking_write_zeroes_and_flush(
            &mut self,
            stream: Resource<OutputStream>,
            len: u64,
        ) -> StreamResult<()> {
            in_tokio(async {
                AsyncHostOutputStream::blocking_write_zeroes_and_flush(self, stream, len).await
            })
        }

        fn subscribe(
            &mut self,
            stream: Resource<OutputStream>,
        ) -> anyhow::Result<Resource<Pollable>> {
            Ok(AsyncHostOutputStream::subscribe(self, stream)?)
        }

        fn write_zeroes(&mut self, stream: Resource<OutputStream>, len: u64) -> StreamResult<()> {
            Ok(AsyncHostOutputStream::write_zeroes(self, stream, len)?)
        }

        fn flush(&mut self, stream: Resource<OutputStream>) -> StreamResult<()> {
            Ok(AsyncHostOutputStream::flush(
                self,
                Resource::new_borrow(stream.rep()),
            )?)
        }

        fn blocking_flush(&mut self, stream: Resource<OutputStream>) -> StreamResult<()> {
            in_tokio(async {
                AsyncHostOutputStream::blocking_flush(self, Resource::new_borrow(stream.rep()))
                    .await
            })
        }

        fn splice(
            &mut self,
            dst: Resource<OutputStream>,
            src: Resource<InputStream>,
            len: u64,
        ) -> StreamResult<u64> {
            in_tokio(async { AsyncHostOutputStream::splice(self, dst, src, len).await })
        }

        fn blocking_splice(
            &mut self,
            dst: Resource<OutputStream>,
            src: Resource<InputStream>,
            len: u64,
        ) -> StreamResult<u64> {
            in_tokio(async { AsyncHostOutputStream::blocking_splice(self, dst, src, len).await })
        }

        fn forward(
            &mut self,
            dst: Resource<OutputStream>,
            src: Resource<InputStream>,
        ) -> StreamResult<u64> {
            in_tokio(async { AsyncHostOutputStream::forward(self, dst, src).await })
        }
    }

    impl<T: WasiView> streams::HostInputStream for T {
        fn drop(&mut self, stream: Resource<InputStream>) -> anyhow::Result<()> {
            AsyncHostInputStream::drop(self, stream)
        }

        fn read(&mut self, stream: Resource<InputStream>, len: u64) -> StreamResult<Vec<u8>> {
            in_tokio(async { AsyncHostInputStream::read(self, stream, len).await })
        }

        fn blocking_read(
            &mut self,
            stream: Resource<InputStream>,
            len: u64,
        ) -> StreamResult<Vec<u8>> {
            in_tokio(async { AsyncHostInputStream::blocking_read(self, stream, len).await })
        }

        fn skip(&mut self, stream: Resource<InputStream>, len: u64) -> StreamResult<u64> {
            in_tokio(async { AsyncHostInputStream::skip(self, stream, len).await })
        }

        fn blocking_skip(&mut self, stream: Resource<InputStream>, len: u64) -> StreamResult<u64> {
            in_tokio(async { AsyncHostInputStream::blocking_skip(self, stream, len).await })
        }

        fn subscribe(
            &mut self,
            stream: Resource<InputStream>,
        ) -> anyhow::Result<Resource<Pollable>> {
            AsyncHostInputStream::subscribe(self, stream)
        }
    }
}
