use crate::bindings::wasi::io::{error, streams};
use crate::poll::{subscribe, Pollable};
use crate::stream::{InputStream, OutputStream, StreamError, StreamResult};
use crate::view::{IoImpl, IoView};
use wasmtime::component::Resource;

impl<T> error::Host for IoImpl<T> where T: IoView {}

impl<T> streams::Host for IoImpl<T>
where
    T: IoView,
{
    fn convert_stream_error(&mut self, err: StreamError) -> anyhow::Result<streams::StreamError> {
        match err {
            StreamError::Closed => Ok(streams::StreamError::Closed),
            StreamError::LastOperationFailed(e) => Ok(streams::StreamError::LastOperationFailed(
                self.table().push(e)?,
            )),
            StreamError::Trap(e) => Err(e),
        }
    }
}

impl<T> error::HostError for IoImpl<T>
where
    T: IoView,
{
    fn drop(&mut self, err: Resource<streams::Error>) -> anyhow::Result<()> {
        self.table().delete(err)?;
        Ok(())
    }

    fn to_debug_string(&mut self, err: Resource<streams::Error>) -> anyhow::Result<String> {
        Ok(format!("{:?}", self.table().get(&err)?))
    }
}

impl<T> streams::HostOutputStream for IoImpl<T>
where
    T: IoView,
{
    async fn drop(&mut self, stream: Resource<OutputStream>) -> anyhow::Result<()> {
        self.table().delete(stream)?.cancel().await;
        Ok(())
    }

    fn check_write(&mut self, stream: Resource<OutputStream>) -> StreamResult<u64> {
        let bytes = self.table().get_mut(&stream)?.check_write()?;
        Ok(bytes as u64)
    }

    fn write(&mut self, stream: Resource<OutputStream>, bytes: Vec<u8>) -> StreamResult<()> {
        self.table().get_mut(&stream)?.write(bytes.into())?;
        Ok(())
    }

    fn subscribe(&mut self, stream: Resource<OutputStream>) -> anyhow::Result<Resource<Pollable>> {
        subscribe(self.table(), stream)
    }

    async fn blocking_write_and_flush(
        &mut self,
        stream: Resource<OutputStream>,
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
        stream: Resource<OutputStream>,
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

    fn write_zeroes(&mut self, stream: Resource<OutputStream>, len: u64) -> StreamResult<()> {
        self.table().get_mut(&stream)?.write_zeroes(len as usize)?;
        Ok(())
    }

    fn flush(&mut self, stream: Resource<OutputStream>) -> StreamResult<()> {
        self.table().get_mut(&stream)?.flush()?;
        Ok(())
    }

    async fn blocking_flush(&mut self, stream: Resource<OutputStream>) -> StreamResult<()> {
        let s = self.table().get_mut(&stream)?;
        s.flush()?;
        s.write_ready().await?;
        Ok(())
    }

    fn splice(
        &mut self,
        dest: Resource<OutputStream>,
        src: Resource<InputStream>,
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
        dest: Resource<OutputStream>,
        src: Resource<InputStream>,
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

impl<T> streams::HostInputStream for IoImpl<T>
where
    T: IoView,
{
    async fn drop(&mut self, stream: Resource<InputStream>) -> anyhow::Result<()> {
        self.table().delete(stream)?.cancel().await;
        Ok(())
    }

    fn read(&mut self, stream: Resource<InputStream>, len: u64) -> StreamResult<Vec<u8>> {
        let len = len.try_into().unwrap_or(usize::MAX);
        let bytes = self.table().get_mut(&stream)?.read(len)?;
        debug_assert!(bytes.len() <= len);
        Ok(bytes.into())
    }

    async fn blocking_read(
        &mut self,
        stream: Resource<InputStream>,
        len: u64,
    ) -> StreamResult<Vec<u8>> {
        let len = len.try_into().unwrap_or(usize::MAX);
        let bytes = self.table().get_mut(&stream)?.blocking_read(len).await?;
        debug_assert!(bytes.len() <= len);
        Ok(bytes.into())
    }

    fn skip(&mut self, stream: Resource<InputStream>, len: u64) -> StreamResult<u64> {
        let len = len.try_into().unwrap_or(usize::MAX);
        let written = self.table().get_mut(&stream)?.skip(len)?;
        Ok(written.try_into().expect("usize always fits in u64"))
    }

    async fn blocking_skip(
        &mut self,
        stream: Resource<InputStream>,
        len: u64,
    ) -> StreamResult<u64> {
        let len = len.try_into().unwrap_or(usize::MAX);
        let written = self.table().get_mut(&stream)?.blocking_skip(len).await?;
        Ok(written.try_into().expect("usize always fits in u64"))
    }

    fn subscribe(&mut self, stream: Resource<InputStream>) -> anyhow::Result<Resource<Pollable>> {
        crate::poll::subscribe(self.table(), stream)
    }
}
