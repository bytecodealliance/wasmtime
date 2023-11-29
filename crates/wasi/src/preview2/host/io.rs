use crate::preview2::{
    bindings::io::error,
    bindings::io::streams::{self, InputStream, OutputStream},
    poll::subscribe,
    Pollable, StreamError, StreamResult, WasiView,
};
use wasmtime::component::{Resource, ResourceTable};

impl<T: WasiView> error::Host for T {}

impl<T: WasiView> streams::Host for T {
    fn convert_stream_error(
        &mut self,
        table: &mut ResourceTable,
        err: StreamError,
    ) -> anyhow::Result<streams::StreamError> {
        match err {
            StreamError::Closed => Ok(streams::StreamError::Closed),
            StreamError::LastOperationFailed(e) => {
                Ok(streams::StreamError::LastOperationFailed(table.push(e)?))
            }
            StreamError::Trap(e) => Err(e),
        }
    }
}

impl<T: WasiView> error::HostError for T {
    fn drop(
        &mut self,
        table: &mut ResourceTable,
        err: Resource<streams::Error>,
    ) -> anyhow::Result<()> {
        table.delete(err)?;
        Ok(())
    }

    fn to_debug_string(
        &mut self,
        table: &mut ResourceTable,
        err: Resource<streams::Error>,
    ) -> anyhow::Result<String> {
        Ok(format!("{:?}", table.get(&err)?))
    }
}

#[async_trait::async_trait]
impl<T: WasiView> streams::HostOutputStream for T {
    fn drop(
        &mut self,
        table: &mut ResourceTable,
        stream: Resource<OutputStream>,
    ) -> anyhow::Result<()> {
        table.delete(stream)?;
        Ok(())
    }

    fn check_write(
        &mut self,
        table: &mut ResourceTable,
        stream: Resource<OutputStream>,
    ) -> StreamResult<u64> {
        let bytes = table.get_mut(&stream)?.check_write()?;
        Ok(bytes as u64)
    }

    fn write(
        &mut self,
        table: &mut ResourceTable,
        stream: Resource<OutputStream>,
        bytes: Vec<u8>,
    ) -> StreamResult<()> {
        table.get_mut(&stream)?.write(bytes.into())?;
        Ok(())
    }

    fn subscribe(
        &mut self,
        table: &mut ResourceTable,
        stream: Resource<OutputStream>,
    ) -> anyhow::Result<Resource<Pollable>> {
        subscribe(table, stream)
    }

    async fn blocking_write_and_flush(
        &mut self,
        table: &mut ResourceTable,
        stream: Resource<OutputStream>,
        bytes: Vec<u8>,
    ) -> StreamResult<()> {
        let s = table.get_mut(&stream)?;

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
        table: &mut ResourceTable,
        stream: Resource<OutputStream>,
        len: u64,
    ) -> StreamResult<()> {
        let s = table.get_mut(&stream)?;

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

    fn write_zeroes(
        &mut self,
        table: &mut ResourceTable,
        stream: Resource<OutputStream>,
        len: u64,
    ) -> StreamResult<()> {
        table.get_mut(&stream)?.write_zeroes(len as usize)?;
        Ok(())
    }

    fn flush(
        &mut self,
        table: &mut ResourceTable,
        stream: Resource<OutputStream>,
    ) -> StreamResult<()> {
        table.get_mut(&stream)?.flush()?;
        Ok(())
    }

    async fn blocking_flush(
        &mut self,
        table: &mut ResourceTable,
        stream: Resource<OutputStream>,
    ) -> StreamResult<()> {
        let s = table.get_mut(&stream)?;
        s.flush()?;
        s.write_ready().await?;
        Ok(())
    }

    async fn splice(
        &mut self,
        table: &mut ResourceTable,
        dest: Resource<OutputStream>,
        src: Resource<InputStream>,
        len: u64,
    ) -> StreamResult<u64> {
        let len = len.try_into().unwrap_or(usize::MAX);

        let permit = {
            let output = table.get_mut(&dest)?;
            output.check_write()?
        };
        let len = len.min(permit);
        if len == 0 {
            return Ok(0);
        }

        let contents = match table.get_mut(&src)? {
            InputStream::Host(h) => h.read(len)?,
            InputStream::File(f) => f.read(len).await?,
        };

        let len = contents.len();
        if len == 0 {
            return Ok(0);
        }

        let output = table.get_mut(&dest)?;
        output.write(contents)?;
        Ok(len.try_into().expect("usize can fit in u64"))
    }

    async fn blocking_splice(
        &mut self,
        table: &mut ResourceTable,
        dest: Resource<OutputStream>,
        src: Resource<InputStream>,
        len: u64,
    ) -> StreamResult<u64> {
        use crate::preview2::Subscribe;

        table.get_mut(&dest)?.ready().await;

        table.get_mut(&src)?.ready().await;

        self.splice(table, dest, src, len).await
    }
}

#[async_trait::async_trait]
impl<T: WasiView> streams::HostInputStream for T {
    fn drop(
        &mut self,
        table: &mut ResourceTable,
        stream: Resource<InputStream>,
    ) -> anyhow::Result<()> {
        self.table_mut().delete(stream)?;
        Ok(())
    }

    async fn read(
        &mut self,
        table: &mut ResourceTable,
        stream: Resource<InputStream>,
        len: u64,
    ) -> StreamResult<Vec<u8>> {
        let len = len.try_into().unwrap_or(usize::MAX);
        let bytes = match table.get_mut(&stream)? {
            InputStream::Host(s) => s.read(len)?,
            InputStream::File(s) => s.read(len).await?,
        };
        debug_assert!(bytes.len() <= len);
        Ok(bytes.into())
    }

    async fn blocking_read(
        &mut self,
        table: &mut ResourceTable,
        stream: Resource<InputStream>,
        len: u64,
    ) -> StreamResult<Vec<u8>> {
        if let InputStream::Host(s) = table.get_mut(&stream)? {
            s.ready().await;
        }
        self.read(table, stream, len).await
    }

    async fn skip(
        &mut self,
        table: &mut ResourceTable,
        stream: Resource<InputStream>,
        len: u64,
    ) -> StreamResult<u64> {
        let len = len.try_into().unwrap_or(usize::MAX);
        let written = match table.get_mut(&stream)? {
            InputStream::Host(s) => s.skip(len)?,
            InputStream::File(s) => s.skip(len).await?,
        };
        Ok(written.try_into().expect("usize always fits in u64"))
    }

    async fn blocking_skip(
        &mut self,
        table: &mut ResourceTable,
        stream: Resource<InputStream>,
        len: u64,
    ) -> StreamResult<u64> {
        if let InputStream::Host(s) = table.get_mut(&stream)? {
            s.ready().await;
        }
        self.skip(table, stream, len).await
    }

    fn subscribe(
        &mut self,
        table: &mut ResourceTable,
        stream: Resource<InputStream>,
    ) -> anyhow::Result<Resource<Pollable>> {
        crate::preview2::poll::subscribe(table, stream)
    }
}

pub mod sync {
    use crate::preview2::{
        bindings::io::streams::{
            self as async_streams, Host as AsyncHost, HostInputStream as AsyncHostInputStream,
            HostOutputStream as AsyncHostOutputStream,
        },
        bindings::sync_io::io::poll::Pollable,
        bindings::sync_io::io::streams::{self, InputStream, OutputStream},
        in_tokio, StreamError, StreamResult, WasiView,
    };
    use wasmtime::component::{Resource, ResourceTable};

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
            table: &mut ResourceTable,
            err: StreamError,
        ) -> anyhow::Result<streams::StreamError> {
            Ok(AsyncHost::convert_stream_error(self, table, err)?.into())
        }
    }

    impl<T: WasiView> streams::HostOutputStream for T {
        fn drop(
            &mut self,
            table: &mut ResourceTable,
            stream: Resource<OutputStream>,
        ) -> anyhow::Result<()> {
            AsyncHostOutputStream::drop(self, table, stream)
        }

        fn check_write(
            &mut self,
            table: &mut ResourceTable,
            stream: Resource<OutputStream>,
        ) -> StreamResult<u64> {
            Ok(AsyncHostOutputStream::check_write(self, table, stream)?)
        }

        fn write(
            &mut self,
            table: &mut ResourceTable,
            stream: Resource<OutputStream>,
            bytes: Vec<u8>,
        ) -> StreamResult<()> {
            Ok(AsyncHostOutputStream::write(self, table, stream, bytes)?)
        }

        fn blocking_write_and_flush(
            &mut self,
            table: &mut ResourceTable,
            stream: Resource<OutputStream>,
            bytes: Vec<u8>,
        ) -> StreamResult<()> {
            in_tokio(async {
                AsyncHostOutputStream::blocking_write_and_flush(self, table, stream, bytes).await
            })
        }

        fn blocking_write_zeroes_and_flush(
            &mut self,
            table: &mut ResourceTable,
            stream: Resource<OutputStream>,
            len: u64,
        ) -> StreamResult<()> {
            in_tokio(async {
                AsyncHostOutputStream::blocking_write_zeroes_and_flush(self, table, stream, len)
                    .await
            })
        }

        fn subscribe(
            &mut self,
            table: &mut ResourceTable,
            stream: Resource<OutputStream>,
        ) -> anyhow::Result<Resource<Pollable>> {
            Ok(AsyncHostOutputStream::subscribe(self, table, stream)?)
        }

        fn write_zeroes(
            &mut self,
            table: &mut ResourceTable,
            stream: Resource<OutputStream>,
            len: u64,
        ) -> StreamResult<()> {
            Ok(AsyncHostOutputStream::write_zeroes(
                self, table, stream, len,
            )?)
        }

        fn flush(
            &mut self,
            table: &mut ResourceTable,
            stream: Resource<OutputStream>,
        ) -> StreamResult<()> {
            Ok(AsyncHostOutputStream::flush(
                self,
                table,
                Resource::new_borrow(stream.rep()),
            )?)
        }

        fn blocking_flush(
            &mut self,
            table: &mut ResourceTable,
            stream: Resource<OutputStream>,
        ) -> StreamResult<()> {
            in_tokio(async {
                AsyncHostOutputStream::blocking_flush(
                    self,
                    table,
                    Resource::new_borrow(stream.rep()),
                )
                .await
            })
        }

        fn splice(
            &mut self,
            table: &mut ResourceTable,
            dst: Resource<OutputStream>,
            src: Resource<InputStream>,
            len: u64,
        ) -> StreamResult<u64> {
            in_tokio(async { AsyncHostOutputStream::splice(self, table, dst, src, len).await })
        }

        fn blocking_splice(
            &mut self,
            table: &mut ResourceTable,
            dst: Resource<OutputStream>,
            src: Resource<InputStream>,
            len: u64,
        ) -> StreamResult<u64> {
            in_tokio(async {
                AsyncHostOutputStream::blocking_splice(self, table, dst, src, len).await
            })
        }
    }

    impl<T: WasiView> streams::HostInputStream for T {
        fn drop(
            &mut self,
            table: &mut ResourceTable,
            stream: Resource<InputStream>,
        ) -> anyhow::Result<()> {
            AsyncHostInputStream::drop(self, table, stream)
        }

        fn read(
            &mut self,
            table: &mut ResourceTable,
            stream: Resource<InputStream>,
            len: u64,
        ) -> StreamResult<Vec<u8>> {
            in_tokio(async { AsyncHostInputStream::read(self, table, stream, len).await })
        }

        fn blocking_read(
            &mut self,
            table: &mut ResourceTable,
            stream: Resource<InputStream>,
            len: u64,
        ) -> StreamResult<Vec<u8>> {
            in_tokio(async { AsyncHostInputStream::blocking_read(self, table, stream, len).await })
        }

        fn skip(
            &mut self,
            table: &mut ResourceTable,
            stream: Resource<InputStream>,
            len: u64,
        ) -> StreamResult<u64> {
            in_tokio(async { AsyncHostInputStream::skip(self, table, stream, len).await })
        }

        fn blocking_skip(
            &mut self,
            table: &mut ResourceTable,
            stream: Resource<InputStream>,
            len: u64,
        ) -> StreamResult<u64> {
            in_tokio(async { AsyncHostInputStream::blocking_skip(self, table, stream, len).await })
        }

        fn subscribe(
            &mut self,
            table: &mut ResourceTable,
            stream: Resource<InputStream>,
        ) -> anyhow::Result<Resource<Pollable>> {
            AsyncHostInputStream::subscribe(self, table, stream)
        }
    }
}
