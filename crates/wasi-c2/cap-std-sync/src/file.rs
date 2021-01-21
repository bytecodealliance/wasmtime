use cap_fs_ext::MetadataExt;
use fs_set_times::{SetTimes, SystemTimeSpec};
use std::any::Any;
use std::convert::TryInto;
use std::io;
use system_interface::fs::{Advice, FileIoExt};
use system_interface::io::ReadReady;
use wasi_c2::{
    file::{FdFlags, FileType, Filestat, WasiFile},
    Error,
};

pub struct File(cap_std::fs::File);

impl File {
    pub fn from_cap_std(file: cap_std::fs::File) -> Self {
        File(file)
    }
}

impl WasiFile for File {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn datasync(&self) -> Result<(), Error> {
        self.0.sync_data()?;
        Ok(())
    }
    fn sync(&self) -> Result<(), Error> {
        self.0.sync_all()?;
        Ok(())
    }
    fn get_filetype(&self) -> Result<FileType, Error> {
        let meta = self.0.metadata()?;
        Ok(filetype_from(&meta.file_type()))
    }
    fn get_fdflags(&self) -> Result<FdFlags, Error> {
        // XXX get_fdflags is not implemented but lets lie rather than panic:
        Ok(FdFlags::empty())
    }
    fn set_fdflags(&self, _fdflags: FdFlags) -> Result<(), Error> {
        todo!("set_fdflags is not implemented")
    }
    fn get_filestat(&self) -> Result<Filestat, Error> {
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
    fn set_filestat_size(&self, size: u64) -> Result<(), Error> {
        self.0.set_len(size)?;
        Ok(())
    }
    fn advise(&self, offset: u64, len: u64, advice: Advice) -> Result<(), Error> {
        self.0.advise(offset, len, advice)?;
        Ok(())
    }
    fn allocate(&self, offset: u64, len: u64) -> Result<(), Error> {
        self.0.allocate(offset, len)?;
        Ok(())
    }
    fn set_times(
        &self,
        atime: Option<wasi_c2::SystemTimeSpec>,
        mtime: Option<wasi_c2::SystemTimeSpec>,
    ) -> Result<(), Error> {
        self.0
            .set_times(convert_systimespec(atime), convert_systimespec(mtime))?;
        Ok(())
    }
    fn read_vectored(&self, bufs: &mut [io::IoSliceMut]) -> Result<u64, Error> {
        let n = self.0.read_vectored(bufs)?;
        Ok(n.try_into().map_err(|_| Error::Overflow)?)
    }
    fn read_vectored_at(&self, bufs: &mut [io::IoSliceMut], offset: u64) -> Result<u64, Error> {
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
    fn peek(&self, buf: &mut [u8]) -> Result<u64, Error> {
        let n = self.0.peek(buf)?;
        Ok(n.try_into().map_err(|_| Error::Overflow)?)
    }
    fn num_ready_bytes(&self) -> Result<u64, Error> {
        Ok(self.0.num_ready_bytes()?)
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
use std::os::windows::io::{AsRawHandle, RawHandle};
#[cfg(windows)]
impl AsRawHandle for File {
    fn as_raw_handle(&self) -> RawHandle {
        self.0.as_raw_handle()
    }
}

#[cfg(unix)]
use std::os::unix::io::{AsRawFd, RawFd};
#[cfg(unix)]
impl AsRawFd for File {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}
pub fn convert_systimespec(t: Option<wasi_c2::SystemTimeSpec>) -> Option<SystemTimeSpec> {
    match t {
        Some(wasi_c2::SystemTimeSpec::Absolute(t)) => Some(SystemTimeSpec::Absolute(t.into_std())),
        Some(wasi_c2::SystemTimeSpec::SymbolicNow) => Some(SystemTimeSpec::SymbolicNow),
        None => None,
    }
}
