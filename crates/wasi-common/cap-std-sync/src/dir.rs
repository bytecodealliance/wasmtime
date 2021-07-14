use crate::file::{filetype_from, File};
use cap_fs_ext::{DirEntryExt, DirExt, MetadataExt, SystemTimeSpec};
use std::any::Any;
use std::path::{Path, PathBuf};
use system_interface::fs::GetSetFdFlags;
use wasi_common::{
    dir::{ReaddirCursor, ReaddirEntity, WasiDir},
    file::{FdFlags, FileType, Filestat, OFlags, WasiFile},
    Error, ErrorExt,
};

pub struct Dir(cap_std::fs::Dir);

impl Dir {
    pub fn from_cap_std(dir: cap_std::fs::Dir) -> Self {
        Dir(dir)
    }

    pub fn open_file_(
        &self,
        symlink_follow: bool,
        path: &str,
        oflags: OFlags,
        read: bool,
        write: bool,
        fdflags: FdFlags,
    ) -> Result<File, Error> {
        use cap_fs_ext::{FollowSymlinks, OpenOptionsFollowExt};

        let mut opts = cap_std::fs::OpenOptions::new();

        if oflags.contains(OFlags::CREATE | OFlags::EXCLUSIVE) {
            opts.create_new(true);
            opts.write(true);
        } else if oflags.contains(OFlags::CREATE) {
            opts.create(true);
            opts.write(true);
        }
        if oflags.contains(OFlags::TRUNCATE) {
            opts.truncate(true);
        }
        if read {
            opts.read(true);
        }
        if write {
            opts.write(true);
        } else {
            // If not opened write, open read. This way the OS lets us open the file.
            // If FileCaps::READ is not set, read calls will be rejected at the
            // get_cap check.
            opts.read(true);
        }
        if fdflags.contains(FdFlags::APPEND) {
            opts.append(true);
        }

        if symlink_follow {
            opts.follow(FollowSymlinks::Yes);
        } else {
            opts.follow(FollowSymlinks::No);
        }
        // the DSYNC, SYNC, and RSYNC flags are ignored! We do not
        // have support for them in cap-std yet.
        // ideally OpenOptions would just support this though:
        // https://github.com/bytecodealliance/cap-std/issues/146
        if fdflags.intersects(
            wasi_common::file::FdFlags::DSYNC
                | wasi_common::file::FdFlags::SYNC
                | wasi_common::file::FdFlags::RSYNC,
        ) {
            return Err(Error::not_supported().context("SYNC family of FdFlags"));
        }

        let mut f = self.0.open_with(Path::new(path), &opts)?;
        // NONBLOCK does not have an OpenOption either, but we can patch that on with set_fd_flags:
        if fdflags.contains(wasi_common::file::FdFlags::NONBLOCK) {
            let set_fd_flags = f.new_set_fd_flags(system_interface::fs::FdFlags::NONBLOCK)?;
            f.set_fd_flags(set_fd_flags)?;
        }
        Ok(File::from_cap_std(f))
    }

    pub fn open_dir_(&self, symlink_follow: bool, path: &str) -> Result<Self, Error> {
        let d = if symlink_follow {
            self.0.open_dir(Path::new(path))?
        } else {
            self.0.open_dir_nofollow(Path::new(path))?
        };
        Ok(Dir::from_cap_std(d))
    }

    pub fn rename_(&self, src_path: &str, dest_dir: &Self, dest_path: &str) -> Result<(), Error> {
        self.0
            .rename(Path::new(src_path), &dest_dir.0, Path::new(dest_path))?;
        Ok(())
    }
    pub fn hard_link_(
        &self,
        src_path: &str,
        target_dir: &Self,
        target_path: &str,
    ) -> Result<(), Error> {
        let src_path = Path::new(src_path);
        let target_path = Path::new(target_path);
        self.0.hard_link(src_path, &target_dir.0, target_path)?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl WasiDir for Dir {
    fn as_any(&self) -> &dyn Any {
        self
    }
    async fn open_file(
        &self,
        symlink_follow: bool,
        path: &str,
        oflags: OFlags,
        read: bool,
        write: bool,
        fdflags: FdFlags,
    ) -> Result<Box<dyn WasiFile>, Error> {
        let f = self.open_file_(symlink_follow, path, oflags, read, write, fdflags)?;
        Ok(Box::new(f))
    }

    async fn open_dir(&self, symlink_follow: bool, path: &str) -> Result<Box<dyn WasiDir>, Error> {
        let d = self.open_dir_(symlink_follow, path)?;
        Ok(Box::new(d))
    }

    async fn create_dir(&self, path: &str) -> Result<(), Error> {
        self.0.create_dir(Path::new(path))?;
        Ok(())
    }
    async fn readdir(
        &self,
        cursor: ReaddirCursor,
    ) -> Result<Box<dyn Iterator<Item = Result<ReaddirEntity, Error>> + Send>, Error> {
        // cap_std's read_dir does not include . and .., we should prepend these.
        // Why does the Ok contain a tuple? We can't construct a cap_std::fs::DirEntry, and we don't
        // have enough info to make a ReaddirEntity yet.
        let dir_meta = self.0.dir_metadata()?;
        let rd = vec![
            {
                let name = ".".to_owned();
                Ok((FileType::Directory, dir_meta.ino(), name))
            },
            {
                let name = "..".to_owned();
                Ok((FileType::Directory, dir_meta.ino(), name))
            },
        ]
        .into_iter()
        .chain({
            // Now process the `DirEntry`s:
            let entries = self.0.entries()?.map(|entry| {
                let entry = entry?;
                let meta = entry.full_metadata()?;
                let inode = meta.ino();
                let filetype = filetype_from(&meta.file_type());
                let name = entry
                    .file_name()
                    .into_string()
                    .map_err(|_| Error::illegal_byte_sequence().context("filename"))?;
                Ok((filetype, inode, name))
            });

            // On Windows, filter out files like `C:\DumpStack.log.tmp` which we
            // can't get a full metadata for.
            #[cfg(windows)]
            let entries = entries.filter(|entry: &Result<_, wasi_common::Error>| {
                use winapi::shared::winerror::{ERROR_ACCESS_DENIED, ERROR_SHARING_VIOLATION};
                if let Err(err) = entry {
                    if let Some(err) = err.downcast_ref::<std::io::Error>() {
                        if err.raw_os_error() == Some(ERROR_SHARING_VIOLATION as i32)
                            || err.raw_os_error() == Some(ERROR_ACCESS_DENIED as i32)
                        {
                            return false;
                        }
                    }
                }
                true
            });

            entries
        })
        // Enumeration of the iterator makes it possible to define the ReaddirCursor
        .enumerate()
        .map(|(ix, r)| match r {
            Ok((filetype, inode, name)) => Ok(ReaddirEntity {
                next: ReaddirCursor::from(ix as u64 + 1),
                filetype,
                inode,
                name,
            }),
            Err(e) => Err(e),
        })
        .skip(u64::from(cursor) as usize);

        Ok(Box::new(rd))
    }

    async fn symlink(&self, src_path: &str, dest_path: &str) -> Result<(), Error> {
        self.0.symlink(src_path, dest_path)?;
        Ok(())
    }
    async fn remove_dir(&self, path: &str) -> Result<(), Error> {
        self.0.remove_dir(Path::new(path))?;
        Ok(())
    }

    async fn unlink_file(&self, path: &str) -> Result<(), Error> {
        self.0.remove_file_or_symlink(Path::new(path))?;
        Ok(())
    }
    async fn read_link(&self, path: &str) -> Result<PathBuf, Error> {
        let link = self.0.read_link(Path::new(path))?;
        Ok(link)
    }
    async fn get_filestat(&self) -> Result<Filestat, Error> {
        let meta = self.0.dir_metadata()?;
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
    async fn get_path_filestat(
        &self,
        path: &str,
        follow_symlinks: bool,
    ) -> Result<Filestat, Error> {
        let meta = if follow_symlinks {
            self.0.metadata(Path::new(path))?
        } else {
            self.0.symlink_metadata(Path::new(path))?
        };
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
    async fn rename(
        &self,
        src_path: &str,
        dest_dir: &dyn WasiDir,
        dest_path: &str,
    ) -> Result<(), Error> {
        let dest_dir = dest_dir
            .as_any()
            .downcast_ref::<Self>()
            .ok_or(Error::badf().context("failed downcast to cap-std Dir"))?;
        self.rename_(src_path, dest_dir, dest_path)
    }
    async fn hard_link(
        &self,
        src_path: &str,
        target_dir: &dyn WasiDir,
        target_path: &str,
    ) -> Result<(), Error> {
        let target_dir = target_dir
            .as_any()
            .downcast_ref::<Self>()
            .ok_or(Error::badf().context("failed downcast to cap-std Dir"))?;
        self.hard_link_(src_path, target_dir, target_path)
    }
    async fn set_times(
        &self,
        path: &str,
        atime: Option<wasi_common::SystemTimeSpec>,
        mtime: Option<wasi_common::SystemTimeSpec>,
        follow_symlinks: bool,
    ) -> Result<(), Error> {
        if follow_symlinks {
            self.0.set_times(
                Path::new(path),
                convert_systimespec(atime),
                convert_systimespec(mtime),
            )?;
        } else {
            self.0.set_symlink_times(
                Path::new(path),
                convert_systimespec(atime),
                convert_systimespec(mtime),
            )?;
        }
        Ok(())
    }
}

fn convert_systimespec(t: Option<wasi_common::SystemTimeSpec>) -> Option<SystemTimeSpec> {
    match t {
        Some(wasi_common::SystemTimeSpec::Absolute(t)) => Some(SystemTimeSpec::Absolute(t)),
        Some(wasi_common::SystemTimeSpec::SymbolicNow) => Some(SystemTimeSpec::SymbolicNow),
        None => None,
    }
}

#[cfg(test)]
mod test {
    use super::Dir;
    use cap_std::ambient_authority;
    #[test]
    fn scratch_dir() {
        let tempdir = tempfile::Builder::new()
            .prefix("cap-std-sync")
            .tempdir()
            .expect("create temporary dir");
        let preopen_dir = cap_std::fs::Dir::open_ambient_dir(tempdir.path(), ambient_authority())
            .expect("open ambient temporary dir");
        let preopen_dir = Dir::from_cap_std(preopen_dir);
        run(wasi_common::WasiDir::open_dir(&preopen_dir, false, "."))
            .expect("open the same directory via WasiDir abstraction");
    }

    // Readdir does not work on windows, so we won't test it there.
    #[cfg(not(windows))]
    #[test]
    fn readdir() {
        use std::collections::HashMap;
        use wasi_common::dir::{ReaddirCursor, ReaddirEntity, WasiDir};
        use wasi_common::file::{FdFlags, FileType, OFlags};

        fn readdir_into_map(dir: &dyn WasiDir) -> HashMap<String, ReaddirEntity> {
            let mut out = HashMap::new();
            for readdir_result in
                run(dir.readdir(ReaddirCursor::from(0))).expect("readdir succeeds")
            {
                let entity = readdir_result.expect("readdir entry is valid");
                out.insert(entity.name.clone(), entity);
            }
            out
        }

        let tempdir = tempfile::Builder::new()
            .prefix("cap-std-sync")
            .tempdir()
            .expect("create temporary dir");
        let preopen_dir = cap_std::fs::Dir::open_ambient_dir(tempdir.path(), ambient_authority())
            .expect("open ambient temporary dir");
        let preopen_dir = Dir::from_cap_std(preopen_dir);

        let entities = readdir_into_map(&preopen_dir);
        assert_eq!(
            entities.len(),
            2,
            "should just be . and .. in empty dir: {:?}",
            entities
        );
        assert!(entities.get(".").is_some());
        assert!(entities.get("..").is_some());

        run(preopen_dir.open_file(
            false,
            "file1",
            OFlags::CREATE,
            true,
            false,
            FdFlags::empty(),
        ))
        .expect("create file1");

        let entities = readdir_into_map(&preopen_dir);
        assert_eq!(entities.len(), 3, "should be ., .., file1 {:?}", entities);
        assert_eq!(
            entities.get(".").expect(". entry").filetype,
            FileType::Directory
        );
        assert_eq!(
            entities.get("..").expect(".. entry").filetype,
            FileType::Directory
        );
        assert_eq!(
            entities.get("file1").expect("file1 entry").filetype,
            FileType::RegularFile
        );
    }

    fn run<F: std::future::Future>(future: F) -> F::Output {
        use std::pin::Pin;
        use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

        let mut f = Pin::from(Box::new(future));
        let waker = dummy_waker();
        let mut cx = Context::from_waker(&waker);
        match f.as_mut().poll(&mut cx) {
            Poll::Ready(val) => return val,
            Poll::Pending => {
                panic!("Cannot wait on pending future: must enable wiggle \"async\" future and execute on an async Store")
            }
        }

        fn dummy_waker() -> Waker {
            return unsafe { Waker::from_raw(clone(5 as *const _)) };

            unsafe fn clone(ptr: *const ()) -> RawWaker {
                assert_eq!(ptr as usize, 5);
                const VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);
                RawWaker::new(ptr, &VTABLE)
            }

            unsafe fn wake(ptr: *const ()) {
                assert_eq!(ptr as usize, 5);
            }

            unsafe fn wake_by_ref(ptr: *const ()) {
                assert_eq!(ptr as usize, 5);
            }

            unsafe fn drop(ptr: *const ()) {
                assert_eq!(ptr as usize, 5);
            }
        }
    }
}
