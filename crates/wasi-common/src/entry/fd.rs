use super::Entry;
use crate::handle::{
    Advice, Fdflags, Fdstat, Filesize, Filestat, Fstflags, HandleRights, Rights, Size,
};
use crate::sched::Timestamp;
use crate::{Error, Result};
use std::convert::TryInto;

impl Entry {
    pub fn fd_advise(&self, offset: Filesize, len: Filesize, advice: Advice) -> Result<()> {
        let required_rights = HandleRights::from_base(Rights::FD_ADVISE);
        self.as_handle(&required_rights)?
            .advise(advice, offset, len)
    }

    pub fn fd_allocate(&self, offset: Filesize, len: Filesize) -> Result<()> {
        let required_rights = HandleRights::from_base(Rights::FD_ALLOCATE);
        self.as_handle(&required_rights)?.allocate(offset, len)
    }

    pub fn fd_close(&self) -> Result<()> {
        // can't close preopened files
        if self.preopen_path.is_some() {
            return Err(Error::Notsup);
        }
        Ok(())
    }

    pub fn fd_datasync(&self) -> Result<()> {
        let required_rights = HandleRights::from_base(Rights::FD_DATASYNC);
        self.as_handle(&required_rights)?.datasync()
    }

    pub fn fd_fdstat_get(&self) -> Result<Fdstat> {
        let required_rights = HandleRights::empty();
        let file = self.as_handle(&required_rights)?;
        let fs_flags = file.fdstat_get()?;
        let rights = self.get_rights();
        let fdstat = Fdstat {
            fs_filetype: self.get_file_type(),
            fs_rights_base: rights.base,
            fs_rights_inheriting: rights.inheriting,
            fs_flags,
        };
        Ok(fdstat)
    }

    pub fn fd_fdstat_set_flags(&self, flags: Fdflags) -> Result<()> {
        let required_rights = HandleRights::from_base(Rights::FD_FDSTAT_SET_FLAGS);
        self.as_handle(&required_rights)?.fdstat_set_flags(flags)
    }

    pub fn fd_fdstat_set_rights(
        &self,
        fs_rights_base: Rights,
        fs_rights_inheriting: Rights,
    ) -> Result<()> {
        let rights = HandleRights::new(fs_rights_base, fs_rights_inheriting);
        if !self.get_rights().contains(&rights) {
            return Err(Error::Notcapable);
        }
        self.set_rights(rights);
        Ok(())
    }

    pub fn fd_filestat_get(&self) -> Result<Filestat> {
        let required_rights = HandleRights::from_base(Rights::FD_FILESTAT_GET);
        let host_filestat = self.as_handle(&required_rights)?.filestat_get()?;
        Ok(host_filestat)
    }

    pub fn fd_filestat_set_size(&self, size: Filesize) -> Result<()> {
        let required_rights = HandleRights::from_base(Rights::FD_FILESTAT_SET_SIZE);
        self.as_handle(&required_rights)?.filestat_set_size(size)
    }

    pub fn fd_filestat_set_times(
        &self,
        atim: Timestamp,
        mtim: Timestamp,
        fst_flags: Fstflags,
    ) -> Result<()> {
        let required_rights = HandleRights::from_base(Rights::FD_FILESTAT_SET_TIMES);
        self.as_handle(&required_rights)?
            .filestat_set_times(atim, mtim, fst_flags)
    }

    pub fn fd_pread(&self, iovs: &mut [std::io::IoSliceMut], offset: Filesize) -> Result<Size> {
        let required_rights = HandleRights::from_base(Rights::FD_READ);
        let host_nread = self
            .as_handle(&required_rights)?
            .read_vectored(iovs)?
            .try_into()?;
        Ok(host_nread)
    }
}
