#![allow(dead_code, unused_variables, unused_imports)]
use cap_std::time::{Duration, SystemTime};
use std::any::Any;
use std::cell::{Cell, Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::convert::TryInto;
use std::io::{Cursor, IoSlice, IoSliceMut, Read, Seek, SeekFrom, Write};
use std::ops::Deref;
use std::path::PathBuf;
use std::rc::{Rc, Weak};
use wasi_common::{
    clocks::WasiSystemClock,
    dir::{ReaddirCursor, ReaddirEntity, WasiDir},
    file::{Advice, FdFlags, FileCaps, FileType, Filestat, OFlags, WasiFile},
    Error, ErrorExt, SystemTimeSpec,
};

pub struct Filesystem {
    root: Rc<RefCell<DirInode>>,
    clock: Box<dyn WasiSystemClock>,
    device_id: u64,
    next_serial: Cell<u64>,
}

pub enum Inode {
    Dir(Rc<RefCell<DirInode>>),
    File(Rc<RefCell<FileInode>>),
}

pub struct DirInode {
    fs: Weak<Filesystem>,
    serial: u64,
    parent: Option<Weak<RefCell<DirInode>>>,
    contents: HashMap<String, Inode>,
    atim: SystemTime,
    mtim: SystemTime,
    ctim: SystemTime,
}

impl DirInode {
    pub fn fs(&self) -> Rc<Filesystem> {
        Weak::upgrade(&self.fs).unwrap()
    }
}

pub struct FileInode {
    fs: Weak<Filesystem>,
    serial: u64,
    contents: Vec<u8>,
    atim: SystemTime,
    mtim: SystemTime,
    ctim: SystemTime,
}

impl FileInode {
    pub fn fs(&self) -> Rc<Filesystem> {
        Weak::upgrade(&self.fs).unwrap()
    }
}

enum FileMode {
    ReadOnly,
    WriteOnly,
    ReadWrite,
}

pub struct File {
    inode: Rc<RefCell<FileInode>>,
    position: Cell<u64>,
    fdflags: FdFlags,
    mode: FileMode,
}

impl File {
    fn is_read(&self) -> bool {
        match self.mode {
            FileMode::ReadOnly | FileMode::ReadWrite => true,
            _ => false,
        }
    }
    fn is_write(&self) -> bool {
        match self.mode {
            FileMode::WriteOnly | FileMode::ReadWrite => true,
            _ => false,
        }
    }
    fn is_append(&self) -> bool {
        self.fdflags.contains(FdFlags::APPEND)
    }
    fn inode(&self) -> Ref<FileInode> {
        self.inode.borrow()
    }
    fn inode_mut(&self) -> RefMut<FileInode> {
        self.inode.borrow_mut()
    }
}

impl WasiFile for File {
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
        Ok(FileType::RegularFile)
    }
    fn get_fdflags(&self) -> Result<FdFlags, Error> {
        Ok(self.fdflags)
    }
    fn set_fdflags(&mut self, fdflags: FdFlags) -> Result<(), Error> {
        self.fdflags = fdflags;
        Ok(())
    }
    fn get_filestat(&self) -> Result<Filestat, Error> {
        let inode = self.inode();
        let fs = inode.fs();
        Ok(Filestat {
            device_id: fs.device_id,
            inode: inode.serial,
            filetype: self.get_filetype().unwrap(),
            nlink: 0,
            size: inode.contents.len() as u64,
            atim: Some(inode.atim.into_std()),
            ctim: Some(inode.ctim.into_std()),
            mtim: Some(inode.mtim.into_std()),
        })
    }
    fn set_filestat_size(&self, size: u64) -> Result<(), Error> {
        let mut inode = self.inode.borrow_mut();
        inode.contents.resize(size.try_into()?, 0);
        Ok(())
    }
    fn advise(&self, _offset: u64, _len: u64, _advice: Advice) -> Result<(), Error> {
        Ok(())
    }
    fn allocate(&self, offset: u64, len: u64) -> Result<(), Error> {
        let mut inode = self.inode.borrow_mut();
        let size = offset.checked_add(len).ok_or_else(|| Error::overflow())?;
        if size > inode.contents.len() as u64 {
            inode.contents.resize(size.try_into()?, 0);
        }
        Ok(())
    }
    fn set_times(
        &self,
        atime: Option<SystemTimeSpec>,
        mtime: Option<SystemTimeSpec>,
    ) -> Result<(), Error> {
        let newtime = |s| match s {
            SystemTimeSpec::SymbolicNow => self.inode().fs().clock.now(Duration::from_secs(0)),
            SystemTimeSpec::Absolute(t) => t,
        };
        let mut inode = self.inode.borrow_mut();
        if let Some(atim) = atime {
            inode.atim = newtime(atim);
        }
        if let Some(mtim) = mtime {
            inode.mtim = newtime(mtim);
        }
        Ok(())
    }
    fn read_vectored(&self, bufs: &mut [IoSliceMut]) -> Result<u64, Error> {
        if !self.is_read() {
            return Err(Error::badf());
        }
        let inode = self.inode();
        let mut cursor = Cursor::new(inode.contents.as_slice());
        cursor.set_position(self.position.get());
        let nbytes = cursor.read_vectored(bufs)?;
        self.position.set(cursor.position());
        Ok(nbytes.try_into()?)
    }
    fn read_vectored_at(&self, bufs: &mut [IoSliceMut], offset: u64) -> Result<u64, Error> {
        if !self.is_read() {
            return Err(Error::badf());
        }
        let inode = self.inode();
        let mut cursor = Cursor::new(inode.contents.as_slice());
        cursor.set_position(offset);
        let nbytes = cursor.read_vectored(bufs)?;
        Ok(nbytes.try_into()?)
    }
    fn write_vectored(&self, bufs: &[IoSlice]) -> Result<u64, Error> {
        if !self.is_write() {
            return Err(Error::badf());
        }
        let mut inode = self.inode_mut();
        let mut cursor = Cursor::new(&mut inode.contents);
        cursor.set_position(self.position.get());
        let nbytes = cursor.write_vectored(bufs)?;
        self.position.set(cursor.position());
        Ok(nbytes.try_into()?)
    }
    fn write_vectored_at(&self, bufs: &[IoSlice], offset: u64) -> Result<u64, Error> {
        if !self.is_write() || self.is_append() {
            return Err(Error::badf());
        }
        let mut inode = self.inode_mut();
        let mut cursor = Cursor::new(&mut inode.contents);
        cursor.set_position(offset);
        let nbytes = cursor.write_vectored(bufs)?;
        self.position.set(cursor.position());
        Ok(nbytes.try_into()?)
    }
    fn seek(&self, pos: SeekFrom) -> Result<u64, Error> {
        if self.is_append() {
            match pos {
                SeekFrom::Current(0) => return Ok(self.position.get()),
                _ => return Err(Error::badf()),
            }
        }
        let inode = self.inode();
        let mut cursor = Cursor::new(inode.contents.as_slice());
        cursor.set_position(self.position.get());
        cursor.seek(pos)?;
        let new_position = cursor.position();
        self.position.set(new_position);
        Ok(new_position)
    }
    fn peek(&self, buf: &mut [u8]) -> Result<u64, Error> {
        if !self.is_read() {
            return Err(Error::badf());
        }
        let inode = self.inode();
        let mut cursor = Cursor::new(inode.contents.as_slice());
        cursor.set_position(self.position.get());
        let nbytes = cursor.read(buf)?;
        Ok(nbytes.try_into()?)
    }
    fn num_ready_bytes(&self) -> Result<u64, Error> {
        if !self.is_read() {
            return Err(Error::badf());
        }
        let len = self.inode().contents.len() as u64;
        Ok(len - self.position.get())
    }
}

pub struct Dir;

impl Dir {}

impl WasiDir for Dir {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn open_file(
        &self,
        symlink_follow: bool,
        path: &str,
        oflags: OFlags,
        read: bool,
        write: bool,
        fdflags: FdFlags,
    ) -> Result<Box<dyn WasiFile>, Error> {
        todo!()
    }

    fn open_dir(&self, symlink_follow: bool, path: &str) -> Result<Box<dyn WasiDir>, Error> {
        todo!()
    }

    fn create_dir(&self, path: &str) -> Result<(), Error> {
        todo!()
    }
    fn readdir(
        &self,
        cursor: ReaddirCursor,
    ) -> Result<Box<dyn Iterator<Item = Result<(ReaddirEntity, String), Error>>>, Error> {
        todo!()
    }

    fn symlink(&self, src_path: &str, dest_path: &str) -> Result<(), Error> {
        todo!()
    }
    fn remove_dir(&self, path: &str) -> Result<(), Error> {
        todo!()
    }

    fn unlink_file(&self, path: &str) -> Result<(), Error> {
        todo!()
    }
    fn read_link(&self, path: &str) -> Result<PathBuf, Error> {
        todo!()
    }
    fn get_filestat(&self) -> Result<Filestat, Error> {
        todo!()
    }
    fn get_path_filestat(&self, path: &str, follow_symlinks: bool) -> Result<Filestat, Error> {
        todo!()
    }
    fn rename(&self, src_path: &str, dest_dir: &dyn WasiDir, dest_path: &str) -> Result<(), Error> {
        todo!()
    }
    fn hard_link(
        &self,
        src_path: &str,
        target_dir: &dyn WasiDir,
        target_path: &str,
    ) -> Result<(), Error> {
        todo!()
    }
    fn set_times(
        &self,
        path: &str,
        atime: Option<wasi_common::SystemTimeSpec>,
        mtime: Option<wasi_common::SystemTimeSpec>,
        follow_symlinks: bool,
    ) -> Result<(), Error> {
        todo!()
    }
}
