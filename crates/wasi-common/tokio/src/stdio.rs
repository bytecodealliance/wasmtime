use crate::file::convert_systimespec;
use fs_set_times::SetTimes;
use std::any::Any;
use std::convert::TryInto;
use std::io;
use std::io::{Read, Write};

use unsafe_io::AsUnsafeFile;
use wasi_common::{
    file::{Advice, FdFlags, FileType, Filestat, WasiFile},
    Error, ErrorExt,
};

mod internal {
    #[cfg(unix)]
    use std::os::unix::io::{AsRawFd, RawFd};
    #[cfg(windows)]
    use std::os::windows::io::{AsRawHandle, RawHandle};
    use unsafe_io::OwnsRaw;

    pub(super) struct TokioStdin(tokio::io::Stdin);
    impl TokioStdin {
        pub fn new() -> Self {
            TokioStdin(tokio::io::stdin())
        }
    }

    #[cfg(windows)]
    impl AsRawHandle for TokioStdin {
        fn as_raw_handle(&self) -> RawHandle {
            self.0.as_raw_handle()
        }
    }
    #[cfg(unix)]
    impl AsRawFd for TokioStdin {
        fn as_raw_fd(&self) -> RawFd {
            self.0.as_raw_fd()
        }
    }
    unsafe impl OwnsRaw for TokioStdin {}

    pub(super) struct TokioStdout(tokio::io::Stdout);
    impl TokioStdout {
        pub fn new() -> Self {
            TokioStdout(tokio::io::stdout())
        }
    }

    #[cfg(windows)]
    impl AsRawHandle for TokioStdout {
        fn as_raw_handle(&self) -> RawHandle {
            self.0.as_raw_handle()
        }
    }
    #[cfg(unix)]
    impl AsRawFd for TokioStdout {
        fn as_raw_fd(&self) -> RawFd {
            self.0.as_raw_fd()
        }
    }
    unsafe impl OwnsRaw for TokioStdout {}

    pub(super) struct TokioStderr(tokio::io::Stderr);
    impl TokioStderr {
        pub fn new() -> Self {
            TokioStderr(tokio::io::stderr())
        }
    }
    #[cfg(windows)]
    impl AsRawHandle for TokioStderr {
        fn as_raw_handle(&self) -> RawHandle {
            self.0.as_raw_handle()
        }
    }
    #[cfg(unix)]
    impl AsRawFd for TokioStderr {
        fn as_raw_fd(&self) -> RawFd {
            self.0.as_raw_fd()
        }
    }
    unsafe impl OwnsRaw for TokioStderr {}
}

pub struct Stdin(internal::TokioStdin);

pub fn stdin() -> Stdin {
    Stdin(internal::TokioStdin::new())
}

#[wiggle::async_trait]
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
        Ok(FileType::Unknown)
    }
    async fn get_fdflags(&self) -> Result<FdFlags, Error> {
        Ok(FdFlags::empty())
    }
    async fn set_fdflags(&mut self, _fdflags: FdFlags) -> Result<(), Error> {
        Err(Error::badf())
    }
    async fn get_filestat(&self) -> Result<Filestat, Error> {
        let meta = self.0.as_file_view().metadata()?;
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
        let n = self.0.as_file_view().read_vectored(bufs)?;
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

    #[cfg(not(windows))]
    async fn num_ready_bytes(&self) -> Result<u64, Error> {
        Ok(posish::io::fionread(&self.0)?)
    }
    #[cfg(windows)]
    async fn num_ready_bytes(&self) -> Result<u64, Error> {
        // conservative but correct is the best we can do
        Ok(0)
    }

    async fn readable(&mut self) -> Result<(), Error> {
        Err(Error::badf())
    }
    async fn writable(&mut self) -> Result<(), Error> {
        Err(Error::badf())
    }
}

macro_rules! wasi_file_write_impl {
    ($ty:ty) => {
        #[wiggle::async_trait]
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
                Ok(FileType::Unknown)
            }
            async fn get_fdflags(&self) -> Result<FdFlags, Error> {
                Ok(FdFlags::APPEND)
            }
            async fn set_fdflags(&mut self, _fdflags: FdFlags) -> Result<(), Error> {
                Err(Error::badf())
            }
            async fn get_filestat(&self) -> Result<Filestat, Error> {
                let meta = self.0.as_file_view().metadata()?;
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
                let n = self.0.as_file_view().write_vectored(bufs)?;
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
            async fn readable(&mut self) -> Result<(), Error> {
                Err(Error::badf())
            }
            async fn writable(&mut self) -> Result<(), Error> {
                Err(Error::badf())
            }
        }
    };
}

pub struct Stdout(internal::TokioStdout);

pub fn stdout() -> Stdout {
    Stdout(internal::TokioStdout::new())
}
wasi_file_write_impl!(Stdout);

pub struct Stderr(internal::TokioStderr);

pub fn stderr() -> Stderr {
    Stderr(internal::TokioStderr::new())
}
wasi_file_write_impl!(Stderr);
