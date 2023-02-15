use crate::{Error, ErrorExt};
use std::any::Any;

/// An input bytestream.
///
/// This is "pseudo" because the real streams will be a type in wit, and
/// built into the wit bindings, and will support async and type parameters.
/// This pseudo-stream abstraction is synchronous and only supports bytes.
#[async_trait::async_trait]
pub trait InputStream: Send + Sync {
    fn as_any(&self) -> &dyn Any;

    /// If this stream is reading from a host file descriptor, return it so
    /// that it can be polled with a host poll.
    #[cfg(unix)]
    fn pollable_read(&self) -> Option<rustix::fd::BorrowedFd> {
        None
    }

    /// If this stream is reading from a host file descriptor, return it so
    /// that it can be polled with a host poll.
    #[cfg(windows)]
    fn pollable_read(&self) -> Option<io_extras::os::windows::BorrowedHandleOrSocket> {
        None
    }

    /// Read bytes. On success, returns a pair holding the number of bytes read
    /// and a flag indicating whether the end of the stream was reached.
    async fn read(&mut self, _buf: &mut [u8]) -> Result<(u64, bool), Error> {
        Err(Error::badf())
    }

    /// Vectored-I/O form of `read`.
    async fn read_vectored<'a>(
        &mut self,
        _bufs: &mut [std::io::IoSliceMut<'a>],
    ) -> Result<(u64, bool), Error> {
        Err(Error::badf())
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

    /// Return the number of bytes that may be read without blocking.
    async fn num_ready_bytes(&self) -> Result<u64, Error> {
        Ok(0)
    }

    /// Test whether this stream is readable.
    async fn readable(&self) -> Result<(), Error>;
}

/// An output bytestream.
///
/// This is "pseudo" because the real streams will be a type in wit, and
/// built into the wit bindings, and will support async and type parameters.
/// This pseudo-stream abstraction is synchronous and only supports bytes.
#[async_trait::async_trait]
pub trait OutputStream: Send + Sync {
    fn as_any(&self) -> &dyn Any;

    /// If this stream is writing from a host file descriptor, return it so
    /// that it can be polled with a host poll.
    #[cfg(unix)]
    fn pollable_write(&self) -> Option<rustix::fd::BorrowedFd> {
        None
    }

    /// If this stream is writing from a host file descriptor, return it so
    /// that it can be polled with a host poll.
    #[cfg(windows)]
    fn pollable_write(&self) -> Option<io_extras::os::windows::BorrowedHandleOrSocket> {
        None
    }

    /// Write bytes. On success, returns the number of bytes written.
    async fn write(&mut self, _buf: &[u8]) -> Result<u64, Error> {
        Err(Error::badf())
    }

    /// Vectored-I/O form of `write`.
    async fn write_vectored<'a>(&mut self, _bufs: &[std::io::IoSlice<'a>]) -> Result<u64, Error> {
        Err(Error::badf())
    }

    /// Test whether vectored I/O writes are known to be optimized in the
    /// underlying implementation.
    fn is_write_vectored(&self) -> bool {
        false
    }

    /// Transfer bytes directly from an input stream to an output stream.
    async fn splice(
        &mut self,
        src: &mut dyn InputStream,
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

    /// Test whether this stream is writeable.
    async fn writable(&self) -> Result<(), Error>;
}

pub trait TableStreamExt {
    fn get_input_stream(&self, fd: u32) -> Result<&dyn InputStream, Error>;
    fn get_input_stream_mut(&mut self, fd: u32) -> Result<&mut Box<dyn InputStream>, Error>;

    fn get_output_stream(&self, fd: u32) -> Result<&dyn OutputStream, Error>;
    fn get_output_stream_mut(&mut self, fd: u32) -> Result<&mut Box<dyn OutputStream>, Error>;
}
impl TableStreamExt for crate::table::Table {
    fn get_input_stream(&self, fd: u32) -> Result<&dyn InputStream, Error> {
        self.get::<Box<dyn InputStream>>(fd).map(|f| f.as_ref())
    }
    fn get_input_stream_mut(&mut self, fd: u32) -> Result<&mut Box<dyn InputStream>, Error> {
        self.get_mut::<Box<dyn InputStream>>(fd)
    }

    fn get_output_stream(&self, fd: u32) -> Result<&dyn OutputStream, Error> {
        self.get::<Box<dyn OutputStream>>(fd).map(|f| f.as_ref())
    }
    fn get_output_stream_mut(&mut self, fd: u32) -> Result<&mut Box<dyn OutputStream>, Error> {
        self.get_mut::<Box<dyn OutputStream>>(fd)
    }
}
