use cap_std::Advice;
use std::any::Any;
use std::convert::TryInto;
use std::io;
use wasi_common::{
    file::{FdFlags, FileType, Filestat, WasiFile},
    Error,
};

pub struct File;

impl File {}

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
        todo!()
    }
    fn get_fdflags(&self) -> Result<FdFlags, Error> {
        todo!()
    }
    fn set_fdflags(&mut self, fdflags: FdFlags) -> Result<(), Error> {
        todo!()
    }
    fn get_filestat(&self) -> Result<Filestat, Error> {
        todo!()
    }
    fn set_filestat_size(&self, size: u64) -> Result<(), Error> {
        todo!()
    }
    fn advise(&self, _offset: u64, _len: u64, _advice: Advice) -> Result<(), Error> {
        Ok(())
    }
    fn allocate(&self, offset: u64, len: u64) -> Result<(), Error> {
        todo!()
    }
    fn set_times(
        &self,
        atime: Option<wasi_common::SystemTimeSpec>,
        mtime: Option<wasi_common::SystemTimeSpec>,
    ) -> Result<(), Error> {
        todo!()
    }
    fn read_vectored(&self, bufs: &mut [io::IoSliceMut]) -> Result<u64, Error> {
        todo!()
    }
    fn read_vectored_at(&self, bufs: &mut [io::IoSliceMut], offset: u64) -> Result<u64, Error> {
        todo!()
    }
    fn write_vectored(&self, bufs: &[io::IoSlice]) -> Result<u64, Error> {
        todo!()
    }
    fn write_vectored_at(&self, bufs: &[io::IoSlice], offset: u64) -> Result<u64, Error> {
        todo!()
    }
    fn seek(&self, pos: std::io::SeekFrom) -> Result<u64, Error> {
        todo!()
    }
    fn peek(&self, buf: &mut [u8]) -> Result<u64, Error> {
        todo!()
    }
    fn num_ready_bytes(&self) -> Result<u64, Error> {
        todo!()
    }
}
