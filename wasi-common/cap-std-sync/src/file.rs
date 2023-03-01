use cap_fs_ext::MetadataExt;
use fs_set_times::{SetTimes, SystemTimeSpec};
use io_lifetimes::AsFilelike;
use is_terminal::IsTerminal;
use std::any::Any;
use std::convert::TryInto;
use std::io;
use std::sync::Arc;
use system_interface::fs::{FileIoExt, GetSetFdFlags};
use system_interface::io::IsReadWrite;
use wasi_common::{
    file::{Advice, FdFlags, FileType, Filestat, WasiFile},
    Error, ErrorExt,
};
#[cfg(windows)]
use windows_sys::Win32::Foundation::ERROR_ACCESS_DENIED;

/// A file handle.
///
/// We hold an `Arc` so that stream views can be regular handles which can
/// be closed, without closing the underlying file descriptor.
pub struct File(Arc<cap_std::fs::File>);

impl File {
    pub fn from_cap_std(file: cap_std::fs::File) -> Self {
        File(Arc::new(file))
    }
}

#[async_trait::async_trait]
impl WasiFile for File {
    fn as_any(&self) -> &dyn Any {
        self
    }
    #[cfg(unix)]
    fn pollable(&self) -> Option<rustix::fd::BorrowedFd> {
        Some(self.0.as_fd())
    }

    #[cfg(windows)]
    fn pollable(&self) -> Option<io_extras::os::windows::BorrowedHandleOrSocket> {
        Some(self.0.as_handle_or_socket())
    }

    async fn datasync(&self) -> Result<(), Error> {
        match self.0.sync_data() {
            Ok(()) => Ok(()),

            // On Windows, `sync_data` uses `FlushFileBuffers` which fails
            // with `ERROR_ACCESS_DENIED` if the file is not open for
            // writing. Ignore this error, for POSIX compatibility.
            #[cfg(windows)]
            Err(e) if e.raw_os_error() == Some(ERROR_ACCESS_DENIED as _) => Ok(()),

            Err(e) => Err(e.into()),
        }
    }

    async fn sync(&self) -> Result<(), Error> {
        match self.0.sync_all() {
            Ok(()) => Ok(()),

            // On Windows, `sync_all` uses `FlushFileBuffers` which fails
            // with `ERROR_ACCESS_DENIED` if the file is not open for
            // writing. Ignore this error, for POSIX compatibility.
            #[cfg(windows)]
            Err(e) if e.raw_os_error() == Some(ERROR_ACCESS_DENIED as _) => Ok(()),

            Err(e) => Err(e.into()),
        }
    }
    async fn get_filetype(&self) -> Result<FileType, Error> {
        let meta = self.0.metadata()?;
        Ok(filetype_from(&meta.file_type()))
    }
    async fn get_fdflags(&self) -> Result<FdFlags, Error> {
        let fdflags = get_fd_flags(&*self.0)?;
        Ok(fdflags)
    }
    async fn get_filestat(&self) -> Result<Filestat, Error> {
        let meta = self.0.metadata()?;
        Ok(Filestat {
            device_id: meta.dev(),
            inode: meta.ino(),
            filetype: filetype_from(&meta.file_type()),
            nlink: meta.nlink(),
            size: meta.len(),
            atim: meta.accessed().map(|t| Some(t.into_std())).unwrap_or(None),
            mtim: meta.modified().map(|t| Some(t.into_std())).unwrap_or(None),
            ctim: meta.created().map(|t| Some(t.into_std())).unwrap_or(None),
        })
    }
    async fn set_filestat_size(&mut self, size: u64) -> Result<(), Error> {
        self.0.set_len(size)?;
        Ok(())
    }
    async fn advise(&mut self, offset: u64, len: u64, advice: Advice) -> Result<(), Error> {
        self.0.advise(offset, len, convert_advice(advice))?;
        Ok(())
    }
    async fn allocate(&mut self, offset: u64, len: u64) -> Result<(), Error> {
        self.0.allocate(offset, len)?;
        Ok(())
    }
    async fn set_times(
        &mut self,
        atime: Option<wasi_common::SystemTimeSpec>,
        mtime: Option<wasi_common::SystemTimeSpec>,
    ) -> Result<(), Error> {
        self.0
            .set_times(convert_systimespec(atime), convert_systimespec(mtime))?;
        Ok(())
    }
    async fn read_at<'a>(&mut self, buf: &mut [u8], offset: u64) -> Result<(u64, bool), Error> {
        match self.0.read_at(buf, offset) {
            Ok(0) => Ok((0, true)),
            Ok(n) => Ok((n as u64, false)),
            Err(err) if err.kind() == io::ErrorKind::Interrupted => Ok((0, false)),
            Err(err) => Err(err.into()),
        }
    }
    async fn read_vectored_at<'a>(
        &mut self,
        bufs: &mut [io::IoSliceMut<'a>],
        offset: u64,
    ) -> Result<(u64, bool), Error> {
        match self.0.read_vectored_at(bufs, offset) {
            Ok(0) => Ok((0, true)),
            Ok(n) => Ok((n as u64, false)),
            Err(err) if err.kind() == io::ErrorKind::Interrupted => Ok((0, false)),
            Err(err) => Err(err.into()),
        }
    }
    fn is_read_vectored_at(&self) -> bool {
        self.0.is_read_vectored_at()
    }
    async fn write_at<'a>(&mut self, buf: &[u8], offset: u64) -> Result<u64, Error> {
        let n = self.0.write_at(buf, offset)?;
        Ok(n.try_into()?)
    }
    async fn write_vectored_at<'a>(
        &mut self,
        bufs: &[io::IoSlice<'a>],
        offset: u64,
    ) -> Result<u64, Error> {
        let n = self.0.write_vectored_at(bufs, offset)?;
        Ok(n.try_into()?)
    }
    fn is_write_vectored_at(&self) -> bool {
        self.0.is_write_vectored_at()
    }
    async fn append<'a>(&mut self, buf: &[u8]) -> Result<u64, Error> {
        let n = self.0.append(buf)?;
        Ok(n.try_into()?)
    }
    async fn append_vectored<'a>(&mut self, bufs: &[io::IoSlice<'a>]) -> Result<u64, Error> {
        let n = self.0.append_vectored(bufs)?;
        Ok(n.try_into()?)
    }
    fn is_append_vectored(&self) -> bool {
        self.0.is_append_vectored()
    }
    fn isatty(&mut self) -> bool {
        self.0.is_terminal()
    }

    async fn readable(&self) -> Result<(), Error> {
        if is_read_write(&*self.0)?.0 {
            Ok(())
        } else {
            Err(Error::badf())
        }
    }

    async fn writable(&self) -> Result<(), Error> {
        if is_read_write(&*self.0)?.1 {
            Ok(())
        } else {
            Err(Error::badf())
        }
    }

    fn dup(&self) -> Box<dyn WasiFile> {
        Box::new(File(Arc::clone(&self.0)))
    }
}

pub fn filetype_from(ft: &cap_std::fs::FileType) -> FileType {
    use cap_fs_ext::FileTypeExt;
    if ft.is_dir() {
        FileType::Directory
    } else if ft.is_symlink() {
        FileType::SymbolicLink
    } else if ft.is_socket() {
        if ft.is_block_device() {
            FileType::SocketDgram
        } else {
            FileType::SocketStream
        }
    } else if ft.is_block_device() {
        FileType::BlockDevice
    } else if ft.is_char_device() {
        FileType::CharacterDevice
    } else if ft.is_file() {
        FileType::RegularFile
    } else {
        FileType::Unknown
    }
}

#[cfg(windows)]
use io_lifetimes::{AsHandle, BorrowedHandle};
#[cfg(windows)]
impl AsHandle for File {
    fn as_handle(&self) -> BorrowedHandle<'_> {
        self.0.as_handle()
    }
}

#[cfg(windows)]
use io_extras::os::windows::{AsHandleOrSocket, BorrowedHandleOrSocket};
#[cfg(windows)]
impl AsHandleOrSocket for File {
    #[inline]
    fn as_handle_or_socket(&self) -> BorrowedHandleOrSocket {
        self.0.as_handle_or_socket()
    }
}

#[cfg(unix)]
use io_lifetimes::{AsFd, BorrowedFd};

#[cfg(unix)]
impl AsFd for File {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

pub(crate) fn convert_systimespec(
    t: Option<wasi_common::SystemTimeSpec>,
) -> Option<SystemTimeSpec> {
    match t {
        Some(wasi_common::SystemTimeSpec::Absolute(t)) => {
            Some(SystemTimeSpec::Absolute(t.into_std()))
        }
        Some(wasi_common::SystemTimeSpec::SymbolicNow) => Some(SystemTimeSpec::SymbolicNow),
        None => None,
    }
}

/// Return the file-descriptor flags for a given file-like object.
///
/// This returns the flags needed to implement [`WasiFile::get_fdflags`].
pub fn get_fd_flags<Filelike: AsFilelike>(f: Filelike) -> io::Result<wasi_common::file::FdFlags> {
    let f = f.as_filelike().get_fd_flags()?;
    let mut out = wasi_common::file::FdFlags::empty();
    if f.contains(system_interface::fs::FdFlags::APPEND) {
        out |= wasi_common::file::FdFlags::APPEND;
    }
    if f.contains(system_interface::fs::FdFlags::DSYNC) {
        out |= wasi_common::file::FdFlags::DSYNC;
    }
    if f.contains(system_interface::fs::FdFlags::NONBLOCK) {
        out |= wasi_common::file::FdFlags::NONBLOCK;
    }
    if f.contains(system_interface::fs::FdFlags::RSYNC) {
        out |= wasi_common::file::FdFlags::RSYNC;
    }
    if f.contains(system_interface::fs::FdFlags::SYNC) {
        out |= wasi_common::file::FdFlags::SYNC;
    }
    Ok(out)
}

/// Return the file-descriptor flags for a given file-like object.
///
/// This returns the flags needed to implement [`WasiFile::get_fdflags`].
pub fn is_read_write<T: IsReadWrite>(t: &T) -> io::Result<(bool, bool)> {
    t.is_read_write()
}

fn convert_advice(advice: Advice) -> system_interface::fs::Advice {
    match advice {
        Advice::Normal => system_interface::fs::Advice::Normal,
        Advice::Sequential => system_interface::fs::Advice::Sequential,
        Advice::Random => system_interface::fs::Advice::Random,
        Advice::WillNeed => system_interface::fs::Advice::WillNeed,
        Advice::DontNeed => system_interface::fs::Advice::DontNeed,
        Advice::NoReuse => system_interface::fs::Advice::NoReuse,
    }
}
