#![allow(unused_variables)]

use crate::{wasi_filesystem, WasiCtx};
use std::io::{IoSlice, IoSliceMut};
use wasi_common::file::TableFileExt;
use wit_bindgen_host_wasmtime_rust::Result as HostResult;

fn convert(
    error: wasi_common::Error,
) -> wit_bindgen_host_wasmtime_rust::Error<wasi_filesystem::Errno> {
    if let Some(errno) = error.downcast_ref() {
        use wasi_common::Errno::*;
        use wasi_filesystem::Errno;

        wit_bindgen_host_wasmtime_rust::Error::new(match errno {
            Acces => Errno::Access,
            Addrinuse => Errno::Addrinuse,
            Addrnotavail => Errno::Addrnotavail,
            Afnosupport => Errno::Afnosupport,
            Again => Errno::Again,
            Already => Errno::Already,
            Badf => Errno::Badf,
            Badmsg => Errno::Badmsg,
            Busy => Errno::Busy,
            Canceled => Errno::Canceled,
            Child => Errno::Child,
            Connaborted => Errno::Connaborted,
            Connrefused => Errno::Connrefused,
            Connreset => Errno::Connreset,
            Deadlk => Errno::Deadlk,
            Destaddrreq => Errno::Destaddrreq,
            Dquot => Errno::Dquot,
            Exist => Errno::Exist,
            Fault => Errno::Fault,
            Fbig => Errno::Fbig,
            Hostunreach => Errno::Hostunreach,
            Idrm => Errno::Idrm,
            Ilseq => Errno::Ilseq,
            Inprogress => Errno::Inprogress,
            Intr => Errno::Intr,
            Inval => Errno::Inval,
            Io => Errno::Io,
            Isconn => Errno::Isconn,
            Isdir => Errno::Isdir,
            Loop => Errno::Loop,
            Mfile => Errno::Mfile,
            Mlink => Errno::Mlink,
            Msgsize => Errno::Msgsize,
            Multihop => Errno::Multihop,
            Nametoolong => Errno::Nametoolong,
            Netdown => Errno::Netdown,
            Netreset => Errno::Netreset,
            Netunreach => Errno::Netunreach,
            Nfile => Errno::Nfile,
            Nobufs => Errno::Nobufs,
            Nodev => Errno::Nodev,
            Noent => Errno::Noent,
            Noexec => Errno::Noexec,
            Nolck => Errno::Nolck,
            Nolink => Errno::Nolink,
            Nomem => Errno::Nomem,
            Nomsg => Errno::Nomsg,
            Noprotoopt => Errno::Noprotoopt,
            Nospc => Errno::Nospc,
            Nosys => Errno::Nosys,
            Notdir => Errno::Notdir,
            Notempty => Errno::Notempty,
            Notrecoverable => Errno::Notrecoverable,
            Notsup => Errno::Notsup,
            Notty => Errno::Notty,
            Nxio => Errno::Nxio,
            Overflow => Errno::Overflow,
            Ownerdead => Errno::Ownerdead,
            Perm => Errno::Perm,
            Pipe => Errno::Pipe,
            Range => Errno::Range,
            Rofs => Errno::Rofs,
            Spipe => Errno::Spipe,
            Srch => Errno::Srch,
            Stale => Errno::Stale,
            Timedout => Errno::Timedout,
            Txtbsy => Errno::Txtbsy,
            Xdev => Errno::Xdev,
            Success | Dom | Notcapable | Notsock | Proto | Protonosupport | Prototype | TooBig
            | Notconn => {
                return error.into().into();
            }
        })
    } else {
        error.into().into()
    }
}

#[async_trait::async_trait]
impl wasi_filesystem::WasiFilesystem for WasiCtx {
    async fn fadvise(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        offset: wasi_filesystem::Filesize,
        len: wasi_filesystem::Filesize,
        advice: wasi_filesystem::Advice,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        todo!()
    }

    async fn datasync(
        &mut self,
        fd: wasi_filesystem::Descriptor,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        todo!()
    }

    async fn flags(
        &mut self,
        fd: wasi_filesystem::Descriptor,
    ) -> HostResult<wasi_filesystem::DescriptorFlags, wasi_filesystem::Errno> {
        todo!()
    }

    async fn todo_type(
        &mut self,
        fd: wasi_filesystem::Descriptor,
    ) -> HostResult<wasi_filesystem::DescriptorType, wasi_filesystem::Errno> {
        todo!()
    }

    async fn set_flags(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        flags: wasi_filesystem::DescriptorFlags,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        todo!()
    }

    async fn set_size(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        size: wasi_filesystem::Filesize,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        todo!()
    }

    async fn set_times(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        atim: wasi_filesystem::NewTimestamp,
        mtim: wasi_filesystem::NewTimestamp,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        todo!()
    }

    async fn pread(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        len: wasi_filesystem::Size,
        offset: wasi_filesystem::Filesize,
    ) -> HostResult<Vec<u8>, wasi_filesystem::Errno> {
        let f = self.table().get_file_mut(u32::from(fd)).map_err(convert)?;

        let mut buffer = vec![0; len.try_into().unwrap()];

        let bytes_read = f
            .read_vectored_at(&mut [IoSliceMut::new(&mut buffer)], offset)
            .await
            .map_err(convert)?;

        buffer.truncate(bytes_read.try_into().unwrap());

        Ok(buffer)
    }

    async fn pwrite(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        buf: Vec<u8>,
        offset: wasi_filesystem::Filesize,
    ) -> HostResult<wasi_filesystem::Size, wasi_filesystem::Errno> {
        let f = self.table().get_file_mut(u32::from(fd)).map_err(convert)?;

        let bytes_written = f
            .write_vectored_at(&[IoSlice::new(&buf)], offset)
            .await
            .map_err(convert)?;

        Ok(wasi_filesystem::Size::try_from(bytes_written).unwrap())
    }

    async fn readdir(
        &mut self,
        fd: wasi_filesystem::Descriptor,
    ) -> HostResult<wasi_filesystem::DirEntryStream, wasi_filesystem::Errno> {
        todo!()
    }

    async fn read_dir_entry(
        &mut self,
        stream: wasi_filesystem::DirEntryStream,
    ) -> HostResult<wasi_filesystem::DirEntry, wasi_filesystem::Errno> {
        todo!()
    }

    async fn seek(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        from: wasi_filesystem::SeekFrom,
    ) -> HostResult<wasi_filesystem::Filesize, wasi_filesystem::Errno> {
        todo!()
    }

    async fn sync(
        &mut self,
        fd: wasi_filesystem::Descriptor,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        todo!()
    }

    async fn tell(
        &mut self,
        fd: wasi_filesystem::Descriptor,
    ) -> HostResult<wasi_filesystem::Filesize, wasi_filesystem::Errno> {
        todo!()
    }

    async fn create_directory_at(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        path: String,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        todo!()
    }

    async fn stat(
        &mut self,
        fd: wasi_filesystem::Descriptor,
    ) -> HostResult<wasi_filesystem::DescriptorStat, wasi_filesystem::Errno> {
        todo!()
    }

    async fn stat_at(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        at_flags: wasi_filesystem::AtFlags,
        path: String,
    ) -> HostResult<wasi_filesystem::DescriptorStat, wasi_filesystem::Errno> {
        todo!()
    }

    async fn set_times_at(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        at_flags: wasi_filesystem::AtFlags,
        path: String,
        atim: wasi_filesystem::NewTimestamp,
        mtim: wasi_filesystem::NewTimestamp,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        todo!()
    }

    async fn link_at(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        old_at_flags: wasi_filesystem::AtFlags,
        old_path: String,
        new_descriptor: wasi_filesystem::Descriptor,
        new_path: String,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        todo!()
    }

    async fn open_at(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        at_flags: wasi_filesystem::AtFlags,
        old_path: String,
        oflags: wasi_filesystem::OFlags,
        flags: wasi_filesystem::DescriptorFlags,
        mode: wasi_filesystem::Mode,
    ) -> HostResult<wasi_filesystem::Descriptor, wasi_filesystem::Errno> {
        todo!()
    }

    async fn close(&mut self, fd: wasi_filesystem::Descriptor) -> anyhow::Result<()> {
        todo!()
    }

    async fn readlink_at(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        path: String,
    ) -> HostResult<String, wasi_filesystem::Errno> {
        todo!()
    }

    async fn remove_directory_at(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        path: String,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        todo!()
    }

    async fn rename_at(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        old_path: String,
        new_fd: wasi_filesystem::Descriptor,
        new_path: String,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        todo!()
    }

    async fn symlink_at(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        old_path: String,
        new_path: String,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        todo!()
    }

    async fn unlink_file_at(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        path: String,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        todo!()
    }

    async fn change_file_permissions_at(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        at_flags: wasi_filesystem::AtFlags,
        path: String,
        mode: wasi_filesystem::Mode,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        todo!()
    }

    async fn change_directory_permissions_at(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        at_flags: wasi_filesystem::AtFlags,
        path: String,
        mode: wasi_filesystem::Mode,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        todo!()
    }

    async fn lock_shared(
        &mut self,
        fd: wasi_filesystem::Descriptor,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        todo!()
    }

    async fn lock_exclusive(
        &mut self,
        fd: wasi_filesystem::Descriptor,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        todo!()
    }

    async fn try_lock_shared(
        &mut self,
        fd: wasi_filesystem::Descriptor,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        todo!()
    }

    async fn try_lock_exclusive(
        &mut self,
        fd: wasi_filesystem::Descriptor,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        todo!()
    }

    async fn unlock(
        &mut self,
        fd: wasi_filesystem::Descriptor,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        todo!()
    }
}
