#![allow(unused_variables)]

use crate::{wasi_filesystem, WasiCtx};
use wit_bindgen_host_wasmtime_rust::Result as HostResult;

impl wasi_filesystem::WasiFilesystem for WasiCtx {
    fn fadvise(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        offset: wasi_filesystem::Filesize,
        len: wasi_filesystem::Filesize,
        advice: wasi_filesystem::Advice,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        todo!()
    }

    fn fallocate(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        offset: u64,
        len: u64,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        todo!()
    }

    fn datasync(
        &mut self,
        fd: wasi_filesystem::Descriptor,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        todo!()
    }

    fn fd_info(
        &mut self,
        fd: wasi_filesystem::Descriptor,
    ) -> HostResult<wasi_filesystem::Info, wasi_filesystem::Errno> {
        todo!()
    }

    fn set_size(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        size: wasi_filesystem::Filesize,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        todo!()
    }

    fn set_times(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        atim: wasi_filesystem::NewTimestamp,
        mtim: wasi_filesystem::NewTimestamp,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        todo!()
    }

    fn pread(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        len: wasi_filesystem::Size,
        offset: wasi_filesystem::Filesize,
    ) -> HostResult<Vec<u8>, wasi_filesystem::Errno> {
        todo!()
    }

    fn pwrite(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        buf: Vec<u8>,
        offset: wasi_filesystem::Filesize,
    ) -> HostResult<wasi_filesystem::Size, wasi_filesystem::Errno> {
        let out = std::str::from_utf8(&buf)
            .map(|s| s.to_string())
            .unwrap_or_else(|_| format!("binary: {:?}", buf));
        println!("pwrite fd {fd}@{offset}: {out}");
        Ok(buf.len() as u32)
    }

    fn readdir(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        rewind: bool,
    ) -> HostResult<Vec<u8>, wasi_filesystem::Errno> {
        todo!()
    }

    fn seek(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        from: wasi_filesystem::SeekFrom,
    ) -> HostResult<wasi_filesystem::Filesize, wasi_filesystem::Errno> {
        todo!()
    }

    fn sync(&mut self, fd: wasi_filesystem::Descriptor) -> HostResult<(), wasi_filesystem::Errno> {
        todo!()
    }

    fn tell(
        &mut self,
        fd: wasi_filesystem::Descriptor,
    ) -> HostResult<wasi_filesystem::Filesize, wasi_filesystem::Errno> {
        todo!()
    }

    fn create_directory_at(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        path: String,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        todo!()
    }

    fn stat_at(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        at_flags: wasi_filesystem::AtFlags,
        path: String,
    ) -> HostResult<wasi_filesystem::Stat, wasi_filesystem::Errno> {
        todo!()
    }

    fn set_times_at(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        at_flags: wasi_filesystem::AtFlags,
        path: String,
        atim: wasi_filesystem::NewTimestamp,
        mtim: wasi_filesystem::NewTimestamp,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        todo!()
    }

    fn link_at(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        old_at_flags: wasi_filesystem::AtFlags,
        old_path: String,
        new_descriptor: wasi_filesystem::Descriptor,
        new_path: String,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        todo!()
    }

    fn open_at(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        at_flags: wasi_filesystem::AtFlags,
        old_path: String,
        oflags: wasi_filesystem::OFlags,
        flags: wasi_filesystem::Flags,
        mode: wasi_filesystem::Mode,
    ) -> HostResult<wasi_filesystem::Descriptor, wasi_filesystem::Errno> {
        todo!()
    }

    fn readlink_at(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        path: String,
    ) -> HostResult<String, wasi_filesystem::Errno> {
        todo!()
    }

    fn remove_directory_at(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        path: String,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        todo!()
    }

    fn rename_at(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        old_path: String,
        new_fd: wasi_filesystem::Descriptor,
        new_path: String,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        todo!()
    }

    fn symlink_at(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        old_path: String,
        new_path: String,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        todo!()
    }

    fn unlink_file_at(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        path: String,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        todo!()
    }

    fn change_file_permissions_at(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        at_flags: wasi_filesystem::AtFlags,
        path: String,
        mode: wasi_filesystem::Mode,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        todo!()
    }

    fn change_directory_permissions_at(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        at_flags: wasi_filesystem::AtFlags,
        path: String,
        mode: wasi_filesystem::Mode,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        todo!()
    }
}
