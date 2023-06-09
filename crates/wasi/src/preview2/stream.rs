use crate::preview2::{HostPollable, Table, TableError};
use anyhow::Error;

// TODO(elliottt): wrapper of (Box<dyn tokio::io::AsyncRead>, buffering), then impl HostInputStream
// for that type, then do the same for a dyn AsyncWrite with buffering for HostOutputStream.
// Testing could be to send the guest a timestamp and have them check that it's 100ms in the past?
// Try putting this in the command tests.

/// An input bytestream.
///
/// This is "pseudo" because the real streams will be a type in wit, and
/// built into the wit bindings, and will support async and type parameters.
/// This pseudo-stream abstraction is synchronous and only supports bytes.
#[async_trait::async_trait]
pub trait HostInputStream: Send + Sync {
    /// Read bytes. On success, returns a pair holding the number of bytes read
    /// and a flag indicating whether the end of the stream was reached.
    async fn read(&mut self, buf: &mut [u8]) -> Result<(u64, bool), Error>;

    /// Vectored-I/O form of `read`.
    async fn read_vectored<'a>(
        &mut self,
        bufs: &mut [std::io::IoSliceMut<'a>],
    ) -> Result<(u64, bool), Error> {
        if bufs.len() > 0 {
            self.read(bufs.get_mut(0).unwrap()).await
        } else {
            self.read(&mut []).await
        }
    }

    /// Test whether vectored I/O reads are known to be optimized in the
    /// underlying implementation.
    fn is_read_vectored(&self) -> bool {
        false
    }

    /// Read bytes from a stream and discard them.
    async fn skip(&mut self, nelem: u64) -> Result<(u64, bool), Error> {
        let mut nread = 0;
        let mut saw_end = false;

        // TODO: Optimize by reading more than one byte at a time.
        for _ in 0..nelem {
            let (num, end) = self.read(&mut [0]).await?;
            nread += num;
            if end {
                saw_end = true;
                break;
            }
        }

        Ok((nread, saw_end))
    }

    /// Get the Pollable implementation for read readiness.
    fn pollable(&self) -> HostPollable;
}

/// An output bytestream.
///
/// This is "pseudo" because the real streams will be a type in wit, and
/// built into the wit bindings, and will support async and type parameters.
/// This pseudo-stream abstraction is synchronous and only supports bytes.
#[async_trait::async_trait]
pub trait HostOutputStream: Send + Sync {
    /// Write bytes. On success, returns the number of bytes written.
    async fn write(&mut self, _buf: &[u8]) -> Result<u64, Error>;

    /// Vectored-I/O form of `write`.
    async fn write_vectored<'a>(&mut self, bufs: &[std::io::IoSlice<'a>]) -> Result<u64, Error> {
        if bufs.len() > 0 {
            self.write(bufs.get(0).unwrap()).await
        } else {
            Ok(0)
        }
    }

    /// Test whether vectored I/O writes are known to be optimized in the
    /// underlying implementation.
    fn is_write_vectored(&self) -> bool {
        false
    }

    /// Transfer bytes directly from an input stream to an output stream.
    async fn splice(
        &mut self,
        src: &mut dyn HostInputStream,
        nelem: u64,
    ) -> Result<(u64, bool), Error> {
        let mut nspliced = 0;
        let mut saw_end = false;

        // TODO: Optimize by splicing more than one byte at a time.
        for _ in 0..nelem {
            let mut buf = [0u8];
            let (num, end) = src.read(&mut buf).await?;
            self.write(&buf).await?;
            nspliced += num;
            if end {
                saw_end = true;
                break;
            }
        }

        Ok((nspliced, saw_end))
    }

    /// Repeatedly write a byte to a stream.
    async fn write_zeroes(&mut self, nelem: u64) -> Result<u64, Error> {
        let mut nwritten = 0;

        // TODO: Optimize by writing more than one byte at a time.
        for _ in 0..nelem {
            let num = self.write(&[0]).await?;
            if num == 0 {
                break;
            }
            nwritten += num;
        }

        Ok(nwritten)
    }

    /// Get the Pollable implementation for write readiness.
    fn pollable(&self) -> HostPollable;
}

pub trait TableStreamExt {
    fn push_input_stream(&mut self, istream: Box<dyn HostInputStream>) -> Result<u32, TableError>;
    fn get_input_stream(&self, fd: u32) -> Result<&dyn HostInputStream, TableError>;
    fn get_input_stream_mut(
        &mut self,
        fd: u32,
    ) -> Result<&mut Box<dyn HostInputStream>, TableError>;

    fn push_output_stream(&mut self, ostream: Box<dyn HostOutputStream>)
        -> Result<u32, TableError>;
    fn get_output_stream(&self, fd: u32) -> Result<&dyn HostOutputStream, TableError>;
    fn get_output_stream_mut(
        &mut self,
        fd: u32,
    ) -> Result<&mut Box<dyn HostOutputStream>, TableError>;
}
impl TableStreamExt for Table {
    fn push_input_stream(&mut self, istream: Box<dyn HostInputStream>) -> Result<u32, TableError> {
        self.push(Box::new(istream))
    }
    fn get_input_stream(&self, fd: u32) -> Result<&dyn HostInputStream, TableError> {
        self.get::<Box<dyn HostInputStream>>(fd).map(|f| f.as_ref())
    }
    fn get_input_stream_mut(
        &mut self,
        fd: u32,
    ) -> Result<&mut Box<dyn HostInputStream>, TableError> {
        self.get_mut::<Box<dyn HostInputStream>>(fd)
    }

    fn push_output_stream(
        &mut self,
        ostream: Box<dyn HostOutputStream>,
    ) -> Result<u32, TableError> {
        self.push(Box::new(ostream))
    }
    fn get_output_stream(&self, fd: u32) -> Result<&dyn HostOutputStream, TableError> {
        self.get::<Box<dyn HostOutputStream>>(fd)
            .map(|f| f.as_ref())
    }
    fn get_output_stream_mut(
        &mut self,
        fd: u32,
    ) -> Result<&mut Box<dyn HostOutputStream>, TableError> {
        self.get_mut::<Box<dyn HostOutputStream>>(fd)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::preview2::pipe::{ReadPipe, WritePipe};
    #[test]
    fn input_stream_in_table() {
        let empty_pipe = ReadPipe::new(std::io::empty());
        let mut table = Table::new();
        let ix = table.push_input_stream(Box::new(empty_pipe)).unwrap();
        let _ = table.get_input_stream(ix).unwrap();
        let _ = table.get_input_stream_mut(ix).unwrap();
    }

    #[test]
    fn output_stream_in_table() {
        let dev_null = WritePipe::new(std::io::sink());
        let mut table = Table::new();
        let ix = table.push_output_stream(Box::new(dev_null)).unwrap();
        let _ = table.get_output_stream(ix).unwrap();
        let _ = table.get_output_stream_mut(ix).unwrap();
    }
}
