use cap_fs_ext::MetadataExt;
use fs_set_times::{SetTimes, SystemTimeSpec};
use io_lifetimes::AsFilelike;
use is_terminal::IsTerminal;
use std::any::Any;
use std::convert::TryInto;
use std::io;
use system_interface::fs::{FileIoExt, GetSetFdFlags};
use system_interface::io::IsReadWrite;
use wasi_common::{
    file::{Advice, FdFlags, FileType, Filestat, WasiFile},
    Error, ErrorExt,
};

pub struct File(cap_std::fs::File);

impl File {
    pub fn from_cap_std(file: cap_std::fs::File) -> Self {
        File(file)
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
    fn pollable(&self) -> Option<io_extras::os::windows::RawHandleOrSocket> {
        Some(self.0.as_raw_handle_or_socket())
    }

    async fn try_clone(&mut self) -> Result<Box<dyn WasiFile>, Error> {
        let clone = self.0.try_clone()?;
        Ok(Box::new(Self(clone)))
    }

    async fn datasync(&mut self) -> Result<(), Error> {
        self.0.sync_data()?;
        Ok(())
    }
    async fn sync(&mut self) -> Result<(), Error> {
        self.0.sync_all()?;
        Ok(())
    }
    async fn get_filetype(&self) -> Result<FileType, Error> {
        let meta = self.0.metadata()?;
        Ok(filetype_from(&meta.file_type()))
    }
    async fn get_fdflags(&self) -> Result<FdFlags, Error> {
        let fdflags = get_fd_flags(&self.0)?;
        Ok(fdflags)
    }
    async fn set_fdflags(&mut self, fdflags: FdFlags) -> Result<(), Error> {
        if fdflags.intersects(
            wasi_common::file::FdFlags::DSYNC
                | wasi_common::file::FdFlags::SYNC
                | wasi_common::file::FdFlags::RSYNC,
        ) {
            return Err(Error::invalid_argument().context("cannot set DSYNC, SYNC, or RSYNC flag"));
        }
        let set_fd_flags = self.0.new_set_fd_flags(to_sysif_fdflags(fdflags))?;
        self.0.set_fd_flags(set_fd_flags)?;
        Ok(())
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
    fn isatty(&mut self) -> bool {
        self.0.is_terminal()
    }

    async fn readable(&self) -> Result<(), Error> {
        if is_read_write(&self.0)?.0 {
            Ok(())
        } else {
            Err(Error::badf())
        }
    }

    async fn writable(&self) -> Result<(), Error> {
        if is_read_write(&self.0)?.1 {
            Ok(())
        } else {
            Err(Error::badf())
        }
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
use io_extras::os::windows::{AsRawHandleOrSocket, RawHandleOrSocket};
#[cfg(windows)]
impl AsRawHandleOrSocket for File {
    #[inline]
    fn as_raw_handle_or_socket(&self) -> RawHandleOrSocket {
        self.0.as_raw_handle_or_socket()
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

pub(crate) fn to_sysif_fdflags(f: wasi_common::file::FdFlags) -> system_interface::fs::FdFlags {
    let mut out = system_interface::fs::FdFlags::empty();
    if f.contains(wasi_common::file::FdFlags::APPEND) {
        out |= system_interface::fs::FdFlags::APPEND;
    }
    if f.contains(wasi_common::file::FdFlags::DSYNC) {
        out |= system_interface::fs::FdFlags::DSYNC;
    }
    if f.contains(wasi_common::file::FdFlags::NONBLOCK) {
        out |= system_interface::fs::FdFlags::NONBLOCK;
    }
    if f.contains(wasi_common::file::FdFlags::RSYNC) {
        out |= system_interface::fs::FdFlags::RSYNC;
    }
    if f.contains(wasi_common::file::FdFlags::SYNC) {
        out |= system_interface::fs::FdFlags::SYNC;
    }
    out
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
pub fn is_read_write<Filelike: AsFilelike>(f: Filelike) -> io::Result<(bool, bool)> {
    f.is_read_write()
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
