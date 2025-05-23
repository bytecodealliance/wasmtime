use crate::tokio::{block_on_dummy_executor, file::File};
use crate::{
    Error, ErrorExt,
    dir::{ReaddirCursor, ReaddirEntity, WasiDir},
    file::{FdFlags, Filestat, OFlags},
};
use std::any::Any;
use std::path::PathBuf;

pub struct Dir(crate::sync::dir::Dir);

impl Dir {
    pub fn from_cap_std(dir: cap_std::fs::Dir) -> Self {
        Dir(crate::sync::dir::Dir::from_cap_std(dir))
    }
}

#[wiggle::async_trait]
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
    ) -> Result<crate::dir::OpenResult, Error> {
        let f = block_on_dummy_executor(move || async move {
            self.0
                .open_file_(symlink_follow, path, oflags, read, write, fdflags)
        })?;
        match f {
            crate::sync::dir::OpenResult::File(f) => {
                Ok(crate::dir::OpenResult::File(Box::new(File::from_inner(f))))
            }
            crate::sync::dir::OpenResult::Dir(d) => {
                Ok(crate::dir::OpenResult::Dir(Box::new(Dir(d))))
            }
        }
    }

    async fn create_dir(&self, path: &str) -> Result<(), Error> {
        block_on_dummy_executor(|| self.0.create_dir(path))
    }
    async fn readdir(
        &self,
        cursor: ReaddirCursor,
    ) -> Result<Box<dyn Iterator<Item = Result<ReaddirEntity, Error>> + Send>, Error> {
        struct I(Box<dyn Iterator<Item = Result<ReaddirEntity, Error>> + Send>);
        impl Iterator for I {
            type Item = Result<ReaddirEntity, Error>;
            fn next(&mut self) -> Option<Self::Item> {
                tokio::task::block_in_place(move || self.0.next())
            }
        }

        let inner = block_on_dummy_executor(move || self.0.readdir(cursor))?;
        Ok(Box::new(I(inner)))
    }

    async fn symlink(&self, src_path: &str, dest_path: &str) -> Result<(), Error> {
        block_on_dummy_executor(move || self.0.symlink(src_path, dest_path))
    }
    async fn remove_dir(&self, path: &str) -> Result<(), Error> {
        block_on_dummy_executor(move || self.0.remove_dir(path))
    }

    async fn unlink_file(&self, path: &str) -> Result<(), Error> {
        block_on_dummy_executor(move || self.0.unlink_file(path))
    }
    async fn read_link(&self, path: &str) -> Result<PathBuf, Error> {
        block_on_dummy_executor(move || self.0.read_link(path))
    }
    async fn get_filestat(&self) -> Result<Filestat, Error> {
        block_on_dummy_executor(|| self.0.get_filestat())
    }
    async fn get_path_filestat(
        &self,
        path: &str,
        follow_symlinks: bool,
    ) -> Result<Filestat, Error> {
        block_on_dummy_executor(move || self.0.get_path_filestat(path, follow_symlinks))
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
            .ok_or(Error::badf().context("failed downcast to tokio Dir"))?;
        block_on_dummy_executor(
            move || async move { self.0.rename_(src_path, &dest_dir.0, dest_path) },
        )
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
            .ok_or(Error::badf().context("failed downcast to tokio Dir"))?;
        block_on_dummy_executor(move || async move {
            self.0.hard_link_(src_path, &target_dir.0, target_path)
        })
    }
    async fn set_times(
        &self,
        path: &str,
        atime: Option<crate::SystemTimeSpec>,
        mtime: Option<crate::SystemTimeSpec>,
        follow_symlinks: bool,
    ) -> Result<(), Error> {
        block_on_dummy_executor(move || self.0.set_times(path, atime, mtime, follow_symlinks))
    }
}

#[cfg(test)]
mod test {
    use super::Dir;
    use crate::file::{FdFlags, OFlags};
    use cap_std::ambient_authority;

    #[tokio::test(flavor = "multi_thread")]
    async fn scratch_dir() {
        let tempdir = tempfile::Builder::new()
            .prefix("cap-std-sync")
            .tempdir()
            .expect("create temporary dir");
        let preopen_dir = cap_std::fs::Dir::open_ambient_dir(tempdir.path(), ambient_authority())
            .expect("open ambient temporary dir");
        let preopen_dir = Dir::from_cap_std(preopen_dir);
        crate::WasiDir::open_file(
            &preopen_dir,
            false,
            ".",
            OFlags::empty(),
            false,
            false,
            FdFlags::empty(),
        )
        .await
        .expect("open the same directory via WasiDir abstraction");
    }

    // Readdir does not work on windows, so we won't test it there.
    #[cfg(not(windows))]
    #[tokio::test(flavor = "multi_thread")]
    async fn readdir() {
        use crate::dir::{ReaddirCursor, ReaddirEntity, WasiDir};
        use crate::file::{FdFlags, FileType, OFlags};
        use std::collections::HashMap;

        async fn readdir_into_map(dir: &dyn WasiDir) -> HashMap<String, ReaddirEntity> {
            let mut out = HashMap::new();
            for readdir_result in dir
                .readdir(ReaddirCursor::from(0))
                .await
                .expect("readdir succeeds")
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

        let entities = readdir_into_map(&preopen_dir).await;
        assert_eq!(
            entities.len(),
            2,
            "should just be . and .. in empty dir: {entities:?}"
        );
        assert!(entities.get(".").is_some());
        assert!(entities.get("..").is_some());

        preopen_dir
            .open_file(
                false,
                "file1",
                OFlags::CREATE,
                true,
                false,
                FdFlags::empty(),
            )
            .await
            .expect("create file1");

        let entities = readdir_into_map(&preopen_dir).await;
        assert_eq!(entities.len(), 3, "should be ., .., file1 {entities:?}");
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
}
