use crate::preview2::{
    block_in_place, HostInputStream, HostOutputStream, StreamState, Table, TableError,
};
use std::sync::Arc;
use bytes::Bytes;

bitflags::bitflags! {
    pub struct FilePerms: usize {
        const READ = 0b1;
        const WRITE = 0b10;
    }
}

pub(crate) struct File {
    pub file: Arc<cap_std::fs::File>,
    pub perms: FilePerms,
}

impl File {
    pub fn new(file: cap_std::fs::File, perms: FilePerms) -> Self {
        Self {
            file: Arc::new(file),
            perms,
        }
    }
}
pub(crate) trait TableFsExt {
    fn push_file(&mut self, file: File) -> Result<u32, TableError>;
    fn delete_file(&mut self, fd: u32) -> Result<File, TableError>;
    fn is_file(&self, fd: u32) -> bool;
    fn get_file(&self, fd: u32) -> Result<&File, TableError>;

    fn push_dir(&mut self, dir: Dir) -> Result<u32, TableError>;
    fn delete_dir(&mut self, fd: u32) -> Result<Dir, TableError>;
    fn is_dir(&self, fd: u32) -> bool;
    fn get_dir(&self, fd: u32) -> Result<&Dir, TableError>;
}

impl TableFsExt for Table {
    fn push_file(&mut self, file: File) -> Result<u32, TableError> {
        self.push(Box::new(file))
    }
    fn delete_file(&mut self, fd: u32) -> Result<File, TableError> {
        self.delete(fd)
    }
    fn is_file(&self, fd: u32) -> bool {
        self.is::<File>(fd)
    }
    fn get_file(&self, fd: u32) -> Result<&File, TableError> {
        self.get(fd)
    }

    fn push_dir(&mut self, dir: Dir) -> Result<u32, TableError> {
        self.push(Box::new(dir))
    }
    fn delete_dir(&mut self, fd: u32) -> Result<Dir, TableError> {
        self.delete(fd)
    }
    fn is_dir(&self, fd: u32) -> bool {
        self.is::<Dir>(fd)
    }
    fn get_dir(&self, fd: u32) -> Result<&Dir, TableError> {
        self.get(fd)
    }
}

bitflags::bitflags! {
    pub struct DirPerms: usize {
        const READ = 0b1;
        const MUTATE = 0b10;
    }
}

pub(crate) struct Dir {
    pub dir: cap_std::fs::Dir,
    pub perms: DirPerms,
    pub file_perms: FilePerms,
}

impl Dir {
    pub fn new(dir: cap_std::fs::Dir, perms: DirPerms, file_perms: FilePerms) -> Self {
        Dir {
            dir,
            perms,
            file_perms,
        }
    }
}

pub(crate) struct FileInputStream {
    file: Arc<cap_std::fs::File>,
    position: u64,
}
impl FileInputStream {
    pub fn new(file: Arc<cap_std::fs::File>, position: u64) -> Self {
        Self { file, position }
    }
}

#[async_trait::async_trait]
impl HostInputStream for FileInputStream {
    fn read(&mut self) -> anyhow::Result<(Bytes, StreamState)> {
        // use system_interface::fs::FileIoExt;
        // let (n, end) = read_result(block_in_place(|| self.file.read_at(buf, self.position)))?;
        // self.position = self.position.wrapping_add(n);
        // Ok((n, end))
        todo!()
    }
    async fn ready(&mut self) -> anyhow::Result<()> {
        Ok(()) // Always immediately ready - file reads cannot block
    }
}

pub(crate) fn read_result(
    r: Result<usize, std::io::Error>,
) -> Result<(u64, StreamState), std::io::Error> {
    match r {
        Ok(0) => Ok((0, StreamState::Closed)),
        Ok(n) => Ok((n as u64, StreamState::Open)),
        Err(e) if e.kind() == std::io::ErrorKind::Interrupted => Ok((0, StreamState::Open)),
        Err(e) => Err(e),
    }
}

pub(crate) struct FileOutputStream {
    file: Arc<cap_std::fs::File>,
    position: u64,
}
impl FileOutputStream {
    pub fn new(file: Arc<cap_std::fs::File>, position: u64) -> Self {
        Self { file, position }
    }
}

#[async_trait::async_trait]
impl HostOutputStream for FileOutputStream {
    /// Write bytes. On success, returns the number of bytes written.
    fn write(&mut self, buf: Bytes) -> anyhow::Result<u64> {
        // use system_interface::fs::FileIoExt;
        // let n = block_in_place(|| self.file.write_at(buf, self.position))? as i64 as u64;
        // self.position = self.position.wrapping_add(n);
        // Ok(n)
        todo!()
    }

    async fn ready(&mut self) -> anyhow::Result<()> {
        Ok(()) // Always immediately ready - file writes cannot block
    }
}

pub(crate) struct FileAppendStream {
    file: Arc<cap_std::fs::File>,
}
impl FileAppendStream {
    pub fn new(file: Arc<cap_std::fs::File>) -> Self {
        Self { file }
    }
}

#[async_trait::async_trait]
impl HostOutputStream for FileAppendStream {
    /// Write bytes. On success, returns the number of bytes written.
    fn write(&mut self, buf: Bytes) -> anyhow::Result<u64> {
        // use system_interface::fs::FileIoExt;
        // Ok(block_in_place(|| self.file.append(buf))? as i64 as u64)
        todo!()
    }

    async fn ready(&mut self) -> anyhow::Result<()> {
        Ok(()) // Always immediately ready - file appends cannot block
    }
}
