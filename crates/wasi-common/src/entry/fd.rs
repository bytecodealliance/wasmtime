use super::Entry;
use crate::handle::{
    Advice, Fdflags, Fdstat, Filesize, Filestat, Filetype, Fstflags, HandleRights, Prestat,
    PrestatDir, Rights, Size,
};
use crate::sched::Timestamp;
use crate::{Error, Result};
use std::convert::TryInto;
use std::ops::{Deref, DerefMut};

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

    pub fn fd_pread(
        &self,
        mut iovs: Vec<wiggle::GuestSlice<u8>>,
        offset: Filesize,
    ) -> Result<Size> {
        let required_rights = HandleRights::from_base(Rights::FD_READ);
        let mut io_slices = iovs
            .iter_mut()
            .map(|s| std::io::IoSliceMut::new(s.deref_mut()))
            .collect::<Vec<std::io::IoSliceMut>>();
        let host_nread = self
            .as_handle(&required_rights)?
            .preadv(&mut io_slices, offset)?
            .try_into()?;
        Ok(host_nread)
    }

    pub fn fd_prestat_get(&self) -> Result<Prestat> {
        // TODO: should we validate any rights here?
        let po_path = self.preopen_path.as_ref().ok_or(Error::Notsup)?;
        if self.get_file_type() != Filetype::Directory {
            return Err(Error::Notdir);
        }

        let path = crate::path::from_host(po_path.as_os_str())?;
        let prestat = PrestatDir {
            pr_name_len: path.len().try_into()?,
        };
        Ok(Prestat::Dir(prestat))
    }

    pub fn fd_prestat_dir_name(&self, path: &mut [u8]) -> Result<()> {
        // TODO: should we validate any rights here?
        let po_path = self.preopen_path.as_ref().ok_or(Error::Notsup)?;
        if self.get_file_type() != Filetype::Directory {
            return Err(Error::Notdir);
        }

        let host_path = crate::path::from_host(po_path.as_os_str())?;
        let host_path = host_path.as_bytes();
        let host_path_len = host_path.len();

        if host_path_len > path.len() {
            return Err(Error::Nametoolong);
        }

        path[..host_path_len].copy_from_slice(host_path);

        Ok(())
    }

    pub fn fd_pwrite(&self, iovs: Vec<wiggle::GuestSlice<u8>>, offset: Filesize) -> Result<Size> {
        if offset > i64::max_value() as u64 {
            return Err(Error::Io);
        }
        let required_rights = HandleRights::from_base(Rights::FD_WRITE | Rights::FD_SEEK);
        let io_slices = iovs
            .iter()
            .map(|s| std::io::IoSlice::new(s.deref()))
            .collect::<Vec<std::io::IoSlice>>();
        let host_nread = self
            .as_handle(&required_rights)?
            .pwritev(&io_slices, offset)?
            .try_into()?;
        Ok(host_nread)
    }

    pub fn fd_read(&self, mut iovs: Vec<wiggle::GuestSlice<u8>>) -> Result<Size> {
        let required_rights = HandleRights::from_base(Rights::FD_READ);
        let mut io_slices = iovs
            .iter_mut()
            .map(|s| std::io::IoSliceMut::new(s.deref_mut()))
            .collect::<Vec<std::io::IoSliceMut>>();
        let host_nread = self
            .as_handle(&required_rights)?
            .read_vectored(&mut io_slices)?
            .try_into()?;
        Ok(host_nread)
    }
}
