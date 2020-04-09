use super::{fd, path};
use crate::entry::EntryRights;
use crate::handle::Handle;
use crate::sandboxed_tty_writer::SandboxedTTYWriter;
use crate::wasi::{types, Errno, Result};
use log::{debug, error};
use std::any::Any;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::mem::ManuallyDrop;

pub(crate) use super::sys_impl::oshandle::*;

#[derive(Debug)]
pub(crate) enum OsHandle {
    OsFile(OsFile),
    Stdin,
    Stdout,
    Stderr,
}

impl OsHandle {
    pub(crate) fn as_os_file(&self) -> Result<&OsFile> {
        match self {
            Self::OsFile(fd) => Ok(fd),
            _ => Err(Errno::Badf),
        }
    }

    pub(crate) fn stdin() -> Self {
        Self::Stdin
    }

    pub(crate) fn stdout() -> Self {
        Self::Stdout
    }

    pub(crate) fn stderr() -> Self {
        Self::Stderr
    }
}

pub(crate) trait AsFile {
    fn as_file(&self) -> ManuallyDrop<File>;
}

pub(crate) trait OsHandleExt: Sized {
    /// Returns the file type.
    fn get_file_type(&self) -> io::Result<types::Filetype>;
    /// Returns the set of all possible rights that are both relevant for the file
    /// type and consistent with the open mode.
    fn get_rights(&self, filetype: types::Filetype) -> io::Result<EntryRights>;
    fn from_null() -> io::Result<Self>;
}

impl From<OsFile> for OsHandle {
    fn from(file: OsFile) -> Self {
        Self::OsFile(file)
    }
}

impl Handle for OsHandle {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn try_clone(&self) -> io::Result<Box<dyn Handle>> {
        let new_handle = match self {
            Self::OsFile(file) => Self::OsFile(file.try_clone()?),
            Self::Stdin => Self::Stdin,
            Self::Stdout => Self::Stdout,
            Self::Stderr => Self::Stderr,
        };
        Ok(Box::new(new_handle))
    }
    fn get_file_type(&self) -> io::Result<types::Filetype> {
        <Self as OsHandleExt>::get_file_type(self)
    }

    fn get_rights(&self) -> io::Result<EntryRights> {
        <Self as OsHandleExt>::get_rights(self, <Self as Handle>::get_file_type(self)?)
    }
    // FdOps
    fn advise(
        &self,
        advice: types::Advice,
        offset: types::Filesize,
        len: types::Filesize,
    ) -> Result<()> {
        fd::advise(self.as_os_file()?, advice, offset, len)
    }
    fn allocate(&self, offset: types::Filesize, len: types::Filesize) -> Result<()> {
        let fd = self.as_file();
        let metadata = fd.metadata()?;
        let current_size = metadata.len();
        let wanted_size = offset.checked_add(len).ok_or(Errno::TooBig)?;
        // This check will be unnecessary when rust-lang/rust#63326 is fixed
        if wanted_size > i64::max_value() as u64 {
            return Err(Errno::TooBig);
        }
        if wanted_size > current_size {
            fd.set_len(wanted_size)?;
        }
        Ok(())
    }
    fn datasync(&self) -> Result<()> {
        self.as_file().sync_data()?;
        Ok(())
    }
    fn fdstat_get(&self) -> Result<types::Fdflags> {
        fd::fdstat_get(&self.as_file())
    }
    fn fdstat_set_flags(&self, fdflags: types::Fdflags) -> Result<()> {
        if let Some(new_file) = fd::fdstat_set_flags(&self.as_file(), fdflags)? {
            // If we don't deal with OsFile, then something went wrong, and we
            // should fail. On the other hand, is that even possible?
            self.as_os_file()?.update_from(new_file);
        }
        Ok(())
    }
    fn filestat_get(&self) -> Result<types::Filestat> {
        fd::filestat_get(&self.as_file())
    }
    fn filestat_set_size(&self, size: types::Filesize) -> Result<()> {
        self.as_os_file()?.as_file().set_len(size)?;
        Ok(())
    }
    fn filestat_set_times(
        &self,
        atim: types::Timestamp,
        mtim: types::Timestamp,
        fst_flags: types::Fstflags,
    ) -> Result<()> {
        fd::filestat_set_times(&self.as_file(), atim, mtim, fst_flags)
    }
    fn preadv(&self, buf: &mut [io::IoSliceMut], offset: u64) -> Result<usize> {
        let mut fd: &File = &self.as_os_file()?.as_file();
        let cur_pos = fd.seek(SeekFrom::Current(0))?;
        fd.seek(SeekFrom::Start(offset))?;
        let nread = self.read_vectored(buf)?;
        fd.seek(SeekFrom::Start(cur_pos))?;
        Ok(nread)
    }
    fn pwritev(&self, buf: &[io::IoSlice], offset: u64) -> Result<usize> {
        let mut fd: &File = &self.as_os_file()?.as_file();
        let cur_pos = fd.seek(SeekFrom::Current(0))?;
        fd.seek(SeekFrom::Start(offset))?;
        let nwritten = self.write_vectored(&buf, false)?;
        fd.seek(SeekFrom::Start(cur_pos))?;
        Ok(nwritten)
    }
    fn read_vectored(&self, iovs: &mut [io::IoSliceMut]) -> Result<usize> {
        let nread = match self {
            Self::OsFile(file) => file.as_file().read_vectored(iovs)?,
            Self::Stdin => io::stdin().read_vectored(iovs)?,
            _ => return Err(Errno::Badf),
        };
        Ok(nread)
    }
    fn readdir<'a>(
        &'a self,
        cookie: types::Dircookie,
    ) -> Result<Box<dyn Iterator<Item = Result<(types::Dirent, String)>> + 'a>> {
        fd::readdir(self.as_os_file()?, cookie)
    }
    fn seek(&self, offset: SeekFrom) -> Result<u64> {
        let pos = self.as_os_file()?.as_file().seek(offset)?;
        Ok(pos)
    }
    fn sync(&self) -> Result<()> {
        self.as_os_file()?.as_file().sync_all()?;
        Ok(())
    }
    fn write_vectored(&self, iovs: &[io::IoSlice], isatty: bool) -> Result<usize> {
        let nwritten = match self {
            Self::OsFile(file) => {
                let mut file: &File = &file.as_file();
                if isatty {
                    SandboxedTTYWriter::new(&mut file).write_vectored(&iovs)?
                } else {
                    file.write_vectored(&iovs)?
                }
            }
            Self::Stdin => return Err(Errno::Badf),
            Self::Stdout => {
                // lock for the duration of the scope
                let stdout = io::stdout();
                let mut stdout = stdout.lock();
                let nwritten = if isatty {
                    SandboxedTTYWriter::new(&mut stdout).write_vectored(&iovs)?
                } else {
                    stdout.write_vectored(&iovs)?
                };
                stdout.flush()?;
                nwritten
            }
            // Always sanitize stderr, even if it's not directly connected to a tty,
            // because stderr is meant for diagnostics rather than binary output,
            // and may be redirected to a file which could end up being displayed
            // on a tty later.
            Self::Stderr => SandboxedTTYWriter::new(&mut io::stderr()).write_vectored(&iovs)?,
        };
        Ok(nwritten)
    }
    // PathOps
    fn create_directory(&self, path: &str) -> Result<()> {
        path::create_directory(self.as_os_file()?, path)
    }
    fn openat(
        &self,
        path: &str,
        read: bool,
        write: bool,
        oflags: types::Oflags,
        fd_flags: types::Fdflags,
    ) -> Result<Box<dyn Handle>> {
        let handle = path::open(self.as_os_file()?, path, read, write, oflags, fd_flags)?;
        Ok(Box::new(handle))
    }
    fn link(
        &self,
        old_path: &str,
        new_handle: Box<dyn Handle>,
        new_path: &str,
        follow: bool,
    ) -> Result<()> {
        let new_handle = match new_handle.as_any().downcast_ref::<Self>() {
            None => {
                error!("Tried to link OS resource with Virtual");
                return Err(Errno::Badf);
            }
            Some(handle) => handle,
        };
        path::link(
            self.as_os_file()?,
            old_path,
            new_handle.as_os_file()?,
            new_path,
            follow,
        )
    }
    fn symlink(&self, old_path: &str, new_path: &str) -> Result<()> {
        path::symlink(old_path, self.as_os_file()?, new_path)
    }
    fn readlink(&self, path: &str, buf: &mut [u8]) -> Result<usize> {
        path::readlink(self.as_os_file()?, path, buf)
    }
    fn readlinkat(&self, path: &str) -> Result<String> {
        path::readlinkat(self.as_os_file()?, path)
    }
    fn rename(&self, old_path: &str, new_handle: Box<dyn Handle>, new_path: &str) -> Result<()> {
        let new_handle = match new_handle.as_any().downcast_ref::<Self>() {
            None => {
                error!("Tried to link OS resource with Virtual");
                return Err(Errno::Badf);
            }
            Some(handle) => handle,
        };
        debug!("rename (old_dirfd, old_path)=({:?}, {:?})", self, old_path);
        debug!(
            "rename (new_dirfd, new_path)=({:?}, {:?})",
            new_handle, new_path
        );
        path::rename(
            self.as_os_file()?,
            old_path,
            new_handle.as_os_file()?,
            new_path,
        )
    }
    fn remove_directory(&self, path: &str) -> Result<()> {
        debug!("remove_directory (dirfd, path)=({:?}, {:?})", self, path);
        path::remove_directory(self.as_os_file()?, path)
    }
    fn unlink_file(&self, path: &str) -> Result<()> {
        path::unlink_file(self.as_os_file()?, path)
    }
}
