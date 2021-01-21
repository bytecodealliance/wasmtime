use cap_fs_ext::MetadataExt;
use fs_set_times::SetTimes;
use std::any::Any;
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
        Ok(FileType::from(&meta.file_type()))
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
            filetype: FileType::from(&meta.file_type()),
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
}

impl FileIoExt for File {
    fn advise(&self, offset: u64, len: u64, advice: Advice) -> io::Result<()> {
        self.0.advise(offset, len, advice)
    }
    fn allocate(&self, offset: u64, len: u64) -> io::Result<()> {
        self.0.allocate(offset, len)
    }
    fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
    fn read_exact(&self, buf: &mut [u8]) -> io::Result<()> {
        self.0.read_exact(buf)
    }
    fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> {
        self.0.read_at(buf, offset)
    }
    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> io::Result<()> {
        self.0.read_exact_at(buf, offset)
    }
    fn read_vectored(&self, bufs: &mut [io::IoSliceMut]) -> io::Result<usize> {
        self.0.read_vectored(bufs)
    }
    fn read_to_end(&self, buf: &mut Vec<u8>) -> io::Result<usize> {
        self.0.read_to_end(buf)
    }
    fn read_to_string(&self, buf: &mut String) -> io::Result<usize> {
        self.0.read_to_string(buf)
    }
    fn write(&self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }
    fn write_all(&self, buf: &[u8]) -> io::Result<()> {
        self.0.write_all(buf)
    }
    fn write_at(&self, buf: &[u8], offset: u64) -> io::Result<usize> {
        self.0.write_at(buf, offset)
    }
    fn write_all_at(&self, buf: &[u8], offset: u64) -> io::Result<()> {
        self.0.write_all_at(buf, offset)
    }
    fn write_vectored(&self, bufs: &[io::IoSlice]) -> io::Result<usize> {
        self.0.write_vectored(bufs)
    }
    fn write_fmt(&self, fmt: std::fmt::Arguments) -> io::Result<()> {
        self.0.write_fmt(fmt)
    }
    fn flush(&self) -> io::Result<()> {
        self.0.flush()
    }
    fn seek(&self, pos: std::io::SeekFrom) -> io::Result<u64> {
        self.0.seek(pos)
    }
    fn stream_position(&self) -> io::Result<u64> {
        self.0.stream_position()
    }
    fn peek(&self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.peek(buf)
    }
}

impl SetTimes for File {
    fn set_times(
        &self,
        atime: Option<fs_set_times::SystemTimeSpec>,
        mtime: Option<fs_set_times::SystemTimeSpec>,
    ) -> io::Result<()> {
        self.0.set_times(atime, mtime)
    }
}

impl ReadReady for File {
    fn num_ready_bytes(&self) -> io::Result<u64> {
        self.0.num_ready_bytes()
    }
}
