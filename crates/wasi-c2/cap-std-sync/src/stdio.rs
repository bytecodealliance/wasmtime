use fs_set_times::SetTimes;
use std::any::Any;
use std::convert::TryInto;
use std::io;
use system_interface::{
    fs::{Advice, FileIoExt},
    io::ReadReady,
};
use wasi_c2::{
    file::{FdFlags, FileType, Filestat, WasiFile},
    Error,
};

#[cfg(unix)]
use std::os::unix::io::{AsRawFd, RawFd};
#[cfg(windows)]
use std::os::windows::io::{AsRawHandle, RawHandle};

macro_rules! wasi_file_impl {
    ($ty:ty, $additional:item) => {
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
                Ok(FileType::CharacterDevice) // XXX wrong
            }
            fn get_fdflags(&self) -> Result<FdFlags, Error> {
                // XXX get_fdflags is not implemented but lets lie rather than panic:
                Ok(FdFlags::empty())
            }
            fn set_fdflags(&self, _fdflags: FdFlags) -> Result<(), Error> {
                // XXX
                Err(Error::Perm)
            }
            fn get_filestat(&self) -> Result<Filestat, Error> {
                // XXX can unsafe-io give a way to get metadata?
                Ok(Filestat {
                    device_id: 0,
                    inode: 0,
                    filetype: self.get_filetype()?,
                    nlink: 0,
                    size: 0,
                    atim: None,
                    mtim: None,
                    ctim: None,
                })
            }
            fn set_filestat_size(&self, _size: u64) -> Result<(), Error> {
                // XXX is this the right error?
                Err(Error::Perm)
            }
            fn advise(&self, offset: u64, len: u64, advice: Advice) -> Result<(), Error> {
                self.0.advise(offset, len, advice)?;
                Ok(())
            }
            fn allocate(&self, offset: u64, len: u64) -> Result<(), Error> {
                self.0.allocate(offset, len)?;
                Ok(())
            }
            fn read_vectored(&self, bufs: &mut [io::IoSliceMut]) -> Result<u64, Error> {
                let n = self.0.read_vectored(bufs)?;
                Ok(n.try_into().map_err(|_| Error::Overflow)?)
            }
            fn read_vectored_at(
                &self,
                bufs: &mut [io::IoSliceMut],
                offset: u64,
            ) -> Result<u64, Error> {
                let n = self.0.read_vectored_at(bufs, offset)?;
                Ok(n.try_into().map_err(|_| Error::Overflow)?)
            }
            fn write_vectored(&self, bufs: &[io::IoSlice]) -> Result<u64, Error> {
                let n = self.0.write_vectored(bufs)?;
                Ok(n.try_into().map_err(|_| Error::Overflow)?)
            }
            fn write_vectored_at(&self, bufs: &[io::IoSlice], offset: u64) -> Result<u64, Error> {
                let n = self.0.write_vectored_at(bufs, offset)?;
                Ok(n.try_into().map_err(|_| Error::Overflow)?)
            }
            fn seek(&self, pos: std::io::SeekFrom) -> Result<u64, Error> {
                Ok(self.0.seek(pos)?)
            }
            fn stream_position(&self) -> Result<u64, Error> {
                Ok(self.0.stream_position()?)
            }
            fn peek(&self, buf: &mut [u8]) -> Result<u64, Error> {
                let n = self.0.peek(buf)?;
                Ok(n.try_into().map_err(|_| Error::Overflow)?)
            }
            fn set_times(
                &self,
                atime: Option<fs_set_times::SystemTimeSpec>,
                mtime: Option<fs_set_times::SystemTimeSpec>,
            ) -> Result<(), Error> {
                self.0.set_times(atime, mtime)?;
                Ok(())
            }
            $additional
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

pub struct Stdin(std::io::Stdin);

pub fn stdin() -> Stdin {
    Stdin(std::io::stdin())
}
wasi_file_impl!(
    Stdin,
    fn num_ready_bytes(&self) -> Result<u64, Error> {
        Ok(self.0.num_ready_bytes()?)
    }
);

pub struct Stdout(std::io::Stdout);

pub fn stdout() -> Stdout {
    Stdout(std::io::stdout())
}
wasi_file_impl!(
    Stdout,
    fn num_ready_bytes(&self) -> Result<u64, Error> {
        Ok(0)
    }
);

pub struct Stderr(std::io::Stderr);

pub fn stderr() -> Stderr {
    Stderr(std::io::stderr())
}
wasi_file_impl!(
    Stderr,
    fn num_ready_bytes(&self) -> Result<u64, Error> {
        Ok(0)
    }
);
