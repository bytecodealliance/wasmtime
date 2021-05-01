use crate::asyncify;
use cap_fs_ext::MetadataExt;
use fs_set_times::{SetTimes, SystemTimeSpec};
use std::any::Any;
use std::convert::TryInto;
use std::io;
use system_interface::fs::{FileIoExt, GetSetFdFlags};
use system_interface::io::ReadReady;
use wasi_common::{
    file::{Advice, FdFlags, FileType, Filestat, WasiFile},
    Error, ErrorExt,
};

mod internal {
    use std::sync::{Mutex, MutexGuard};
    #[cfg(not(windows))]
    use unsafe_io::os::posish::{AsRawFd, RawFd};
    #[cfg(windows)]
    use unsafe_io::os::windows::{AsRawHandleOrSocket, RawHandleOrSocket};
    use unsafe_io::OwnsRaw;

    // This internal type wraps tokio's File so that we can impl the
    // `AsUnsafeFile` trait. We impl this on an internal type, rather than on
    // super::File, because we don't want consumers of this library to be able
    // to use our `AsUnsafeFile`.
    // Mutex is required because this type requires internal mutation for the
    // tokio AsyncWriteExt methods to work, and must be Send.
    pub(super) struct Internal(Mutex<tokio::fs::File>);
    impl Internal {
        pub fn new(f: tokio::fs::File) -> Self {
            Internal(Mutex::new(f))
        }
        pub fn inner(&self) -> MutexGuard<tokio::fs::File> {
            self.0.lock().unwrap()
        }
    }

    #[cfg(not(windows))]
    impl AsRawFd for Internal {
        fn as_raw_fd(&self) -> RawFd {
            self.inner().as_raw_fd()
        }
    }

    #[cfg(windows)]
    impl AsRawHandleOrSocket for Internal {
        fn as_raw_handle_or_socket(&self) -> RawHandleOrSocket {
            self.inner().as_raw_handle_or_socket()
        }
    }

    // Safety: `Internal` owns its handle.
    unsafe impl OwnsRaw for Internal {}
}

pub struct File(internal::Internal);

impl File {
    pub fn from_cap_std(file: cap_std::fs::File) -> Self {
        File(internal::Internal::new(tokio::fs::File::from_std(
            file.into_std(),
        )))
    }

    async fn metadata(&self) -> Result<cap_std::fs::Metadata, Error> {
        use unsafe_io::AsUnsafeFile;
        asyncify(|| Ok(cap_std::fs::Metadata::from_file(&self.0.as_file_view())?))
    }
}

#[wiggle::async_trait]
impl WasiFile for File {
    fn as_any(&self) -> &dyn Any {
        self
    }
    async fn datasync(&self) -> Result<(), Error> {
        self.0.inner().sync_data().await?;
        Ok(())
    }
    async fn sync(&self) -> Result<(), Error> {
        self.0.inner().sync_all().await?;
        Ok(())
    }
    async fn get_filetype(&self) -> Result<FileType, Error> {
        let meta = self.metadata().await?;
        Ok(filetype_from(&meta.file_type()))
    }
    async fn get_fdflags(&self) -> Result<FdFlags, Error> {
        let fdflags = asyncify(|| self.0.get_fd_flags())?;
        Ok(from_sysif_fdflags(fdflags))
    }
    async fn set_fdflags(&mut self, fdflags: FdFlags) -> Result<(), Error> {
        if fdflags.intersects(
            wasi_common::file::FdFlags::DSYNC
                | wasi_common::file::FdFlags::SYNC
                | wasi_common::file::FdFlags::RSYNC,
        ) {
            return Err(Error::invalid_argument().context("cannot set DSYNC, SYNC, or RSYNC flag"));
        }
        asyncify(move || self.0.set_fd_flags(to_sysif_fdflags(fdflags)))?;
        Ok(())
    }
    async fn get_filestat(&self) -> Result<Filestat, Error> {
        let meta = self.metadata().await?;
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
        self.0.inner().set_len(size).await?;
        Ok(())
    }
    async fn advise(&self, offset: u64, len: u64, advice: Advice) -> Result<(), Error> {
        asyncify(move || self.0.advise(offset, len, convert_advice(advice)))?;
        Ok(())
    }
    async fn allocate(&self, offset: u64, len: u64) -> Result<(), Error> {
        asyncify(move || self.0.allocate(offset, len))?;
        Ok(())
    }
    async fn set_times(
        &self,
        atime: Option<wasi_common::SystemTimeSpec>,
        mtime: Option<wasi_common::SystemTimeSpec>,
    ) -> Result<(), Error> {
        asyncify(|| {
            self.0
                .set_times(convert_systimespec(atime), convert_systimespec(mtime))
        })?;
        Ok(())
    }
    async fn read_vectored<'a>(&self, bufs: &mut [io::IoSliceMut<'a>]) -> Result<u64, Error> {
        use std::ops::DerefMut;
        use tokio::io::AsyncReadExt;
        let mut nbytes: usize = 0;
        for b in bufs.iter_mut() {
            let n = self.0.inner().read(b.deref_mut()).await?;
            nbytes += n;
            if n < b.len() {
                break;
            }
        }
        Ok(nbytes.try_into()?)
    }
    async fn read_vectored_at<'a>(
        &self,
        bufs: &mut [io::IoSliceMut<'a>],
        offset: u64,
    ) -> Result<u64, Error> {
        let n = asyncify(move || self.0.read_vectored_at(bufs, offset))?;
        Ok(n.try_into()?)
    }
    async fn write_vectored<'a>(&self, bufs: &[io::IoSlice<'a>]) -> Result<u64, Error> {
        use tokio::io::AsyncWriteExt;
        let mut n: usize = 0;
        n += self.0.inner().write_vectored(bufs).await?;
        Ok(n.try_into()?)
    }
    async fn write_vectored_at<'a>(
        &self,
        bufs: &[io::IoSlice<'a>],
        offset: u64,
    ) -> Result<u64, Error> {
        let n = asyncify(move || self.0.write_vectored_at(bufs, offset))?;
        Ok(n.try_into()?)
    }
    async fn seek(&self, pos: std::io::SeekFrom) -> Result<u64, Error> {
        use tokio::io::AsyncSeekExt;
        Ok(self.0.inner().seek(pos).await?)
    }
    async fn peek(&self, buf: &mut [u8]) -> Result<u64, Error> {
        let n = asyncify(move || self.0.peek(buf))?;
        Ok(n.try_into()?)
    }
    async fn num_ready_bytes(&self) -> Result<u64, Error> {
        use unsafe_io::AsUnsafeFile;
        asyncify(|| self.0.as_file_view().num_ready_bytes())
    }
    #[cfg(not(windows))]
    async fn readable(&mut self) -> Result<(), Error> {
        // The Inner impls OwnsRaw, which asserts exclusive use of the handle by the owned object.
        // AsyncFd needs to wrap an owned `impl std::os::unix::io::AsRawFd`. Rather than introduce
        // mutability to let it own the `Inner`, we are depending on the `&mut self` bound on this
        // async method to ensure this is the only Future which can access the RawFd during the
        // lifetime of the AsyncFd.
        use tokio::io::{unix::AsyncFd, Interest};
        use unsafe_io::os::posish::AsRawFd;
        let rawfd = self.0.as_raw_fd();
        match AsyncFd::with_interest(rawfd, Interest::READABLE) {
            Ok(asyncfd) => {
                let _ = asyncfd.readable().await?;
                Ok(())
            }
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                // if e is EPERM, this file isnt supported by epoll because it is immediately
                // available for reading:
                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }
    #[cfg(windows)]
    async fn readable(&mut self) -> Result<(), Error> {
        // Windows uses a rawfd based scheduler :(
        Err(Error::badf())
    }

    #[cfg(not(windows))]
    async fn writable(&mut self) -> Result<(), Error> {
        // The Inner impls OwnsRaw, which asserts exclusive use of the handle by the owned object.
        // AsyncFd needs to wrap an owned `impl std::os::unix::io::AsRawFd`. Rather than introduce
        // mutability to let it own the `Inner`, we are depending on the `&mut self` bound on this
        // async method to ensure this is the only Future which can access the RawFd during the
        // lifetime of the AsyncFd.
        use tokio::io::{unix::AsyncFd, Interest};
        use unsafe_io::os::posish::AsRawFd;
        let rawfd = self.0.as_raw_fd();
        match AsyncFd::with_interest(rawfd, Interest::WRITABLE) {
            Ok(asyncfd) => {
                let _ = asyncfd.writable().await?;
                Ok(())
            }
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                // if e is EPERM, this file isnt supported by epoll because it is immediately
                // available for writing:
                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }
    #[cfg(windows)]
    async fn writable(&mut self) -> Result<(), Error> {
        // Windows uses a rawfd based scheduler :(
        Err(Error::badf())
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

pub fn convert_systimespec(t: Option<wasi_common::SystemTimeSpec>) -> Option<SystemTimeSpec> {
    match t {
        Some(wasi_common::SystemTimeSpec::Absolute(t)) => {
            Some(SystemTimeSpec::Absolute(t.into_std()))
        }
        Some(wasi_common::SystemTimeSpec::SymbolicNow) => Some(SystemTimeSpec::SymbolicNow),
        None => None,
    }
}

pub fn to_sysif_fdflags(f: wasi_common::file::FdFlags) -> system_interface::fs::FdFlags {
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
pub fn from_sysif_fdflags(f: system_interface::fs::FdFlags) -> wasi_common::file::FdFlags {
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
    out
}
pub fn convert_advice(advice: Advice) -> system_interface::fs::Advice {
    match advice {
        Advice::Normal => system_interface::fs::Advice::Normal,
        Advice::Sequential => system_interface::fs::Advice::Sequential,
        Advice::Random => system_interface::fs::Advice::Random,
        Advice::WillNeed => system_interface::fs::Advice::WillNeed,
        Advice::DontNeed => system_interface::fs::Advice::DontNeed,
        Advice::NoReuse => system_interface::fs::Advice::NoReuse,
    }
}
