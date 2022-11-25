use cap_fs_ext::MetadataExt;
use fs_set_times::{SetTimes, SystemTimeSpec};
use io_lifetimes::AsFilelike;
use is_terminal::IsTerminal;
use std::any::Any;
use std::convert::TryInto;
use std::io;
use std::sync::{Arc, RwLock, RwLockReadGuard};
use system_interface::{
    fs::{FileIoExt, GetSetFdFlags},
    io::{IoExt, ReadReady},
};
use wasi_common::{
    file::{Advice, FdFlags, FileType, Filestat, WasiFile},
    Error, ErrorExt,
};

#[cfg(unix)]
use io_lifetimes::{AsFd, BorrowedFd};

#[cfg(windows)]
use io_lifetimes::{AsHandle, BorrowedHandle};

#[cfg(windows)]
use io_extras::os::windows::{AsRawHandleOrSocket, RawHandleOrSocket};

pub struct BorrowedFile<'a>(RwLockReadGuard<'a, cap_std::fs::File>);

#[cfg(unix)]
impl AsFd for BorrowedFile<'_> {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

#[cfg(windows)]
impl AsHandle for BorrowedFile<'_> {
    fn as_handle(&self) -> BorrowedHandle<'_> {
        self.0.as_handle()
    }
}

#[cfg(windows)]
impl AsRawHandleOrSocket for BorrowedFile<'_> {
    #[inline]
    fn as_raw_handle_or_socket(&self) -> RawHandleOrSocket {
        self.0.as_raw_handle_or_socket()
    }
}

pub struct File(RwLock<cap_std::fs::File>);

impl File {
    pub fn from_cap_std(file: cap_std::fs::File) -> Self {
        File(RwLock::new(file))
    }

    pub fn borrow(&self) -> BorrowedFile {
        BorrowedFile(self.0.read().unwrap())
    }
}

#[async_trait::async_trait]
impl WasiFile for File {
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[cfg(unix)]
    fn pollable(&self) -> Option<Arc<dyn AsFd + '_>> {
        Some(Arc::new(self.borrow()))
    }

    #[cfg(windows)]
    fn pollable(&self) -> Option<Arc<dyn AsRawHandleOrSocket + '_>> {
        Some(Arc::new(BorrowedFile(self.0.read().unwrap())))
    }

    async fn datasync(&self) -> Result<(), Error> {
        self.0.read().unwrap().sync_data()?;
        Ok(())
    }
    async fn sync(&self) -> Result<(), Error> {
        self.0.read().unwrap().sync_all()?;
        Ok(())
    }
    async fn get_filetype(&self) -> Result<FileType, Error> {
        let meta = self.0.read().unwrap().metadata()?;
        Ok(filetype_from(&meta.file_type()))
    }
    async fn get_fdflags(&self) -> Result<FdFlags, Error> {
        let file = self.0.read().unwrap();
        let fdflags = get_fd_flags(&*file)?;
        Ok(fdflags)
    }
    async fn set_fdflags(&self, fdflags: FdFlags) -> Result<(), Error> {
        if fdflags.intersects(
            wasi_common::file::FdFlags::DSYNC
                | wasi_common::file::FdFlags::SYNC
                | wasi_common::file::FdFlags::RSYNC,
        ) {
            return Err(Error::invalid_argument().context("cannot set DSYNC, SYNC, or RSYNC flag"));
        }
        let mut file = self.0.write().unwrap();
        let set_fd_flags = (*file).new_set_fd_flags(to_sysif_fdflags(fdflags))?;
        (*file).set_fd_flags(set_fd_flags)?;
        Ok(())
    }
    async fn get_filestat(&self) -> Result<Filestat, Error> {
        let meta = self.0.read().unwrap().metadata()?;
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
    async fn set_filestat_size(&self, size: u64) -> Result<(), Error> {
        self.0.read().unwrap().set_len(size)?;
        Ok(())
    }
    async fn advise(&self, offset: u64, len: u64, advice: Advice) -> Result<(), Error> {
        self.0
            .read()
            .unwrap()
            .advise(offset, len, convert_advice(advice))?;
        Ok(())
    }
    async fn allocate(&self, offset: u64, len: u64) -> Result<(), Error> {
        self.0.read().unwrap().allocate(offset, len)?;
        Ok(())
    }
    async fn set_times(
        &self,
        atime: Option<wasi_common::SystemTimeSpec>,
        mtime: Option<wasi_common::SystemTimeSpec>,
    ) -> Result<(), Error> {
        self.0
            .read()
            .unwrap()
            .set_times(convert_systimespec(atime), convert_systimespec(mtime))?;
        Ok(())
    }
    async fn read_vectored<'a>(&self, bufs: &mut [io::IoSliceMut<'a>]) -> Result<u64, Error> {
        let n = self.0.read().unwrap().read_vectored(bufs)?;
        Ok(n.try_into()?)
    }
    async fn read_vectored_at<'a>(
        &self,
        bufs: &mut [io::IoSliceMut<'a>],
        offset: u64,
    ) -> Result<u64, Error> {
        let n = self.0.read().unwrap().read_vectored_at(bufs, offset)?;
        Ok(n.try_into()?)
    }
    async fn write_vectored<'a>(&self, bufs: &[io::IoSlice<'a>]) -> Result<u64, Error> {
        let n = self.0.read().unwrap().write_vectored(bufs)?;
        Ok(n.try_into()?)
    }
    async fn write_vectored_at<'a>(
        &self,
        bufs: &[io::IoSlice<'a>],
        offset: u64,
    ) -> Result<u64, Error> {
        let n = self.0.read().unwrap().write_vectored_at(bufs, offset)?;
        Ok(n.try_into()?)
    }
    async fn seek(&self, pos: std::io::SeekFrom) -> Result<u64, Error> {
        Ok(self.0.read().unwrap().seek(pos)?)
    }
    async fn peek(&self, buf: &mut [u8]) -> Result<u64, Error> {
        let n = self.0.read().unwrap().peek(buf)?;
        Ok(n.try_into()?)
    }
    fn num_ready_bytes(&self) -> Result<u64, Error> {
        Ok(self.0.read().unwrap().num_ready_bytes()?)
    }
    fn isatty(&self) -> bool {
        self.0.read().unwrap().is_terminal()
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
