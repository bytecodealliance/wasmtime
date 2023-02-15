use std::any::Any;
use std::convert::TryInto;
use std::io;
use std::io::{Read, Write};
use system_interface::io::ReadReady;

#[cfg(windows)]
use io_extras::os::windows::{AsHandleOrSocket, BorrowedHandleOrSocket};
#[cfg(unix)]
use io_lifetimes::{AsFd, BorrowedFd};
#[cfg(windows)]
use io_lifetimes::{AsHandle, BorrowedHandle};
use wasi_common::{
    stream::{InputStream, OutputStream},
    Error, ErrorExt,
};

pub struct Stdin(std::io::Stdin);

pub fn stdin() -> Stdin {
    Stdin(std::io::stdin())
}

#[async_trait::async_trait]
impl InputStream for Stdin {
    fn as_any(&self) -> &dyn Any {
        self
    }
    #[cfg(unix)]
    fn pollable_read(&self) -> Option<rustix::fd::BorrowedFd> {
        Some(self.0.as_fd())
    }

    #[cfg(windows)]
    fn pollable_read(&self) -> Option<io_extras::os::windows::BorrowedHandleOrSocket> {
        Some(self.0.as_handle_or_socket())
    }

    async fn read(&mut self, buf: &mut [u8]) -> Result<(u64, bool), Error> {
        match Read::read(&mut self.0, buf) {
            Ok(0) => Ok((0, true)),
            Ok(n) => Ok((n as u64, false)),
            Err(err) if err.kind() == io::ErrorKind::Interrupted => Ok((0, false)),
            Err(err) => Err(err.into()),
        }
    }
    async fn read_vectored<'a>(
        &mut self,
        bufs: &mut [io::IoSliceMut<'a>],
    ) -> Result<(u64, bool), Error> {
        match Read::read_vectored(&mut self.0, bufs) {
            Ok(0) => Ok((0, true)),
            Ok(n) => Ok((n as u64, false)),
            Err(err) if err.kind() == io::ErrorKind::Interrupted => Ok((0, false)),
            Err(err) => Err(err.into()),
        }
    }
    #[cfg(can_vector)]
    fn is_read_vectored(&self) {
        Read::is_read_vectored(&mut self.0)
    }

    async fn skip(&mut self, nelem: u64) -> Result<(u64, bool), Error> {
        let num = io::copy(&mut io::Read::take(&mut self.0, nelem), &mut io::sink())?;
        Ok((num, num < nelem))
    }

    async fn num_ready_bytes(&self) -> Result<u64, Error> {
        Ok(self.0.num_ready_bytes()?)
    }

    async fn readable(&self) -> Result<(), Error> {
        Err(Error::badf())
    }
}
#[cfg(windows)]
impl AsHandle for Stdin {
    fn as_handle(&self) -> BorrowedHandle<'_> {
        self.0.as_handle()
    }
}
#[cfg(windows)]
impl AsHandleOrSocket for Stdin {
    #[inline]
    fn as_handle_or_socket(&self) -> BorrowedHandleOrSocket {
        self.0.as_handle_or_socket()
    }
}
#[cfg(unix)]
impl AsFd for Stdin {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

macro_rules! wasi_file_write_impl {
    ($ty:ty, $ident:ident) => {
        #[async_trait::async_trait]
        impl OutputStream for $ty {
            fn as_any(&self) -> &dyn Any {
                self
            }

            #[cfg(unix)]
            fn pollable_write(&self) -> Option<rustix::fd::BorrowedFd> {
                Some(self.0.as_fd())
            }
            #[cfg(windows)]
            fn pollable_write(&self) -> Option<io_extras::os::windows::BorrowedHandleOrSocket> {
                Some(self.0.as_handle_or_socket())
            }

            async fn write(&mut self, buf: &[u8]) -> Result<u64, Error> {
                let n = Write::write(&mut self.0, buf)?;
                Ok(n.try_into()?)
            }
            async fn write_vectored<'a>(&mut self, bufs: &[io::IoSlice<'a>]) -> Result<u64, Error> {
                let n = Write::write_vectored(&mut self.0, bufs)?;
                Ok(n.try_into()?)
            }
            #[cfg(can_vector)]
            fn is_write_vectored(&self) {
                Write::is_write_vectored(&mut self.0)
            }
            // TODO: Optimize for stdio streams.
            /*
            async fn splice(
                &mut self,
                src: &mut dyn InputStream,
                nelem: u64,
            ) -> Result<u64, Error> {
                todo!()
            }
            */

            async fn write_zeroes(&mut self, nelem: u64) -> Result<u64, Error> {
                let num = io::copy(&mut io::Read::take(io::repeat(0), nelem), &mut self.0)?;
                Ok(num)
            }

            async fn writable(&self) -> Result<(), Error> {
                Ok(())
            }
        }
        #[cfg(windows)]
        impl AsHandle for $ty {
            fn as_handle(&self) -> BorrowedHandle<'_> {
                self.0.as_handle()
            }
        }
        #[cfg(unix)]
        impl AsFd for $ty {
            fn as_fd(&self) -> BorrowedFd<'_> {
                self.0.as_fd()
            }
        }
        #[cfg(windows)]
        impl AsHandleOrSocket for $ty {
            #[inline]
            fn as_handle_or_socket(&self) -> BorrowedHandleOrSocket {
                self.0.as_handle_or_socket()
            }
        }
    };
}

pub struct Stdout(std::io::Stdout);

pub fn stdout() -> Stdout {
    Stdout(std::io::stdout())
}
wasi_file_write_impl!(Stdout, Stdout);

pub struct Stderr(std::io::Stderr);

pub fn stderr() -> Stderr {
    Stderr(std::io::stderr())
}
wasi_file_write_impl!(Stderr, Stderr);
