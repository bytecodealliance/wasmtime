#![allow(dead_code, unused_variables, unused_imports)]
use cap_std::time::{Duration, SystemTime};
use std::any::Any;
use std::cell::{Cell, Ref, RefCell, RefMut};
use std::collections::{hash_map::Entry, HashMap};
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
    // Always .get() out a Some - this is an RefCell<Option to get around a circular init problem
    root: RefCell<Option<Rc<RefCell<DirInode>>>>,
    clock: Box<dyn WasiSystemClock>,
    device_id: u64,
    next_serial: Cell<u64>,
}

impl Filesystem {
    pub fn new(clock: Box<dyn WasiSystemClock>, device_id: u64) -> Rc<Filesystem> {
        let now = clock.now(Duration::from_secs(0));
        let fs = Rc::new(Filesystem {
            root: RefCell::new(None),
            clock,
            device_id,
            next_serial: Cell::new(1),
        });
        let root = Rc::new(RefCell::new(DirInode {
            fs: Rc::downgrade(&fs),
            serial: 0,
            parent: None,
            contents: HashMap::new(),
            atim: now,
            mtim: now,
            ctim: now,
        }));
        fs.root.replace(Some(root.clone()));
        fs
    }
    pub fn root(&self) -> Box<dyn WasiDir> {
        Box::new(Dir(self
            .root
            .borrow()
            .as_ref()
            .expect("root option always Some after init")
            .clone())) as Box<dyn WasiDir>
    }
    fn now(&self) -> SystemTime {
        self.clock.now(Duration::from_secs(0))
    }
    fn fresh_serial(&self) -> u64 {
        let s = self.next_serial.get();
        self.next_serial.set(s + 1);
        s
    }
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

pub struct Dir(Rc<RefCell<DirInode>>);

impl Dir {
    fn at_path<F, A>(&self, path: &str, f: F) -> Result<A, Error>
    where
        F: FnOnce(&Dir, &str) -> Result<A, Error>,
    {
        // Doesnt even attempt to deal with trailing slashes
        if let Some(slash_index) = path.find('/') {
            let dirname = &path[..slash_index];
            let rest = &path[slash_index + 1..];
            if rest == "" {
                return Err(Error::invalid_argument()
                    .context("empty filename, probably related to trailing slashes"));
            }
            if let Some(inode) = self.0.borrow().contents.get(dirname) {
                match inode {
                    Inode::Dir(d) => Dir(d.clone()).at_path(rest, f),
                    Inode::File { .. } => Err(Error::not_found()),
                }
            } else {
                Err(Error::not_found())
            }
        } else {
            f(self, path)
        }
    }
    fn child_dir(&self, name: &str) -> Result<Rc<RefCell<DirInode>>, Error> {
        match self.0.borrow().contents.get(name) {
            Some(Inode::Dir(d)) => Ok(d.clone()),
            _ => Err(Error::not_found()),
        }
    }
    fn child_file(&self, name: &str) -> Result<Rc<RefCell<FileInode>>, Error> {
        match self.0.borrow().contents.get(name) {
            Some(Inode::File(f)) => Ok(f.clone()),
            _ => Err(Error::not_found()),
        }
    }
}

impl WasiDir for Dir {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn open_file(
        &self,
        _symlink_follow: bool,
        path: &str,
        oflags: OFlags,
        read: bool,
        write: bool,
        fdflags: FdFlags,
    ) -> Result<Box<dyn WasiFile>, Error> {
        let mode = if read && write {
            FileMode::ReadWrite
        } else if read {
            FileMode::ReadOnly
        } else if write {
            FileMode::WriteOnly
        } else {
            return Err(Error::invalid_argument().context("must be read or write"));
        };
        // XXX TERRIBLE CODE DUPLICATION HERE
        self.at_path(path, |dir, filename| {
            if oflags.contains(OFlags::CREATE | OFlags::EXCLUSIVE) {
                match dir.child_file(filename) {
                    Err(_notfound) => {
                        let d = dir.0.borrow();
                        let now = d.fs().now();
                        let serial = d.fs().fresh_serial();
                        let inode = Rc::new(RefCell::new(FileInode {
                            fs: d.fs.clone(),
                            serial,
                            contents: Vec::new(),
                            atim: now,
                            ctim: now,
                            mtim: now,
                        }));
                        dir.0
                            .borrow_mut()
                            .contents
                            .insert(filename.to_owned(), Inode::File(inode.clone()));
                        Ok(Box::new(File {
                            inode,
                            position: Cell::new(0),
                            fdflags,
                            mode,
                        }) as Box<dyn WasiFile>)
                    }
                    Ok(_) => Err(Error::exist()),
                }
            } else if oflags.contains(OFlags::CREATE) {
                match dir.child_file(filename) {
                    Ok(inode) => {
                        // XXX update atime here!
                        Ok(Box::new(File {
                            inode,
                            position: Cell::new(0),
                            fdflags,
                            mode,
                        }) as Box<dyn WasiFile>)
                    }
                    Err(_notfound) => {
                        let d = dir.0.borrow();
                        let now = d.fs().now();
                        let serial = d.fs().fresh_serial();
                        let inode = Rc::new(RefCell::new(FileInode {
                            fs: d.fs.clone(),
                            serial,
                            contents: Vec::new(),
                            atim: now,
                            ctim: now,
                            mtim: now,
                        }));
                        dir.0
                            .borrow_mut()
                            .contents
                            .insert(filename.to_owned(), Inode::File(inode.clone()));
                        Ok(Box::new(File {
                            inode,
                            position: Cell::new(0),
                            fdflags,
                            mode,
                        }) as Box<dyn WasiFile>)
                    }
                }
            } else {
                let inode = dir.child_file(filename)?;
                // XXX update atime here!
                Ok(Box::new(File {
                    inode,
                    position: Cell::new(0),
                    fdflags,
                    mode,
                }) as Box<dyn WasiFile>)
            }
        })
    }

    fn open_dir(&self, _symlink_follow: bool, path: &str) -> Result<Box<dyn WasiDir>, Error> {
        self.at_path(path, |dir, dirname| {
            let d = dir.child_dir(dirname)?;
            Ok(Box::new(Dir(d)) as Box<dyn WasiDir>)
        })
    }

    fn create_dir(&self, path: &str) -> Result<(), Error> {
        self.at_path(path, |dir, dirname| {
            let d = dir.0.borrow();
            let fs = d.fs.clone();
            let serial = d.fs().fresh_serial();
            let now = d.fs().now();
            drop(d);
            match dir.0.borrow_mut().contents.entry(dirname.to_owned()) {
                Entry::Vacant(v) => {
                    let parent = Some(Rc::downgrade(&self.0));
                    v.insert(Inode::Dir(Rc::new(RefCell::new(DirInode {
                        fs,
                        serial,
                        parent,
                        contents: HashMap::new(),
                        atim: now,
                        mtim: now,
                        ctim: now,
                    }))));
                    Ok(())
                }
                Entry::Occupied(_) => Err(Error::exist()),
            }
        })
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
