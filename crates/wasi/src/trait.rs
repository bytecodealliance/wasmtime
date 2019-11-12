use std::convert::{TryFrom, TryInto};
use wasi_common::{wasi, wasi32};
use wasmtime_bindings::{AbiPrimitive, WasmMem};
use wasmtime_runtime::{Export, VMContext};

#[wasmtime_trait(module(wasi_mod), context(WasiMem))]
pub trait Wasi {
    fn args_get(
        &self,
        ctx: WasiMem,
        argv: wasi32::uintptr_t,
        argv_buf: wasi32::uintptr_t,
    ) -> WasiResult;

    fn args_sizes_get(
        &self,
        ctx: WasiMem,
        argc: wasi32::uintptr_t,
        argv_buf_size: wasi32::uintptr_t,
    ) -> WasiResult;

    fn clock_res_get(
        &mut self,
        vmctx: *mut VMContext,
        clock_id: wasi::__wasi_clockid_t,
        resolution: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t;

    fn clock_time_get(
        &mut self,
        vmctx: *mut VMContext,
        clock_id: wasi::__wasi_clockid_t,
        precision: wasi::__wasi_timestamp_t,
        time: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t;

    fn environ_get(
        &mut self,
        vmctx: *mut VMContext,
        environ: wasi32::uintptr_t,
        environ_buf: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t;

    fn environ_sizes_get(
        &self,
        ctx: WasiMem,
        environ_count: wasi32::uintptr_t,
        environ_buf_size: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t;

    fn fd_prestat_get(
        &self,
        ctx: WasiMem,
        fd: wasi::__wasi_fd_t,
        buf: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t;

    fn fd_prestat_dir_name(
        &mut self,
        vmctx: *mut VMContext,
        fd: wasi::__wasi_fd_t,
        path: wasi32::uintptr_t,
        path_len: wasi32::size_t,
    ) -> wasi::__wasi_errno_t;

    fn fd_close(&mut self, vmctx: *mut VMContext, fd: wasi::__wasi_fd_t) -> wasi::__wasi_errno_t;

    fn fd_datasync(&mut self, vmctx: *mut VMContext, fd: wasi::__wasi_fd_t)
        -> wasi::__wasi_errno_t;

    fn fd_pread(
        &mut self,
        vmctx: *mut VMContext,
        fd: wasi::__wasi_fd_t,
        iovs: wasi32::uintptr_t,
        iovs_len: wasi32::size_t,
        offset: wasi::__wasi_filesize_t,
        nread: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t;

    fn fd_pwrite(
        &mut self,
        vmctx: *mut VMContext,
        fd: wasi::__wasi_fd_t,
        iovs: wasi32::uintptr_t,
        iovs_len: wasi32::size_t,
        offset: wasi::__wasi_filesize_t,
        nwritten: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t;

    fn fd_read(
        &mut self,
        vmctx: *mut VMContext,
        fd: wasi::__wasi_fd_t,
        iovs: wasi32::uintptr_t,
        iovs_len: wasi32::size_t,
        nread: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t;

    fn fd_renumber(
        &mut self,
        vmctx: *mut VMContext,
        from: wasi::__wasi_fd_t,
        to: wasi::__wasi_fd_t,
    ) -> wasi::__wasi_errno_t;

    fn fd_seek(
        &mut self,
        vmctx: *mut VMContext,
        fd: wasi::__wasi_fd_t,
        offset: wasi::__wasi_filedelta_t,
        whence: wasi::__wasi_whence_t,
        newoffset: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t;

    fn fd_tell(
        &mut self,
        vmctx: *mut VMContext,
        fd: wasi::__wasi_fd_t,
        newoffset: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t;

    fn fd_fdstat_get(
        &mut self,
        vmctx: *mut VMContext,
        fd: wasi::__wasi_fd_t,
        buf: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t;

    fn fd_fdstat_set_flags(
        &mut self,
        vmctx: *mut VMContext,
        fd: wasi::__wasi_fd_t,
        flags: wasi::__wasi_fdflags_t,
    ) -> wasi::__wasi_errno_t;

    fn fd_fdstat_set_rights(
        &mut self,
        vmctx: *mut VMContext,
        fd: wasi::__wasi_fd_t,
        fs_rights_base: wasi::__wasi_rights_t,
        fs_rights_inheriting: wasi::__wasi_rights_t,
    ) -> wasi::__wasi_errno_t;

    fn fd_sync(&mut self, vmctx: *mut VMContext, fd: wasi::__wasi_fd_t) -> wasi::__wasi_errno_t;

    fn fd_write(
        &mut self,
        ctx: WasiMem,
        fd: wasi::__wasi_fd_t,
        iovs: wasi32::uintptr_t,
        iovs_len: wasi32::size_t,
        nwritten: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t;

    fn fd_advise(
        &mut self,
        vmctx: *mut VMContext,
        fd: wasi::__wasi_fd_t,
        offset: wasi::__wasi_filesize_t,
        len: wasi::__wasi_filesize_t,
        advice: wasi::__wasi_advice_t,
    ) -> wasi::__wasi_errno_t;

    fn fd_allocate(
        &mut self,
        vmctx: *mut VMContext,
        fd: wasi::__wasi_fd_t,
        offset: wasi::__wasi_filesize_t,
        len: wasi::__wasi_filesize_t,
    ) -> wasi::__wasi_errno_t;

    fn path_create_directory(
        &mut self,
        vmctx: *mut VMContext,
        fd: wasi::__wasi_fd_t,
        path: wasi32::uintptr_t,
        path_len: wasi32::size_t,
    ) -> wasi::__wasi_errno_t;

    fn path_link(
        &mut self,
        vmctx: *mut VMContext,
        fd0: wasi::__wasi_fd_t,
        flags0: wasi::__wasi_lookupflags_t,
        path0: wasi32::uintptr_t,
        path_len0: wasi32::size_t,
        fd1: wasi::__wasi_fd_t,
        path1: wasi32::uintptr_t,
        path_len1: wasi32::size_t,
    ) -> wasi::__wasi_errno_t;

    fn path_open(
        &mut self,
        vmctx: *mut VMContext,
        dirfd: wasi::__wasi_fd_t,
        dirflags: wasi::__wasi_lookupflags_t,
        path: wasi32::uintptr_t,
        path_len: wasi32::size_t,
        oflags: wasi::__wasi_oflags_t,
        fs_rights_base: wasi::__wasi_rights_t,
        fs_rights_inheriting: wasi::__wasi_rights_t,
        fs_flags: wasi::__wasi_fdflags_t,
        fd: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t;

    fn fd_readdir(
        &mut self,
        vmctx: *mut VMContext,
        fd: wasi::__wasi_fd_t,
        buf: wasi32::uintptr_t,
        buf_len: wasi32::size_t,
        cookie: wasi::__wasi_dircookie_t,
        buf_used: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t;

    fn path_readlink(
        &mut self,
        vmctx: *mut VMContext,
        fd: wasi::__wasi_fd_t,
        path: wasi32::uintptr_t,
        path_len: wasi32::size_t,
        buf: wasi32::uintptr_t,
        buf_len: wasi32::size_t,
        buf_used: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t;

    fn path_rename(
        &mut self,
        vmctx: *mut VMContext,
        fd0: wasi::__wasi_fd_t,
        path0: wasi32::uintptr_t,
        path_len0: wasi32::size_t,
        fd1: wasi::__wasi_fd_t,
        path1: wasi32::uintptr_t,
        path_len1: wasi32::size_t,
    ) -> wasi::__wasi_errno_t;

    fn fd_filestat_get(
        &mut self,
        vmctx: *mut VMContext,
        fd: wasi::__wasi_fd_t,
        buf: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t;

    fn fd_filestat_set_times(
        &mut self,
        vmctx: *mut VMContext,
        fd: wasi::__wasi_fd_t,
        st_atim: wasi::__wasi_timestamp_t,
        st_mtim: wasi::__wasi_timestamp_t,
        fstflags: wasi::__wasi_fstflags_t,
    ) -> wasi::__wasi_errno_t;

    fn fd_filestat_set_size(
        &mut self,
        vmctx: *mut VMContext,
        fd: wasi::__wasi_fd_t,
        size: wasi::__wasi_filesize_t,
    ) -> wasi::__wasi_errno_t;

    fn path_filestat_get(
        &mut self,
        vmctx: *mut VMContext,
        fd: wasi::__wasi_fd_t,
        flags: wasi::__wasi_lookupflags_t,
        path: wasi32::uintptr_t,
        path_len: wasi32::size_t,
        buf: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t;

    fn path_filestat_set_times(
        &mut self,
        vmctx: *mut VMContext,
        fd: wasi::__wasi_fd_t,
        flags: wasi::__wasi_lookupflags_t,
        path: wasi32::uintptr_t,
        path_len: wasi32::size_t,
        st_atim: wasi::__wasi_timestamp_t,
        st_mtim: wasi::__wasi_timestamp_t,
        fstflags: wasi::__wasi_fstflags_t,
    ) -> wasi::__wasi_errno_t;

    fn path_symlink(
        &mut self,
        vmctx: *mut VMContext,
        path0: wasi32::uintptr_t,
        path_len0: wasi32::size_t,
        fd: wasi::__wasi_fd_t,
        path1: wasi32::uintptr_t,
        path_len1: wasi32::size_t,
    ) -> wasi::__wasi_errno_t;

    fn path_unlink_file(
        &mut self,
        vmctx: *mut VMContext,
        fd: wasi::__wasi_fd_t,
        path: wasi32::uintptr_t,
        path_len: wasi32::size_t,
    ) -> wasi::__wasi_errno_t;

    fn path_remove_directory(
        &mut self,
        vmctx: *mut VMContext,
        fd: wasi::__wasi_fd_t,
        path: wasi32::uintptr_t,
        path_len: wasi32::size_t,
    ) -> wasi::__wasi_errno_t;

    fn poll_oneoff(
        &mut self,
        vmctx: *mut VMContext,
        in_: wasi32::uintptr_t,
        out: wasi32::uintptr_t,
        nsubscriptions: wasi32::size_t,
        nevents: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t;

    fn proc_exit(&mut self, _vmctx: *mut VMContext, rval: u32);

    fn proc_raise(
        &mut self,
        vmctx: *mut VMContext,
        sig: wasi::__wasi_signal_t,
    ) -> wasi::__wasi_errno_t;

    fn random_get(
        &mut self,
        vmctx: *mut VMContext,
        buf: wasi32::uintptr_t,
        buf_len: wasi32::size_t,
    ) -> wasi::__wasi_errno_t;

    fn sched_yield(&mut self, _vmctx: *mut VMContext) -> wasi::__wasi_errno_t;

    fn sock_recv(
        &mut self,
        vmctx: *mut VMContext,
        sock: wasi::__wasi_fd_t,
        ri_data: wasi32::uintptr_t,
        ri_data_len: wasi32::size_t,
        ri_flags: wasi::__wasi_riflags_t,
        ro_datalen: wasi32::uintptr_t,
        ro_flags: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t;

    fn sock_send(
        &mut self,
        vmctx: *mut VMContext,
        sock: wasi::__wasi_fd_t,
        si_data: wasi32::uintptr_t,
        si_data_len: wasi32::size_t,
        si_flags: wasi::__wasi_siflags_t,
        so_datalen: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t;

    fn sock_shutdown(
        &mut self,
        vmctx: *mut VMContext,
        sock: wasi::__wasi_fd_t,
        how: wasi::__wasi_sdflags_t,
    ) -> wasi::__wasi_errno_t;
}

pub struct WasiMem(*mut VMContext);

impl WasiMem {
    pub fn from_vmctx(vmctx: *mut VMContext) -> Self {
        WasiMem(vmctx)
    }

    pub fn require_memory<'a>(&self) -> &'a mut [u8] {
        unsafe {
            match (*self.0).lookup_global_export("memory") {
                Some(Export::Memory {
                    definition,
                    vmctx: _,
                    memory: _,
                }) => {
                    std::slice::from_raw_parts_mut((*definition).base, (*definition).current_length)
                }
                x => {
                    panic!(
                        "no export named \"memory\", or the export isn't a mem: {:?}",
                        x
                    );
                }
            }
        }
    }

    pub fn get_memory<'a>(&self) -> Result<&'a mut [u8], wasi::__wasi_errno_t> {
        unsafe {
            match (*self.0).lookup_global_export("memory") {
                Some(Export::Memory {
                    definition,
                    vmctx: _,
                    memory: _,
                }) => Ok(std::slice::from_raw_parts_mut(
                    (*definition).base,
                    (*definition).current_length,
                )),
                x => {
                    eprintln!(
                        "no export named \"memory\", or the export isn't a mem: {:?}",
                        x
                    );
                    Err(wasi::__WASI_ENOTSUP)
                }
            }
        }
    }
}

impl WasmMem for WasiMem {
    type Abi = wasi32::uintptr_t;
    fn as_ptr<T>(&self, off: Self::Abi) -> *mut T {
        let mem = self.require_memory();
        let p = &mut mem[usize::try_from(off).unwrap()];
        p as *mut u8 as *mut T
    }
    fn as_off<T>(&self, ptr: *const T) -> Self::Abi {
        let mem = self.require_memory();
        let ptr = ptr as usize;
        // TODO use offset from
        let offset = (ptr as usize) - (mem.as_ptr() as usize);
        if offset >= mem.len() {
            panic!("offset out of mem boundary");
        }
        offset.try_into().unwrap()
    }
}

pub struct WasiResult(wasi::__wasi_errno_t);

impl AbiPrimitive for WasiResult {
    type Abi = i32;
    fn convert_to_abi(self) -> Self::Abi {
        self.0.try_into().unwrap()
    }
    fn create_from_abi(ret: Self::Abi) -> Self {
        WasiResult(ret.try_into().unwrap())
    }
}

impl From<Result<(), wasi::__wasi_errno_t>> for WasiResult {
    fn from(r: Result<(), wasi::__wasi_errno_t>) -> WasiResult {
        match r {
            Ok(()) => WasiResult(wasi::__WASI_ESUCCESS),
            Err(v) => {
                debug_assert!(v != wasi::__WASI_ESUCCESS);
                WasiResult(v)
            }
        }
    }
}

impl From<wasi::__wasi_errno_t> for WasiResult {
    fn from(r: wasi::__wasi_errno_t) -> Self {
        WasiResult(r)
    }
}

impl Into<wasi::__wasi_errno_t> for WasiResult {
    fn into(self) -> wasi::__wasi_errno_t {
        self.0
    }
}

// TODO detect nightly
#[cfg(nightly)]
impl std::ops::Try for WasiResult {
    type Ok = ();
    type Error = wasi::__wasi_errno_t;
    fn into_result(self) -> Result<Self::Ok, Self::Error> {
        if self.0 == wasi::__WASI_ESUCCESS {
            Ok(())
        } else {
            Err(self.0)
        }
    }
    fn from_error(v: Self::Error) -> Self {
        debug_assert!(v != wasi::__WASI_ESUCCESS);
        WasiResult(v)
    }
    fn from_ok(v: Self::Ok) -> Self {
        WasiResult(wasi::__WASI_ESUCCESS)
    }
}
