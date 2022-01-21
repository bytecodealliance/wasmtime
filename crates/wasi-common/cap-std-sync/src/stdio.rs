use crate::file::convert_systimespec;
use fs_set_times::SetTimes;
use io_lifetimes::AsFilelike;
use std::any::Any;
use std::convert::TryInto;
use std::fs::File;
use std::io;
use std::io::{Read, Write};
use system_interface::io::ReadReady;

#[cfg(windows)]
use io_extras::os::windows::{AsRawHandleOrSocket, RawHandleOrSocket};
#[cfg(unix)]
use io_lifetimes::{AsFd, BorrowedFd};
#[cfg(windows)]
use io_lifetimes::{AsHandle, BorrowedHandle};
use wasi_common::{
    file::{Advice, FdFlags, FileType, Filestat, WasiFile},
    Error, ErrorExt,
};

pub struct Stdin(std::io::Stdin);

pub fn stdin() -> Stdin {
    Stdin(std::io::stdin())
}

#[async_trait::async_trait]
impl WasiFile for Stdin {
    fn as_any(&self) -> &dyn Any {
        self
    }
    async fn datasync(&self) -> Result<(), Error> {
        Ok(())
    }
    async fn sync(&self) -> Result<(), Error> {
        Ok(())
    }
    async fn get_filetype(&self) -> Result<FileType, Error> {
        if self.isatty() {
            Ok(FileType::CharacterDevice)
        } else {
            Ok(FileType::Unknown)
        }
    }
    async fn get_fdflags(&self) -> Result<FdFlags, Error> {
        Ok(FdFlags::empty())
    }
    async fn set_fdflags(&mut self, _fdflags: FdFlags) -> Result<(), Error> {
        Err(Error::badf())
    }
    async fn get_filestat(&self) -> Result<Filestat, Error> {
        let meta = self.0.as_filelike_view::<File>().metadata()?;
        Ok(Filestat {
            device_id: 0,
            inode: 0,
            filetype: self.get_filetype().await?,
            nlink: 0,
            size: meta.len(),
            atim: meta.accessed().ok(),
            mtim: meta.modified().ok(),
            ctim: meta.created().ok(),
        })
    }
    async fn set_filestat_size(&self, _size: u64) -> Result<(), Error> {
        Err(Error::badf())
    }
    async fn advise(&self, _offset: u64, _len: u64, _advice: Advice) -> Result<(), Error> {
        Err(Error::badf())
    }
    async fn allocate(&self, _offset: u64, _len: u64) -> Result<(), Error> {
        Err(Error::badf())
    }
    async fn read_vectored<'a>(&self, bufs: &mut [io::IoSliceMut<'a>]) -> Result<u64, Error> {
        let n = self.0.as_filelike_view::<File>().read_vectored(bufs)?;
        Ok(n.try_into().map_err(|_| Error::range())?)
    }
    async fn read_vectored_at<'a>(
        &self,
        _bufs: &mut [io::IoSliceMut<'a>],
        _offset: u64,
    ) -> Result<u64, Error> {
        Err(Error::seek_pipe())
    }
    async fn write_vectored<'a>(&self, _bufs: &[io::IoSlice<'a>]) -> Result<u64, Error> {
        Err(Error::badf())
    }
    async fn write_vectored_at<'a>(
        &self,
        _bufs: &[io::IoSlice<'a>],
        _offset: u64,
    ) -> Result<u64, Error> {
        Err(Error::badf())
    }
    async fn seek(&self, _pos: std::io::SeekFrom) -> Result<u64, Error> {
        Err(Error::seek_pipe())
    }
    async fn peek(&self, _buf: &mut [u8]) -> Result<u64, Error> {
        Err(Error::seek_pipe())
    }
    async fn set_times(
        &self,
        atime: Option<wasi_common::SystemTimeSpec>,
        mtime: Option<wasi_common::SystemTimeSpec>,
    ) -> Result<(), Error> {
        self.0
            .set_times(convert_systimespec(atime), convert_systimespec(mtime))?;
        Ok(())
    }
    async fn num_ready_bytes(&self) -> Result<u64, Error> {
        Ok(self.0.num_ready_bytes()?)
    }
    fn isatty(&self) -> bool {
        #[cfg(unix)]
        {
            rustix::io::isatty(&self.0)
        }
        #[cfg(not(unix))]
        {
            atty::is(atty::Stream::Stdin)
        }
    }
    async fn readable(&self) -> Result<(), Error> {
        Err(Error::badf())
    }
    async fn writable(&self) -> Result<(), Error> {
        Err(Error::badf())
    }

    async fn sock_accept(&mut self, _fdflags: FdFlags) -> Result<Box<dyn WasiFile>, Error> {
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
        #[async_trait::async_trait]
        impl WasiFile for $ty {
            fn as_any(&self) -> &dyn Any {
                self
            }
            async fn datasync(&self) -> Result<(), Error> {
                Ok(())
            }
            async fn sync(&self) -> Result<(), Error> {
                Ok(())
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
            async fn set_fdflags(&mut self, _fdflags: FdFlags) -> Result<(), Error> {
                Err(Error::badf())
            }
            async fn get_filestat(&self) -> Result<Filestat, Error> {
                let meta = self.0.as_filelike_view::<File>().metadata()?;
                Ok(Filestat {
                    device_id: 0,
                    inode: 0,
                    filetype: self.get_filetype().await?,
                    nlink: 0,
                    size: meta.len(),
                    atim: meta.accessed().ok(),
                    mtim: meta.modified().ok(),
                    ctim: meta.created().ok(),
                })
            }
            async fn set_filestat_size(&self, _size: u64) -> Result<(), Error> {
                Err(Error::badf())
            }
            async fn advise(&self, _offset: u64, _len: u64, _advice: Advice) -> Result<(), Error> {
                Err(Error::badf())
            }
            async fn allocate(&self, _offset: u64, _len: u64) -> Result<(), Error> {
                Err(Error::badf())
            }
            async fn read_vectored<'a>(
                &self,
                _bufs: &mut [io::IoSliceMut<'a>],
            ) -> Result<u64, Error> {
                Err(Error::badf())
            }
            async fn read_vectored_at<'a>(
                &self,
                _bufs: &mut [io::IoSliceMut<'a>],
                _offset: u64,
            ) -> Result<u64, Error> {
                Err(Error::badf())
            }
            async fn write_vectored<'a>(&self, bufs: &[io::IoSlice<'a>]) -> Result<u64, Error> {
                let n = self.0.as_filelike_view::<File>().write_vectored(bufs)?;
                Ok(n.try_into().map_err(|c| Error::range().context(c))?)
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
            async fn peek(&self, _buf: &mut [u8]) -> Result<u64, Error> {
                Err(Error::badf())
            }
            async fn set_times(
                &self,
                atime: Option<wasi_common::SystemTimeSpec>,
                mtime: Option<wasi_common::SystemTimeSpec>,
            ) -> Result<(), Error> {
                self.0
                    .set_times(convert_systimespec(atime), convert_systimespec(mtime))?;
                Ok(())
            }
            async fn num_ready_bytes(&self) -> Result<u64, Error> {
                Ok(0)
            }
            fn isatty(&self) -> bool {
                #[cfg(unix)]
                {
                    rustix::io::isatty(&self.0)
                }
                #[cfg(not(unix))]
                {
                    atty::is(atty::Stream::$ident)
                }
            }
            async fn readable(&self) -> Result<(), Error> {
                Err(Error::badf())
            }
            async fn writable(&self) -> Result<(), Error> {
                Err(Error::badf())
            }
            async fn sock_accept(&mut self, _fdflags: FdFlags) -> Result<Box<dyn WasiFile>, Error> {
                Err(Error::badf())
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
