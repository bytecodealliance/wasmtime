use crate::sync::file::convert_systimespec;
use fs_set_times::SetTimes;
use std::any::Any;
use std::io::{self, IsTerminal, Read, Write};
use system_interface::io::ReadReady;

use crate::{
    Error, ErrorExt,
    file::{FdFlags, FileType, WasiFile},
};
#[cfg(windows)]
use io_extras::os::windows::{AsRawHandleOrSocket, RawHandleOrSocket};
#[cfg(unix)]
use io_lifetimes::{AsFd, BorrowedFd};
#[cfg(windows)]
use io_lifetimes::{AsHandle, BorrowedHandle};

pub struct Stdin(std::io::Stdin);

pub fn stdin() -> Stdin {
    Stdin(std::io::stdin())
}

#[wiggle::async_trait]
impl WasiFile for Stdin {
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[cfg(unix)]
    fn pollable(&self) -> Option<rustix::fd::BorrowedFd> {
        Some(self.0.as_fd())
    }

    #[cfg(windows)]
    fn pollable(&self) -> Option<io_extras::os::windows::RawHandleOrSocket> {
        Some(self.0.as_raw_handle_or_socket())
    }

    async fn get_filetype(&self) -> Result<FileType, Error> {
        if self.isatty() {
            Ok(FileType::CharacterDevice)
        } else {
            Ok(FileType::Unknown)
        }
    }
    async fn read_vectored<'a>(&self, bufs: &mut [io::IoSliceMut<'a>]) -> Result<u64, Error> {
        let n = self.0.lock().read_vectored(bufs)?;
        Ok(n.try_into().map_err(|_| Error::range())?)
    }
    async fn read_vectored_at<'a>(
        &self,
        _bufs: &mut [io::IoSliceMut<'a>],
        _offset: u64,
    ) -> Result<u64, Error> {
        Err(Error::seek_pipe())
    }
    async fn seek(&self, _pos: std::io::SeekFrom) -> Result<u64, Error> {
        Err(Error::seek_pipe())
    }
    async fn peek(&self, _buf: &mut [u8]) -> Result<u64, Error> {
        Err(Error::seek_pipe())
    }
    async fn set_times(
        &self,
        atime: Option<crate::SystemTimeSpec>,
        mtime: Option<crate::SystemTimeSpec>,
    ) -> Result<(), Error> {
        self.0
            .set_times(convert_systimespec(atime), convert_systimespec(mtime))?;
        Ok(())
    }
    fn num_ready_bytes(&self) -> Result<u64, Error> {
        Ok(self.0.num_ready_bytes()?)
    }
    fn isatty(&self) -> bool {
        #[cfg(unix)]
        return self.0.as_fd().is_terminal();
        #[cfg(windows)]
        return self.0.as_handle().is_terminal();
    }
}
#[cfg(windows)]
impl AsHandle for Stdin {
    fn as_handle(&self) -> BorrowedHandle<'_> {
        self.0.as_handle()
    }
}
#[cfg(windows)]
impl AsRawHandleOrSocket for Stdin {
    #[inline]
    fn as_raw_handle_or_socket(&self) -> RawHandleOrSocket {
        self.0.as_raw_handle_or_socket()
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
        #[wiggle::async_trait]
        impl WasiFile for $ty {
            fn as_any(&self) -> &dyn Any {
                self
            }
            #[cfg(unix)]
            fn pollable(&self) -> Option<rustix::fd::BorrowedFd> {
                Some(self.0.as_fd())
            }
            #[cfg(windows)]
            fn pollable(&self) -> Option<io_extras::os::windows::RawHandleOrSocket> {
                Some(self.0.as_raw_handle_or_socket())
            }
            async fn get_filetype(&self) -> Result<FileType, Error> {
                if self.isatty() {
                    Ok(FileType::CharacterDevice)
                } else {
                    Ok(FileType::Unknown)
                }
            }
            async fn get_fdflags(&self) -> Result<FdFlags, Error> {
                Ok(FdFlags::APPEND)
            }
            async fn write_vectored<'a>(&self, bufs: &[io::IoSlice<'a>]) -> Result<u64, Error> {
                let mut io = self.0.lock();
                let n = io.write_vectored(bufs)?;
                // On a successful write additionally flush out the bytes to
                // handle stdio buffering done by libstd since WASI interfaces
                // here aren't buffered.
                io.flush()?;
                Ok(n.try_into().map_err(|_| {
                    Error::range().context("converting write_vectored total length")
                })?)
            }
            async fn write_vectored_at<'a>(
                &self,
                _bufs: &[io::IoSlice<'a>],
                _offset: u64,
            ) -> Result<u64, Error> {
                Err(Error::seek_pipe())
            }
            async fn seek(&self, _pos: std::io::SeekFrom) -> Result<u64, Error> {
                Err(Error::seek_pipe())
            }
            async fn set_times(
                &self,
                atime: Option<crate::SystemTimeSpec>,
                mtime: Option<crate::SystemTimeSpec>,
            ) -> Result<(), Error> {
                self.0
                    .set_times(convert_systimespec(atime), convert_systimespec(mtime))?;
                Ok(())
            }
            fn isatty(&self) -> bool {
                self.0.is_terminal()
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
        impl AsRawHandleOrSocket for $ty {
            #[inline]
            fn as_raw_handle_or_socket(&self) -> RawHandleOrSocket {
                self.0.as_raw_handle_or_socket()
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
