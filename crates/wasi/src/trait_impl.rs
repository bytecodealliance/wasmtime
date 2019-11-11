use crate::r#trait::wasi_mod;
use crate::r#trait::{Wasi, WasiMem, WasiResult};

use log::trace;
use wasi_common::{hostcalls, wasi, wasi32, WasiCtx};
use wasmtime_runtime::VMContext;

pub struct WasiImpl(WasiCtx);

impl WasiImpl {
    fn get_wasi_ctx(&self) -> &WasiCtx {
        &self.0
    }

    fn get_wasi_ctx_mut(&mut self) -> &mut WasiCtx {
        &mut self.0
    }
}

#[allow(unused_variables)]
impl Wasi for WasiImpl {
    fn args_get(
        &self,
        ctx: WasiMem,
        argv: wasi32::uintptr_t,
        argv_buf: wasi32::uintptr_t,
    ) -> WasiResult {
        trace!("args_get(argv={:#x?}, argv_buf={:#x?})", argv, argv_buf,);
        let wasi_ctx = self.get_wasi_ctx();
        let memory = ctx.require_memory();
        unsafe { hostcalls::args_get(wasi_ctx, memory, argv, argv_buf).into() }
    }

    fn args_sizes_get(
        &self,
        ctx: WasiMem,
        argc: wasi32::uintptr_t,
        argv_buf_size: wasi32::uintptr_t,
    ) -> WasiResult {
        trace!(
            "args_sizes_get(argc={:#x?}, argv_buf_size={:#x?})",
            argc,
            argv_buf_size,
        );
        let wasi_ctx = self.get_wasi_ctx();
        let memory = ctx.require_memory();
        unsafe { hostcalls::args_sizes_get(wasi_ctx, memory, argc, argv_buf_size).into() }
    }

    fn clock_res_get(
        &mut self,
        vmctx: *mut VMContext,
        clock_id: wasi::__wasi_clockid_t,
        resolution: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t {
        eprintln!("!!! clock_res_get");
        wasi::__WASI_ENOTSUP
    }

    fn clock_time_get(
        &mut self,
        vmctx: *mut VMContext,
        clock_id: wasi::__wasi_clockid_t,
        precision: wasi::__wasi_timestamp_t,
        time: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t {
        eprintln!("!!! clock_time_get");
        wasi::__WASI_ENOTSUP
    }

    fn environ_get(
        &mut self,
        vmctx: *mut VMContext,
        environ: wasi32::uintptr_t,
        environ_buf: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t {
        eprintln!("!!! environ_get");
        wasi::__WASI_ENOTSUP
    }

    fn environ_sizes_get(
        &self,
        ctx: WasiMem,
        environ_count: wasi32::uintptr_t,
        environ_buf_size: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t {
        trace!(
            "environ_sizes_get(environ_count={:#x?}, environ_buf_size={:#x?})",
            environ_count,
            environ_buf_size,
        );
        let wasi_ctx = self.get_wasi_ctx();
        let memory = ctx.require_memory();
        unsafe { hostcalls::environ_sizes_get(wasi_ctx, memory, environ_count, environ_buf_size) }
    }

    fn fd_prestat_get(
        &self,
        ctx: WasiMem,
        fd: wasi::__wasi_fd_t,
        buf: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t {
        trace!("fd_prestat_get(fd={:?}, buf={:#x?})", fd, buf);
        let wasi_ctx = self.get_wasi_ctx();
        let memory = ctx.require_memory();
        unsafe { hostcalls::fd_prestat_get(wasi_ctx, memory, fd, buf) }
    }

    fn fd_prestat_dir_name(
        &mut self,
        vmctx: *mut VMContext,
        fd: wasi::__wasi_fd_t,
        path: wasi32::uintptr_t,
        path_len: wasi32::size_t,
    ) -> wasi::__wasi_errno_t {
        eprintln!("!!! fd_prestat_dir_name");
        wasi::__WASI_ENOTSUP
    }

    fn fd_close(&mut self, vmctx: *mut VMContext, fd: wasi::__wasi_fd_t) -> wasi::__wasi_errno_t {
        eprintln!("!!! fd_close");
        wasi::__WASI_ENOTSUP
    }

    fn fd_datasync(
        &mut self,
        vmctx: *mut VMContext,
        fd: wasi::__wasi_fd_t,
    ) -> wasi::__wasi_errno_t {
        eprintln!("!!! fd_datasync");
        wasi::__WASI_ENOTSUP
    }

    fn fd_pread(
        &mut self,
        vmctx: *mut VMContext,
        fd: wasi::__wasi_fd_t,
        iovs: wasi32::uintptr_t,
        iovs_len: wasi32::size_t,
        offset: wasi::__wasi_filesize_t,
        nread: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t {
        eprintln!("!!! fd_pread");
        wasi::__WASI_ENOTSUP
    }

    fn fd_pwrite(
        &mut self,
        vmctx: *mut VMContext,
        fd: wasi::__wasi_fd_t,
        iovs: wasi32::uintptr_t,
        iovs_len: wasi32::size_t,
        offset: wasi::__wasi_filesize_t,
        nwritten: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t {
        eprintln!("!!! fd_pwrite");
        wasi::__WASI_ENOTSUP
    }

    fn fd_read(
        &mut self,
        vmctx: *mut VMContext,
        fd: wasi::__wasi_fd_t,
        iovs: wasi32::uintptr_t,
        iovs_len: wasi32::size_t,
        nread: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t {
        eprintln!("!!! fd_read");
        wasi::__WASI_ENOTSUP
    }

    fn fd_renumber(
        &mut self,
        vmctx: *mut VMContext,
        from: wasi::__wasi_fd_t,
        to: wasi::__wasi_fd_t,
    ) -> wasi::__wasi_errno_t {
        eprintln!("!!! fd_renumber");
        wasi::__WASI_ENOTSUP
    }

    fn fd_seek(
        &mut self,
        vmctx: *mut VMContext,
        fd: wasi::__wasi_fd_t,
        offset: wasi::__wasi_filedelta_t,
        whence: wasi::__wasi_whence_t,
        newoffset: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t {
        eprintln!("!!! fd_seek");
        wasi::__WASI_ENOTSUP
    }

    fn fd_tell(
        &mut self,
        vmctx: *mut VMContext,
        fd: wasi::__wasi_fd_t,
        newoffset: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t {
        eprintln!("!!! fd_tell");
        wasi::__WASI_ENOTSUP
    }

    fn fd_fdstat_get(
        &mut self,
        vmctx: *mut VMContext,
        fd: wasi::__wasi_fd_t,
        buf: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t {
        eprintln!("!!! fd_fdstat_get");
        wasi::__WASI_ENOTSUP
    }

    fn fd_fdstat_set_flags(
        &mut self,
        vmctx: *mut VMContext,
        fd: wasi::__wasi_fd_t,
        flags: wasi::__wasi_fdflags_t,
    ) -> wasi::__wasi_errno_t {
        eprintln!("!!! fd_fdstat_set_flags");
        wasi::__WASI_ENOTSUP
    }

    fn fd_fdstat_set_rights(
        &mut self,
        vmctx: *mut VMContext,
        fd: wasi::__wasi_fd_t,
        fs_rights_base: wasi::__wasi_rights_t,
        fs_rights_inheriting: wasi::__wasi_rights_t,
    ) -> wasi::__wasi_errno_t {
        eprintln!("!!! fd_fdstat_set_rights");
        wasi::__WASI_ENOTSUP
    }

    fn fd_sync(&mut self, vmctx: *mut VMContext, fd: wasi::__wasi_fd_t) -> wasi::__wasi_errno_t {
        eprintln!("!!! fd_sync");
        wasi::__WASI_ENOTSUP
    }

    fn fd_write(
        &mut self,
        ctx: WasiMem,
        fd: wasi::__wasi_fd_t,
        iovs: wasi32::uintptr_t,
        iovs_len: wasi32::size_t,
        nwritten: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t {
        trace!(
            "fd_write(fd={:?}, iovs={:#x?}, iovs_len={:?}, nwritten={:#x?})",
            fd,
            iovs,
            iovs_len,
            nwritten
        );
        let wasi_ctx = self.get_wasi_ctx_mut();
        let memory = ctx.require_memory();
        unsafe { hostcalls::fd_write(wasi_ctx, memory, fd, iovs, iovs_len, nwritten) }
    }

    fn fd_advise(
        &mut self,
        vmctx: *mut VMContext,
        fd: wasi::__wasi_fd_t,
        offset: wasi::__wasi_filesize_t,
        len: wasi::__wasi_filesize_t,
        advice: wasi::__wasi_advice_t,
    ) -> wasi::__wasi_errno_t {
        eprintln!("!!! fd_advise");
        wasi::__WASI_ENOTSUP
    }

    fn fd_allocate(
        &mut self,
        vmctx: *mut VMContext,
        fd: wasi::__wasi_fd_t,
        offset: wasi::__wasi_filesize_t,
        len: wasi::__wasi_filesize_t,
    ) -> wasi::__wasi_errno_t {
        eprintln!("!!! fd_allocate");
        wasi::__WASI_ENOTSUP
    }

    fn path_create_directory(
        &mut self,
        vmctx: *mut VMContext,
        fd: wasi::__wasi_fd_t,
        path: wasi32::uintptr_t,
        path_len: wasi32::size_t,
    ) -> wasi::__wasi_errno_t {
        eprintln!("!!! path_create_directory");
        wasi::__WASI_ENOTSUP
    }

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
    ) -> wasi::__wasi_errno_t {
        eprintln!("!!! path_link");
        wasi::__WASI_ENOTSUP
    }

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
    ) -> wasi::__wasi_errno_t {
        eprintln!("!!! path_open");
        wasi::__WASI_ENOTSUP
    }

    fn fd_readdir(
        &mut self,
        vmctx: *mut VMContext,
        fd: wasi::__wasi_fd_t,
        buf: wasi32::uintptr_t,
        buf_len: wasi32::size_t,
        cookie: wasi::__wasi_dircookie_t,
        buf_used: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t {
        eprintln!("!!! fd_readdir");
        wasi::__WASI_ENOTSUP
    }

    fn path_readlink(
        &mut self,
        vmctx: *mut VMContext,
        fd: wasi::__wasi_fd_t,
        path: wasi32::uintptr_t,
        path_len: wasi32::size_t,
        buf: wasi32::uintptr_t,
        buf_len: wasi32::size_t,
        buf_used: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t {
        eprintln!("!!! path_readlink");
        wasi::__WASI_ENOTSUP
    }

    fn path_rename(
        &mut self,
        vmctx: *mut VMContext,
        fd0: wasi::__wasi_fd_t,
        path0: wasi32::uintptr_t,
        path_len0: wasi32::size_t,
        fd1: wasi::__wasi_fd_t,
        path1: wasi32::uintptr_t,
        path_len1: wasi32::size_t,
    ) -> wasi::__wasi_errno_t {
        eprintln!("!!! path_rename");
        wasi::__WASI_ENOTSUP
    }

    fn fd_filestat_get(
        &mut self,
        vmctx: *mut VMContext,
        fd: wasi::__wasi_fd_t,
        buf: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t {
        eprintln!("!!! fd_filestat_get");
        wasi::__WASI_ENOTSUP
    }

    fn fd_filestat_set_times(
        &mut self,
        vmctx: *mut VMContext,
        fd: wasi::__wasi_fd_t,
        st_atim: wasi::__wasi_timestamp_t,
        st_mtim: wasi::__wasi_timestamp_t,
        fstflags: wasi::__wasi_fstflags_t,
    ) -> wasi::__wasi_errno_t {
        eprintln!("!!! fd_filestat_set_times");
        wasi::__WASI_ENOTSUP
    }

    fn fd_filestat_set_size(
        &mut self,
        vmctx: *mut VMContext,
        fd: wasi::__wasi_fd_t,
        size: wasi::__wasi_filesize_t,
    ) -> wasi::__wasi_errno_t {
        eprintln!("!!! fd_filestat_set_size");
        wasi::__WASI_ENOTSUP
    }

    fn path_filestat_get(
        &mut self,
        vmctx: *mut VMContext,
        fd: wasi::__wasi_fd_t,
        flags: wasi::__wasi_lookupflags_t,
        path: wasi32::uintptr_t,
        path_len: wasi32::size_t,
        buf: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t {
        eprintln!("!!! path_filestat_get");
        wasi::__WASI_ENOTSUP
    }

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
    ) -> wasi::__wasi_errno_t {
        eprintln!("!!! path_filestat_set_times");
        wasi::__WASI_ENOTSUP
    }

    fn path_symlink(
        &mut self,
        vmctx: *mut VMContext,
        path0: wasi32::uintptr_t,
        path_len0: wasi32::size_t,
        fd: wasi::__wasi_fd_t,
        path1: wasi32::uintptr_t,
        path_len1: wasi32::size_t,
    ) -> wasi::__wasi_errno_t {
        eprintln!("!!! path_symlink");
        wasi::__WASI_ENOTSUP
    }

    fn path_unlink_file(
        &mut self,
        vmctx: *mut VMContext,
        fd: wasi::__wasi_fd_t,
        path: wasi32::uintptr_t,
        path_len: wasi32::size_t,
    ) -> wasi::__wasi_errno_t {
        eprintln!("!!! path_unlink_file");
        wasi::__WASI_ENOTSUP
    }

    fn path_remove_directory(
        &mut self,
        vmctx: *mut VMContext,
        fd: wasi::__wasi_fd_t,
        path: wasi32::uintptr_t,
        path_len: wasi32::size_t,
    ) -> wasi::__wasi_errno_t {
        eprintln!("!!! path_remove_directory");
        wasi::__WASI_ENOTSUP
    }

    fn poll_oneoff(
        &mut self,
        vmctx: *mut VMContext,
        in_: wasi32::uintptr_t,
        out: wasi32::uintptr_t,
        nsubscriptions: wasi32::size_t,
        nevents: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t {
        eprintln!("!!! poll_oneoff");
        wasi::__WASI_ENOTSUP
    }

    fn proc_exit(&mut self, _vmctx: *mut VMContext, rval: u32) {}

    fn proc_raise(
        &mut self,
        vmctx: *mut VMContext,
        sig: wasi::__wasi_signal_t,
    ) -> wasi::__wasi_errno_t {
        eprintln!("!!! proc_exit");
        wasi::__WASI_ENOTSUP
    }

    fn random_get(
        &mut self,
        vmctx: *mut VMContext,
        buf: wasi32::uintptr_t,
        buf_len: wasi32::size_t,
    ) -> wasi::__wasi_errno_t {
        eprintln!("!!! random_get");
        wasi::__WASI_ENOTSUP
    }

    fn sched_yield(&mut self, _vmctx: *mut VMContext) -> wasi::__wasi_errno_t {
        eprintln!("!!! sched_yield");
        wasi::__WASI_ENOTSUP
    }

    fn sock_recv(
        &mut self,
        vmctx: *mut VMContext,
        sock: wasi::__wasi_fd_t,
        ri_data: wasi32::uintptr_t,
        ri_data_len: wasi32::size_t,
        ri_flags: wasi::__wasi_riflags_t,
        ro_datalen: wasi32::uintptr_t,
        ro_flags: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t {
        eprintln!("!!! sock_recv");
        wasi::__WASI_ENOTSUP
    }

    fn sock_send(
        &mut self,
        vmctx: *mut VMContext,
        sock: wasi::__wasi_fd_t,
        si_data: wasi32::uintptr_t,
        si_data_len: wasi32::size_t,
        si_flags: wasi::__wasi_siflags_t,
        so_datalen: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t {
        eprintln!("!!! sock_send");
        wasi::__WASI_ENOTSUP
    }

    fn sock_shutdown(
        &mut self,
        vmctx: *mut VMContext,
        sock: wasi::__wasi_fd_t,
        how: wasi::__wasi_sdflags_t,
    ) -> wasi::__wasi_errno_t {
        eprintln!("!!! sock_shutdown");
        wasi::__WASI_ENOTSUP
    }
}

use cranelift_entity::PrimaryMap;
use cranelift_wasm::DefinedFuncIndex;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::rc::Rc;
use wasi_common::WasiCtxBuilder;
use wasmtime_bindings::FnMetadata;
use wasmtime_environ::{Export, Module};
use wasmtime_runtime::{Imports, InstanceHandle, InstantiationError, VMFunctionBody};

pub fn instantiate_wasi2(
    _prefix: &str,
    global_exports: Rc<RefCell<HashMap<String, Option<wasmtime_runtime::Export>>>>,
    preopened_dirs: &[(String, File)],
    argv: &[String],
    environ: &[(String, String)],
) -> Result<InstanceHandle, InstantiationError> {
    let mut module = Module::new();
    let mut finished_functions: PrimaryMap<DefinedFuncIndex, *const VMFunctionBody> =
        PrimaryMap::new();

    for FnMetadata {
        name,
        signature,
        address,
    } in wasi_mod::metadata().into_iter()
    {
        let sig_id = module.signatures.push(signature);
        let func_id = module.functions.push(sig_id);
        module
            .exports
            .insert(name.to_string(), Export::Function(func_id));

        finished_functions.push(address as *const VMFunctionBody);
    }

    let imports = Imports::none();
    let data_initializers = Vec::new();
    let signatures = PrimaryMap::new();

    let mut wasi_ctx_builder = WasiCtxBuilder::new()
        .inherit_stdio()
        .args(argv)
        .envs(environ);

    for (dir, f) in preopened_dirs {
        wasi_ctx_builder = wasi_ctx_builder.preopened_dir(
            f.try_clone().map_err(|err| {
                InstantiationError::Resource(format!(
                    "couldn't clone an instance handle to pre-opened dir: {}",
                    err
                ))
            })?,
            dir,
        );
    }

    let wasi_ctx = wasi_ctx_builder.build().map_err(|err| {
        InstantiationError::Resource(format!("couldn't assemble WASI context object: {}", err))
    })?;
    let wasi_state = wasi_mod::State {
        subject: ::std::cell::RefCell::new(::std::boxed::Box::new(WasiImpl(wasi_ctx))),
    };

    InstanceHandle::new(
        Rc::new(module),
        global_exports,
        finished_functions.into_boxed_slice(),
        imports,
        &data_initializers,
        signatures.into_boxed_slice(),
        None,
        Box::new(wasi_state),
    )
}
