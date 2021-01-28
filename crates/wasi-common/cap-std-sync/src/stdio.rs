use crate::file::convert_systimespec;
use fs_set_times::SetTimes;
use std::any::Any;
use std::convert::TryInto;
use std::io;
use std::io::{Read, Write};
use system_interface::{fs::Advice, io::ReadReady};

#[cfg(unix)]
use std::os::unix::io::{AsRawFd, RawFd};
#[cfg(windows)]
use std::os::windows::io::{AsRawHandle, RawHandle};
use unsafe_io::AsUnsafeFile;
use wasi_common::{
    file::{FdFlags, FileType, Filestat, WasiFile},
    Error, ErrorExt,
};

pub struct Stdin(std::io::Stdin);

pub fn stdin() -> Stdin {
    Stdin(std::io::stdin())
}

impl WasiFile for Stdin {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn datasync(&self) -> Result<(), Error> {
        Ok(())
    }
    fn sync(&self) -> Result<(), Error> {
        Ok(())
    }
    fn get_filetype(&self) -> Result<FileType, Error> {
        Ok(FileType::Unknown)
    }
    fn get_fdflags(&self) -> Result<FdFlags, Error> {
        Ok(FdFlags::empty())
    }
    fn set_fdflags(&mut self, _fdflags: FdFlags) -> Result<(), Error> {
        Err(Error::badf())
    }
    fn get_filestat(&self) -> Result<Filestat, Error> {
        let meta = self.0.as_file_view().metadata()?;
        Ok(Filestat {
            device_id: 0,
            inode: 0,
            filetype: self.get_filetype()?,
            nlink: 0,
            size: meta.len(),
            atim: meta.accessed().ok(),
            mtim: meta.modified().ok(),
            ctim: meta.created().ok(),
        })
    }
    fn set_filestat_size(&self, _size: u64) -> Result<(), Error> {
        Err(Error::badf())
    }
    fn advise(&self, _offset: u64, _len: u64, _advice: Advice) -> Result<(), Error> {
        Err(Error::badf())
    }
    fn allocate(&self, _offset: u64, _len: u64) -> Result<(), Error> {
        Err(Error::badf())
    }
    fn read_vectored(&self, bufs: &mut [io::IoSliceMut]) -> Result<u64, Error> {
        let n = self.0.as_file_view().read_vectored(bufs)?;
        Ok(n.try_into().map_err(|_| Error::range())?)
    }
    fn read_vectored_at(&self, _bufs: &mut [io::IoSliceMut], _offset: u64) -> Result<u64, Error> {
        Err(Error::seek_pipe())
    }
    fn write_vectored(&self, _bufs: &[io::IoSlice]) -> Result<u64, Error> {
        Err(Error::badf())
    }
    fn write_vectored_at(&self, _bufs: &[io::IoSlice], _offset: u64) -> Result<u64, Error> {
        Err(Error::badf())
    }
    fn seek(&self, _pos: std::io::SeekFrom) -> Result<u64, Error> {
        Err(Error::seek_pipe())
    }
    fn peek(&self, _buf: &mut [u8]) -> Result<u64, Error> {
        Err(Error::seek_pipe())
    }
    fn set_times(
        &self,
        atime: Option<wasi_common::SystemTimeSpec>,
        mtime: Option<wasi_common::SystemTimeSpec>,
    ) -> Result<(), Error> {
        self.0
            .set_times(convert_systimespec(atime), convert_systimespec(mtime))?;
        Ok(())
    }
    fn num_ready_bytes(&self) -> Result<u64, Error> {
        Ok(self.0.num_ready_bytes()?)
    }
}
#[cfg(windows)]
impl AsRawHandle for Stdin {
    fn as_raw_handle(&self) -> RawHandle {
        self.0.as_raw_handle()
    }
}
#[cfg(unix)]
impl AsRawFd for Stdin {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

macro_rules! wasi_file_write_impl {
    ($ty:ty) => {
        impl WasiFile for $ty {
            fn as_any(&self) -> &dyn Any {
                self
            }
            fn datasync(&self) -> Result<(), Error> {
                Ok(())
            }
            fn sync(&self) -> Result<(), Error> {
                Ok(())
            }
            fn get_filetype(&self) -> Result<FileType, Error> {
                Ok(FileType::Unknown)
            }
            fn get_fdflags(&self) -> Result<FdFlags, Error> {
                Ok(FdFlags::APPEND)
            }
            fn set_fdflags(&mut self, _fdflags: FdFlags) -> Result<(), Error> {
                Err(Error::badf())
            }
            fn get_filestat(&self) -> Result<Filestat, Error> {
                let meta = self.0.as_file_view().metadata()?;
                Ok(Filestat {
                    device_id: 0,
                    inode: 0,
                    filetype: self.get_filetype()?,
                    nlink: 0,
                    size: meta.len(),
                    atim: meta.accessed().ok(),
                    mtim: meta.modified().ok(),
                    ctim: meta.created().ok(),
                })
            }
            fn set_filestat_size(&self, _size: u64) -> Result<(), Error> {
                Err(Error::badf())
            }
            fn advise(&self, _offset: u64, _len: u64, _advice: Advice) -> Result<(), Error> {
                Err(Error::badf())
            }
            fn allocate(&self, _offset: u64, _len: u64) -> Result<(), Error> {
                Err(Error::badf())
            }
            fn read_vectored(&self, _bufs: &mut [io::IoSliceMut]) -> Result<u64, Error> {
                Err(Error::badf())
            }
            fn read_vectored_at(
                &self,
                _bufs: &mut [io::IoSliceMut],
                _offset: u64,
            ) -> Result<u64, Error> {
                Err(Error::badf())
            }
            fn write_vectored(&self, bufs: &[io::IoSlice]) -> Result<u64, Error> {
                let n = self.0.as_file_view().write_vectored(bufs)?;
                Ok(n.try_into().map_err(|c| Error::range().context(c))?)
            }
            fn write_vectored_at(&self, _bufs: &[io::IoSlice], _offset: u64) -> Result<u64, Error> {
                Err(Error::seek_pipe())
            }
            fn seek(&self, _pos: std::io::SeekFrom) -> Result<u64, Error> {
                Err(Error::seek_pipe())
            }
            fn peek(&self, _buf: &mut [u8]) -> Result<u64, Error> {
                Err(Error::badf())
            }
            fn set_times(
                &self,
                atime: Option<wasi_common::SystemTimeSpec>,
                mtime: Option<wasi_common::SystemTimeSpec>,
            ) -> Result<(), Error> {
                self.0
                    .set_times(convert_systimespec(atime), convert_systimespec(mtime))?;
                Ok(())
            }
            fn num_ready_bytes(&self) -> Result<u64, Error> {
                Ok(0)
            }
        }
        #[cfg(windows)]
        impl AsRawHandle for $ty {
            fn as_raw_handle(&self) -> RawHandle {
                self.0.as_raw_handle()
            }
        }
        #[cfg(unix)]
        impl AsRawFd for $ty {
            fn as_raw_fd(&self) -> RawFd {
                self.0.as_raw_fd()
            }
        }
    };
}

pub struct Stdout(std::io::Stdout);

pub fn stdout() -> Stdout {
    Stdout(std::io::stdout())
}
wasi_file_write_impl!(Stdout);

pub struct Stderr(std::io::Stderr);

pub fn stderr() -> Stderr {
    Stderr(std::io::stderr())
}
wasi_file_write_impl!(Stderr);
