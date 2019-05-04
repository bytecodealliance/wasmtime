use cranelift_codegen::ir::types::{Type, I32, I64};
use wasi_common::{host, hostcalls, memory, wasm32, WasiCtx};
use wasmtime_runtime::{Export, VMContext};

fn return_encoded_errno(e: host::__wasi_errno_t) -> wasm32::__wasi_errno_t {
    let errno = memory::enc_errno(e);
    trace!("    -> errno={}", wasm32::strerror(errno));
    errno
}

fn get_wasi_ctx(vmctx: &mut VMContext) -> Result<&mut WasiCtx, host::__wasi_errno_t> {
    unsafe {
        vmctx.host_state().downcast_mut::<WasiCtx>().ok_or_else(|| {
            println!("!!! no host state named WasiCtx available");
            host::__WASI_EINVAL
        })
    }
}

fn get_memory(vmctx: &mut VMContext) -> Result<&mut [u8], host::__wasi_errno_t> {
    unsafe {
        match vmctx.lookup_global_export("memory") {
            Some(Export::Memory {
                definition,
                vmctx: _,
                memory: _,
            }) => Ok(std::slice::from_raw_parts_mut(
                (*definition).base,
                (*definition).current_length,
            )),
            x => {
                println!(
                    "!!! no export named __wasi_memory, or the export isn't a mem: {:?}",
                    x
                );
                Err(host::__WASI_EINVAL)
            }
        }
    }
}

pub trait AbiRet {
    type Abi;
    fn convert(self) -> Self::Abi;
    fn codegen_tys() -> Vec<Type>;
}

pub trait AbiParam {
    type Abi;
    fn convert(arg: Self::Abi) -> Self;
    fn codegen_ty() -> Type;
}

macro_rules! cast32 {
    ($($i:ident)*) => ($(
        impl AbiRet for $i {
            type Abi = i32;

            fn convert(self) -> Self::Abi {
                self as i32
            }

            fn codegen_tys() -> Vec<Type> { vec![I32] }
        }

        impl AbiParam for $i {
            type Abi = i32;

            fn convert(param: i32) -> Self {
                param as $i
            }

            fn codegen_ty() -> Type { I32 }
        }
    )*)
}

macro_rules! cast64 {
    ($($i:ident)*) => ($(
        impl AbiRet for $i {
            type Abi = i64;

            fn convert(self) -> Self::Abi {
                self as i64
            }

            fn codegen_tys() -> Vec<Type> { vec![I64] }
        }

        impl AbiParam for $i {
            type Abi = i64;

            fn convert(param: i64) -> Self {
                param as $i
            }

            fn codegen_ty() -> Type { I64 }
        }
    )*)
}

cast32!(i8 i16 i32 u8 u16 u32);
cast64!(i64 u64);

impl AbiRet for () {
    type Abi = ();
    fn convert(self) {}
    fn codegen_tys() -> Vec<Type> {
        Vec::new()
    }
}

macro_rules! syscalls {
    ($(pub unsafe extern "C" fn $name:ident($ctx:ident: *mut VMContext $(, $arg:ident: $ty:ty)*,) -> $ret:ty {
        $($body:tt)*
    })*) => ($(
        pub mod $name {
            use super::*;

            /// Returns the codegen types of all the parameters to the shim
            /// generated
            pub fn params() -> Vec<Type> {
                vec![$(<$ty as AbiParam>::codegen_ty()),*]
            }

            /// Returns the codegen types of all the results of the shim
            /// generated
            pub fn results() -> Vec<Type> {
                <$ret as AbiRet>::codegen_tys()
            }

            /// The actual function pointer to the shim for a syscall.
            ///
            /// NB: ideally we'd expose `shim` below, but it seems like there's
            /// a compiler bug which prvents that from being cast to a `usize`.
            pub static SHIM: unsafe extern "C" fn(
                *mut VMContext,
                $(<$ty as AbiParam>::Abi),*
            ) -> <$ret as AbiRet>::Abi = shim;

            unsafe extern "C" fn shim(
                $ctx: *mut VMContext,
                $($arg: <$ty as AbiParam>::Abi,)*
            ) -> <$ret as AbiRet>::Abi {
                let r = super::$name($ctx, $(<$ty as AbiParam>::convert($arg),)*);
                <$ret as AbiRet>::convert(r)
            }
        }

        pub unsafe extern "C" fn $name($ctx: *mut VMContext, $($arg: $ty,)*) -> $ret {
            $($body)*
        }
    )*)
}

syscalls! {
    pub unsafe extern "C" fn args_get(
        vmctx: *mut VMContext,
        argv: wasm32::uintptr_t,
        argv_buf: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t {
        trace!(
            "args_get(argv={:#x?}, argv_buf={:#x?})",
            argv,
            argv_buf,
        );

        let wasi_ctx = match get_wasi_ctx(&mut *vmctx) {
            Ok(ctx) => ctx,
            Err(e) => return return_encoded_errno(e),
        };

        let memory = match get_memory(&mut *vmctx) {
            Ok(memory) => memory,
            Err(e) => return return_encoded_errno(e),
        };

        hostcalls::args_get(wasi_ctx, memory, argv, argv_buf)
    }

    pub unsafe extern "C" fn args_sizes_get(
        vmctx: *mut VMContext,
        argc: wasm32::uintptr_t,
        argv_buf_size: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t {
        trace!(
            "args_sizes_get(argc={:#x?}, argv_buf_size={:#x?})",
            argc,
            argv_buf_size,
        );

        let wasi_ctx = match get_wasi_ctx(&mut *vmctx) {
            Ok(ctx) => ctx,
            Err(e) => return return_encoded_errno(e),
        };

        let memory = match get_memory(&mut *vmctx) {
            Ok(memory) => memory,
            Err(e) => return return_encoded_errno(e),
        };

        hostcalls::args_sizes_get(wasi_ctx, memory, argc, argv_buf_size)
    }

    pub unsafe extern "C" fn clock_res_get(
        vmctx: *mut VMContext,
        clock_id: wasm32::__wasi_clockid_t,
        resolution: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t {
        trace!(
            "clock_res_get(clock_id={:?}, resolution={:#x?})",
            clock_id,
            resolution,
        );

        let memory = match get_memory(&mut *vmctx) {
            Ok(memory) => memory,
            Err(e) => return return_encoded_errno(e),
        };

        hostcalls::clock_res_get(memory, clock_id, resolution)
    }

    pub unsafe extern "C" fn clock_time_get(
        vmctx: *mut VMContext,
        clock_id: wasm32::__wasi_clockid_t,
        precision: wasm32::__wasi_timestamp_t,
        time: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t {
        trace!(
            "clock_time_get(clock_id={:?}, precision={:?}, time={:#x?})",
            clock_id,
            precision,
            time,
        );

        let memory = match get_memory(&mut *vmctx) {
            Ok(memory) => memory,
            Err(e) => return return_encoded_errno(e),
        };

        hostcalls::clock_time_get(memory, clock_id, precision, time)
    }

    pub unsafe extern "C" fn environ_get(
        vmctx: *mut VMContext,
        environ: wasm32::uintptr_t,
        environ_buf: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t {
        trace!(
            "environ_get(environ={:#x?}, environ_buf={:#x?})",
            environ,
            environ_buf,
        );

        let wasi_ctx = match get_wasi_ctx(&mut *vmctx) {
            Ok(ctx) => ctx,
            Err(e) => return return_encoded_errno(e),
        };

        let memory = match get_memory(&mut *vmctx) {
            Ok(memory) => memory,
            Err(e) => return return_encoded_errno(e),
        };

        hostcalls::environ_get(wasi_ctx, memory, environ, environ_buf)
    }

    pub unsafe extern "C" fn environ_sizes_get(
        vmctx: *mut VMContext,
        environ_count: wasm32::uintptr_t,
        environ_buf_size: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t {
        trace!(
            "environ_sizes_get(environ_count={:#x?}, environ_buf_size={:#x?})",
            environ_count,
            environ_buf_size,
        );

        let wasi_ctx = match get_wasi_ctx(&mut *vmctx) {
            Ok(ctx) => ctx,
            Err(e) => return return_encoded_errno(e),
        };

        let memory = match get_memory(&mut *vmctx) {
            Ok(memory) => memory,
            Err(e) => return return_encoded_errno(e),
        };

        hostcalls::environ_sizes_get(wasi_ctx, memory, environ_count, environ_buf_size)
    }

    pub unsafe extern "C" fn fd_prestat_get(
        vmctx: *mut VMContext,
        fd: wasm32::__wasi_fd_t,
        buf: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t {
        trace!("fd_prestat_get(fd={:?}, buf={:#x?})", fd, buf);

        let wasi_ctx = match get_wasi_ctx(&mut *vmctx) {
            Ok(ctx) => ctx,
            Err(e) => return return_encoded_errno(e),
        };

        let memory = match get_memory(&mut *vmctx) {
            Ok(memory) => memory,
            Err(e) => return return_encoded_errno(e),
        };

        hostcalls::fd_prestat_get(wasi_ctx, memory, fd, buf)
    }

    pub unsafe extern "C" fn fd_prestat_dir_name(
        vmctx: *mut VMContext,
        fd: wasm32::__wasi_fd_t,
        path: wasm32::uintptr_t,
        path_len: wasm32::size_t,
    ) -> wasm32::__wasi_errno_t {
        trace!("fd_prestat_dir_name(fd={:?}, path={:#x?}, path_len={})", fd, path, path_len);

        let wasi_ctx = match get_wasi_ctx(&mut *vmctx) {
            Ok(ctx) => ctx,
            Err(e) => return return_encoded_errno(e),
        };

        let memory = match get_memory(&mut *vmctx) {
            Ok(memory) => memory,
            Err(e) => return return_encoded_errno(e),
        };

        hostcalls::fd_prestat_dir_name(wasi_ctx, memory, fd, path, path_len)
    }

    pub unsafe extern "C" fn fd_close(
        vmctx: *mut VMContext,
        fd: wasm32::__wasi_fd_t,
    ) -> wasm32::__wasi_errno_t {
        trace!("fd_close(fd={:?})", fd);

        let wasi_ctx = match get_wasi_ctx(&mut *vmctx) {
            Ok(ctx) => ctx,
            Err(e) => return return_encoded_errno(e),
        };

        hostcalls::fd_close(wasi_ctx, fd)
    }

    pub unsafe extern "C" fn fd_datasync(
        vmctx: *mut VMContext,
        fd: wasm32::__wasi_fd_t,
    ) -> wasm32::__wasi_errno_t {
        trace!("fd_datasync(fd={:?})", fd);

        let wasi_ctx = match get_wasi_ctx(&mut *vmctx) {
            Ok(ctx) => ctx,
            Err(e) => return return_encoded_errno(e),
        };

        let memory = match get_memory(&mut *vmctx) {
            Ok(memory) => memory,
            Err(e) => return return_encoded_errno(e),
        };

        hostcalls::fd_datasync(wasi_ctx, memory, fd)
    }

    pub unsafe extern "C" fn fd_pread(
        vmctx: *mut VMContext,
        fd: wasm32::__wasi_fd_t,
        iovs: wasm32::uintptr_t,
        iovs_len: wasm32::size_t,
        offset: wasm32::__wasi_filesize_t,
        nread: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t {
        trace!(
            "fd_pread(fd={:?}, iovs={:#x?}, iovs_len={:?}, offset={}, nread={:#x?})",
            fd,
            iovs,
            iovs_len,
            offset,
            nread
        );

        let wasi_ctx = match get_wasi_ctx(&mut *vmctx) {
            Ok(ctx) => ctx,
            Err(e) => return return_encoded_errno(e),
        };

        let memory = match get_memory(&mut *vmctx) {
            Ok(memory) => memory,
            Err(e) => return return_encoded_errno(e),
        };

        hostcalls::fd_pread(
            wasi_ctx,
            memory,
            fd,
            iovs,
            iovs_len,
            offset,
            nread
        )
    }

    pub unsafe extern "C" fn fd_pwrite(
        vmctx: *mut VMContext,
        fd: wasm32::__wasi_fd_t,
        iovs: wasm32::uintptr_t,
        iovs_len: wasm32::size_t,
        offset: wasm32::__wasi_filesize_t,
        nwritten: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t {
        trace!(
            "fd_pwrite(fd={:?}, iovs={:#x?}, iovs_len={:?}, offset={}, nwritten={:#x?})",
            fd,
            iovs,
            iovs_len,
            offset,
            nwritten
        );

        let wasi_ctx = match get_wasi_ctx(&mut *vmctx) {
            Ok(ctx) => ctx,
            Err(e) => return return_encoded_errno(e),
        };

        let memory = match get_memory(&mut *vmctx) {
            Ok(memory) => memory,
            Err(e) => return return_encoded_errno(e),
        };

        hostcalls::fd_pwrite(
            wasi_ctx,
            memory,
            fd,
            iovs,
            iovs_len,
            offset,
            nwritten
        )
    }

    pub unsafe extern "C" fn fd_read(
        vmctx: *mut VMContext,
        fd: wasm32::__wasi_fd_t,
        iovs: wasm32::uintptr_t,
        iovs_len: wasm32::size_t,
        nread: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t {
        trace!(
            "fd_read(fd={:?}, iovs={:#x?}, iovs_len={:?}, nread={:#x?})",
            fd,
            iovs,
            iovs_len,
            nread
        );

        let wasi_ctx = match get_wasi_ctx(&mut *vmctx) {
            Ok(ctx) => ctx,
            Err(e) => return return_encoded_errno(e),
        };

        let memory = match get_memory(&mut *vmctx) {
            Ok(memory) => memory,
            Err(e) => return return_encoded_errno(e),
        };

        hostcalls::fd_read(wasi_ctx, memory, fd, iovs, iovs_len, nread)
    }

    pub unsafe extern "C" fn fd_renumber(
        vmctx: *mut VMContext,
        from: wasm32::__wasi_fd_t,
        to: wasm32::__wasi_fd_t,
    ) -> wasm32::__wasi_errno_t {
        trace!("fd_renumber(from={:?}, to={:?})", from, to);

        let wasi_ctx = match get_wasi_ctx(&mut *vmctx) {
            Ok(ctx) => ctx,
            Err(e) => return return_encoded_errno(e),
        };

        let memory = match get_memory(&mut *vmctx) {
            Ok(memory) => memory,
            Err(e) => return return_encoded_errno(e),
        };

        hostcalls::fd_renumber(wasi_ctx, memory, from, to)
    }

    pub unsafe extern "C" fn fd_seek(
        vmctx: *mut VMContext,
        fd: wasm32::__wasi_fd_t,
        offset: wasm32::__wasi_filedelta_t,
        whence: wasm32::__wasi_whence_t,
        newoffset: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t {
        trace!(
            "fd_seek(fd={:?}, offset={:?}, whence={}, newoffset={:#x?})",
            fd,
            offset,
            wasm32::whence_to_str(whence),
            newoffset
        );

        let wasi_ctx = match get_wasi_ctx(&mut *vmctx) {
            Ok(ctx) => ctx,
            Err(e) => return return_encoded_errno(e),
        };

        let memory = match get_memory(&mut *vmctx) {
            Ok(memory) => memory,
            Err(e) => return return_encoded_errno(e),
        };

        hostcalls::fd_seek(wasi_ctx, memory, fd, offset, whence, newoffset)
    }

    pub unsafe extern "C" fn fd_tell(
        vmctx: *mut VMContext,
        fd: wasm32::__wasi_fd_t,
        newoffset: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t {
        trace!("fd_tell(fd={:?}, newoffset={:#x?})", fd, newoffset);

        let wasi_ctx = match get_wasi_ctx(&mut *vmctx) {
            Ok(ctx) => ctx,
            Err(e) => return return_encoded_errno(e),
        };

        let memory = match get_memory(&mut *vmctx) {
            Ok(memory) => memory,
            Err(e) => return return_encoded_errno(e),
        };

        hostcalls::fd_tell(wasi_ctx, memory, fd, newoffset)
    }

    pub unsafe extern "C" fn fd_fdstat_get(
        vmctx: *mut VMContext,
        fd: wasm32::__wasi_fd_t,
        buf: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t {
        trace!("fd_fdstat_get(fd={:?}, buf={:#x?})", fd, buf);

        let wasi_ctx = match get_wasi_ctx(&mut *vmctx) {
            Ok(ctx) => ctx,
            Err(e) => return return_encoded_errno(e),
        };

        let memory = match get_memory(&mut *vmctx) {
            Ok(memory) => memory,
            Err(e) => return return_encoded_errno(e),
        };

        hostcalls::fd_fdstat_get(wasi_ctx, memory, fd, buf)
    }

    pub unsafe extern "C" fn fd_fdstat_set_flags(
        vmctx: *mut VMContext,
        fd: wasm32::__wasi_fd_t,
        flags: wasm32::__wasi_fdflags_t,
    ) -> wasm32::__wasi_errno_t {
        trace!(
            "fd_fdstat_set_flags(fd={:?}, flags={:#x?})",
            fd,
            flags
        );

        let wasi_ctx = match get_wasi_ctx(&mut *vmctx) {
            Ok(ctx) => ctx,
            Err(e) => return return_encoded_errno(e),
        };

        hostcalls::fd_fdstat_set_flags(wasi_ctx, fd, flags)
    }

    pub unsafe extern "C" fn fd_fdstat_set_rights(
        vmctx: *mut VMContext,
        fd: wasm32::__wasi_fd_t,
        fs_rights_base: wasm32::__wasi_rights_t,
        fs_rights_inheriting: wasm32::__wasi_rights_t,
    ) -> wasm32::__wasi_errno_t {
        trace!(
            "fd_fdstat_set_rights(fd={:?}, fs_rights_base={:#x?}, fs_rights_inheriting={:#x?})",
            fd,
            fs_rights_base,
            fs_rights_inheriting
        );

        let wasi_ctx = match get_wasi_ctx(&mut *vmctx) {
            Ok(ctx) => ctx,
            Err(e) => return return_encoded_errno(e),
        };

        let memory = match get_memory(&mut *vmctx) {
            Ok(memory) => memory,
            Err(e) => return return_encoded_errno(e),
        };

        hostcalls::fd_fdstat_set_rights(
            wasi_ctx,
            memory,
            fd,
            fs_rights_base,
            fs_rights_inheriting
        )
    }

    pub unsafe extern "C" fn fd_sync(
        vmctx: *mut VMContext,
        fd: wasm32::__wasi_fd_t,
    ) -> wasm32::__wasi_errno_t {
        trace!("fd_sync(fd={:?})", fd);

        let wasi_ctx = match get_wasi_ctx(&mut *vmctx) {
            Ok(ctx) => ctx,
            Err(e) => return return_encoded_errno(e),
        };

        let memory = match get_memory(&mut *vmctx) {
            Ok(memory) => memory,
            Err(e) => return return_encoded_errno(e),
        };

        hostcalls::fd_sync(wasi_ctx, memory, fd)
    }

    pub unsafe extern "C" fn fd_write(
        vmctx: *mut VMContext,
        fd: wasm32::__wasi_fd_t,
        iovs: wasm32::uintptr_t,
        iovs_len: wasm32::size_t,
        nwritten: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t {
        trace!(
            "fd_write(fd={:?}, iovs={:#x?}, iovs_len={:?}, nwritten={:#x?})",
            fd,
            iovs,
            iovs_len,
            nwritten
        );

        let wasi_ctx = match get_wasi_ctx(&mut *vmctx) {
            Ok(ctx) => ctx,
            Err(e) => return return_encoded_errno(e),
        };

        let memory = match get_memory(&mut *vmctx) {
            Ok(memory) => memory,
            Err(e) => return return_encoded_errno(e),
        };

        hostcalls::fd_write(wasi_ctx, memory, fd, iovs, iovs_len, nwritten)
    }

    pub unsafe extern "C" fn fd_advise(
        vmctx: *mut VMContext,
        fd: wasm32::__wasi_fd_t,
        offset: wasm32::__wasi_filesize_t,
        len: wasm32::__wasi_filesize_t,
        advice: wasm32::__wasi_advice_t,
    ) -> wasm32::__wasi_errno_t {
        trace!(
            "fd_advise(fd={:?}, offset={}, len={}, advice={:?})",
            fd,
            offset,
            len,
            advice
        );

        let wasi_ctx = match get_wasi_ctx(&mut *vmctx) {
            Ok(ctx) => ctx,
            Err(e) => return return_encoded_errno(e),
        };

        let memory = match get_memory(&mut *vmctx) {
            Ok(memory) => memory,
            Err(e) => return return_encoded_errno(e),
        };

        hostcalls::fd_advise(wasi_ctx, memory, fd, offset, len, advice)
    }

    pub unsafe extern "C" fn fd_allocate(
        vmctx: *mut VMContext,
        fd: wasm32::__wasi_fd_t,
        offset: wasm32::__wasi_filesize_t,
        len: wasm32::__wasi_filesize_t,
    ) -> wasm32::__wasi_errno_t {
        trace!("fd_allocate(fd={:?}, offset={}, len={})", fd, offset, len);

        let wasi_ctx = match get_wasi_ctx(&mut *vmctx) {
            Ok(ctx) => ctx,
            Err(e) => return return_encoded_errno(e),
        };

        let memory = match get_memory(&mut *vmctx) {
            Ok(memory) => memory,
            Err(e) => return return_encoded_errno(e),
        };

        hostcalls::fd_allocate(wasi_ctx, memory, fd, offset, len)
    }

    pub unsafe extern "C" fn path_create_directory(
        vmctx: *mut VMContext,
        fd: wasm32::__wasi_fd_t,
        path: wasm32::uintptr_t,
        path_len: wasm32::size_t,
    ) -> wasm32::__wasi_errno_t {
        trace!(
            "path_create_directory(fd={:?}, path={:#x?}, path_len={})",
            fd,
            path,
            path_len,
        );

        let wasi_ctx = match get_wasi_ctx(&mut *vmctx) {
            Ok(ctx) => ctx,
            Err(e) => return return_encoded_errno(e),
        };

        let memory = match get_memory(&mut *vmctx) {
            Ok(memory) => memory,
            Err(e) => return return_encoded_errno(e),
        };

        hostcalls::path_create_directory(wasi_ctx, memory, fd, path, path_len)
    }

    pub unsafe extern "C" fn path_link(
        vmctx: *mut VMContext,
        fd0: wasm32::__wasi_fd_t,
        flags0: wasm32::__wasi_lookupflags_t,
        path0: wasm32::uintptr_t,
        path_len0: wasm32::size_t,
        fd1: wasm32::__wasi_fd_t,
        path1: wasm32::uintptr_t,
        path_len1: wasm32::size_t,
    ) -> wasm32::__wasi_errno_t {
        trace!(
            "path_link(fd0={:?}, flags0={:?}, path0={:#x?}, path_len0={}, fd1={:?}, path1={:#x?}, path_len1={})",
            fd0,
            flags0,
            path0,
            path_len0,
            fd1,
            path1,
            path_len1
        );

        let wasi_ctx = match get_wasi_ctx(&mut *vmctx) {
            Ok(ctx) => ctx,
            Err(e) => return return_encoded_errno(e),
        };

        let memory = match get_memory(&mut *vmctx) {
            Ok(memory) => memory,
            Err(e) => return return_encoded_errno(e),
        };

        hostcalls::path_link(
            wasi_ctx,
            memory,
            fd0,
            flags0,
            path0,
            path_len0,
            fd1,
            path1,
            path_len1
        )
    }

    // TODO: When multi-value happens, switch to that instead of passing
    // the `fd` by reference?
    pub unsafe extern "C" fn path_open(
        vmctx: *mut VMContext,
        dirfd: wasm32::__wasi_fd_t,
        dirflags: wasm32::__wasi_lookupflags_t,
        path: wasm32::uintptr_t,
        path_len: wasm32::size_t,
        oflags: wasm32::__wasi_oflags_t,
        fs_rights_base: wasm32::__wasi_rights_t,
        fs_rights_inheriting: wasm32::__wasi_rights_t,
        fs_flags: wasm32::__wasi_fdflags_t,
        fd: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t {
        trace!(
            "path_open(dirfd={:?}, dirflags={:?}, path={:#x?}, path_len={:?}, oflags={:#x?}, fs_rights_base={:#x?}, fs_rights_inheriting={:#x?}, fs_flags={:#x?}, fd={:#x?})",
            dirfd,
            dirflags,
            path,
            path_len,
            oflags,
            fs_rights_base,
            fs_rights_inheriting,
            fs_flags,
            fd
        );

        let wasi_ctx = match get_wasi_ctx(&mut *vmctx) {
            Ok(ctx) => ctx,
            Err(e) => return return_encoded_errno(e),
        };

        let memory = match get_memory(&mut *vmctx) {
            Ok(memory) => memory,
            Err(e) => return return_encoded_errno(e),
        };

        hostcalls::path_open(
            wasi_ctx,
            memory,
            dirfd,
            dirflags,
            path,
            path_len,
            oflags,
            fs_rights_base,
            fs_rights_inheriting,
            fs_flags,
            fd
        )
    }

    pub unsafe extern "C" fn fd_readdir(
        vmctx: *mut VMContext,
        fd: wasm32::__wasi_fd_t,
        buf: wasm32::uintptr_t,
        buf_len: wasm32::size_t,
        cookie: wasm32::__wasi_dircookie_t,
        buf_used: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t {
        trace!(
            "fd_readdir(fd={:?}, buf={:#x?}, buf_len={}, cookie={:#x?}, buf_used={:#x?})",
            fd,
            buf,
            buf_len,
            cookie,
            buf_used,
        );

        let wasi_ctx = match get_wasi_ctx(&mut *vmctx) {
            Ok(ctx) => ctx,
            Err(e) => return return_encoded_errno(e),
        };

        let memory = match get_memory(&mut *vmctx) {
            Ok(memory) => memory,
            Err(e) => return return_encoded_errno(e),
        };

        hostcalls::fd_readdir(
            wasi_ctx,
            memory,
            fd,
            buf,
            buf_len,
            cookie,
            buf_used
        )
    }

    pub unsafe extern "C" fn path_readlink(
        vmctx: *mut VMContext,
        fd: wasm32::__wasi_fd_t,
        path: wasm32::uintptr_t,
        path_len: wasm32::size_t,
        buf: wasm32::uintptr_t,
        buf_len: wasm32::size_t,
        buf_used: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t {
        trace!(
            "path_readlink(fd={:?}, path={:#x?}, path_len={:?}, buf={:#x?}, buf_len={}, buf_used={:#x?})",
            fd,
            path,
            path_len,
            buf,
            buf_len,
            buf_used,
        );

        let wasi_ctx = match get_wasi_ctx(&mut *vmctx) {
            Ok(ctx) => ctx,
            Err(e) => return return_encoded_errno(e),
        };

        let memory = match get_memory(&mut *vmctx) {
            Ok(memory) => memory,
            Err(e) => return return_encoded_errno(e),
        };

        hostcalls::path_readlink(
            wasi_ctx,
            memory,
            fd,
            path,
            path_len,
            buf,
            buf_len,
            buf_used
        )
    }

    pub unsafe extern "C" fn path_rename(
        vmctx: *mut VMContext,
        fd0: wasm32::__wasi_fd_t,
        path0: wasm32::uintptr_t,
        path_len0: wasm32::size_t,
        fd1: wasm32::__wasi_fd_t,
        path1: wasm32::uintptr_t,
        path_len1: wasm32::size_t,
    ) -> wasm32::__wasi_errno_t {
        trace!(
            "path_rename(fd0={:?}, path0={:#x?}, path_len0={:?}, fd1={:?}, path1={:#x?}, path_len1={:?})",
            fd0,
            path0,
            path_len0,
            fd1,
            path1,
            path_len1,
        );

        let wasi_ctx = match get_wasi_ctx(&mut *vmctx) {
            Ok(ctx) => ctx,
            Err(e) => return return_encoded_errno(e),
        };

        let memory = match get_memory(&mut *vmctx) {
            Ok(memory) => memory,
            Err(e) => return return_encoded_errno(e),
        };

        hostcalls::path_rename(
            wasi_ctx,
            memory,
            fd0,
            path0,
            path_len0,
            fd1,
            path1,
            path_len1
        )
    }

    pub unsafe extern "C" fn fd_filestat_get(
        vmctx: *mut VMContext,
        fd: wasm32::__wasi_fd_t,
        buf: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t {
        trace!("fd_filestat_get(fd={:?}, buf={:#x?})", fd, buf);

        let wasi_ctx = match get_wasi_ctx(&mut *vmctx) {
            Ok(ctx) => ctx,
            Err(e) => return return_encoded_errno(e),
        };

        let memory = match get_memory(&mut *vmctx) {
            Ok(memory) => memory,
            Err(e) => return return_encoded_errno(e),
        };

        hostcalls::fd_filestat_get(wasi_ctx, memory, fd, buf)
    }

    pub unsafe extern "C" fn fd_filestat_set_times(
        vmctx: *mut VMContext,
        fd: wasm32::__wasi_fd_t,
        st_atim: wasm32::__wasi_timestamp_t,
        st_mtim: wasm32::__wasi_timestamp_t,
        fstflags: wasm32::__wasi_fstflags_t,
    ) -> wasm32::__wasi_errno_t {
        trace!(
            "fd_filestat_set_times(fd={:?}, st_atim={}, st_mtim={}, fstflags={:#x?})",
            fd,
            st_atim, st_mtim,
            fstflags
        );

        let wasi_ctx = match get_wasi_ctx(&mut *vmctx) {
            Ok(ctx) => ctx,
            Err(e) => return return_encoded_errno(e),
        };

        let memory = match get_memory(&mut *vmctx) {
            Ok(memory) => memory,
            Err(e) => return return_encoded_errno(e),
        };

        hostcalls::fd_filestat_set_times(wasi_ctx, memory, fd, st_atim, st_mtim, fstflags)
    }

    pub unsafe extern "C" fn fd_filestat_set_size(
        vmctx: *mut VMContext,
        fd: wasm32::__wasi_fd_t,
        size: wasm32::__wasi_filesize_t,
    ) -> wasm32::__wasi_errno_t {
        trace!(
            "fd_filestat_set_size(fd={:?}, size={})",
            fd,
            size
        );

        let wasi_ctx = match get_wasi_ctx(&mut *vmctx) {
            Ok(ctx) => ctx,
            Err(e) => return return_encoded_errno(e),
        };

        let memory = match get_memory(&mut *vmctx) {
            Ok(memory) => memory,
            Err(e) => return return_encoded_errno(e),
        };

        hostcalls::fd_filestat_set_size(wasi_ctx, memory, fd, size)
    }

    pub unsafe extern "C" fn path_filestat_get(
        vmctx: *mut VMContext,
        fd: wasm32::__wasi_fd_t,
        flags: wasm32::__wasi_lookupflags_t,
        path: wasm32::uintptr_t,
        path_len: wasm32::size_t,
        buf: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t {
        trace!(
            "path_filestat_get(fd={:?}, flags={:?}, path={:#x?}, path_len={}, buf={:#x?})",
            fd,
            flags,
            path,
            path_len,
            buf
        );

        let wasi_ctx = match get_wasi_ctx(&mut *vmctx) {
            Ok(ctx) => ctx,
            Err(e) => return return_encoded_errno(e),
        };

        let memory = match get_memory(&mut *vmctx) {
            Ok(memory) => memory,
            Err(e) => return return_encoded_errno(e),
        };

        hostcalls::path_filestat_get(wasi_ctx, memory, fd, flags, path, path_len, buf)
    }

    pub unsafe extern "C" fn path_filestat_set_times(
        vmctx: *mut VMContext,
        fd: wasm32::__wasi_fd_t,
        flags: wasm32::__wasi_lookupflags_t,
        path: wasm32::uintptr_t,
        path_len: wasm32::size_t,
        st_atim: wasm32::__wasi_timestamp_t,
        st_mtim: wasm32::__wasi_timestamp_t,
        fstflags: wasm32::__wasi_fstflags_t,
    ) -> wasm32::__wasi_errno_t {
        trace!(
            "path_filestat_set_times(fd={:?}, flags={:?}, path={:#x?}, path_len={}, st_atim={}, st_mtim={}, fstflags={:#x?})",
            fd,
            flags,
            path,
            path_len,
            st_atim, st_mtim,
            fstflags
        );

        let wasi_ctx = match get_wasi_ctx(&mut *vmctx) {
            Ok(ctx) => ctx,
            Err(e) => return return_encoded_errno(e),
        };

        let memory = match get_memory(&mut *vmctx) {
            Ok(memory) => memory,
            Err(e) => return return_encoded_errno(e),
        };

        hostcalls::path_filestat_set_times(
            wasi_ctx,
            memory,
            fd,
            flags,
            path,
            path_len,
            st_atim,
            st_mtim,
            fstflags
        )
    }

    pub unsafe extern "C" fn path_symlink(
        vmctx: *mut VMContext,
        path0: wasm32::uintptr_t,
        path_len0: wasm32::size_t,
        fd: wasm32::__wasi_fd_t,
        path1: wasm32::uintptr_t,
        path_len1: wasm32::size_t,
    ) -> wasm32::__wasi_errno_t {
        trace!(
            "path_symlink(path0={:#x?}, path_len0={}, fd={:?}, path1={:#x?}, path_len1={})",
            path0,
            path_len0,
            fd,
            path1,
            path_len1
        );

        let wasi_ctx = match get_wasi_ctx(&mut *vmctx) {
            Ok(ctx) => ctx,
            Err(e) => return return_encoded_errno(e),
        };

        let memory = match get_memory(&mut *vmctx) {
            Ok(memory) => memory,
            Err(e) => return return_encoded_errno(e),
        };

        hostcalls::path_symlink(
            wasi_ctx,
            memory,
            path0,
            path_len0,
            fd,
            path1,
            path_len1
        )
    }

    pub unsafe extern "C" fn path_unlink_file(
        vmctx: *mut VMContext,
        fd: wasm32::__wasi_fd_t,
        path: wasm32::uintptr_t,
        path_len: wasm32::size_t,
    ) -> wasm32::__wasi_errno_t {
        trace!(
            "path_unlink_file(fd={:?}, path={:#x?}, path_len={})",
            fd,
            path,
            path_len
        );

        let wasi_ctx = match get_wasi_ctx(&mut *vmctx) {
            Ok(ctx) => ctx,
            Err(e) => return return_encoded_errno(e),
        };

        let memory = match get_memory(&mut *vmctx) {
            Ok(memory) => memory,
            Err(e) => return return_encoded_errno(e),
        };

        hostcalls::path_unlink_file(wasi_ctx, memory, fd, path, path_len)
    }

    pub unsafe extern "C" fn path_remove_directory(
        vmctx: *mut VMContext,
        fd: wasm32::__wasi_fd_t,
        path: wasm32::uintptr_t,
        path_len: wasm32::size_t,
    ) -> wasm32::__wasi_errno_t {
        trace!(
            "path_remove_directory(fd={:?}, path={:#x?}, path_len={})",
            fd,
            path,
            path_len
        );

        let wasi_ctx = match get_wasi_ctx(&mut *vmctx) {
            Ok(ctx) => ctx,
            Err(e) => return return_encoded_errno(e),
        };

        let memory = match get_memory(&mut *vmctx) {
            Ok(memory) => memory,
            Err(e) => return return_encoded_errno(e),
        };

        hostcalls::path_remove_directory(wasi_ctx, memory, fd, path, path_len)
    }

    pub unsafe extern "C" fn poll_oneoff(
        vmctx: *mut VMContext,
        in_: wasm32::uintptr_t,
        out: wasm32::uintptr_t,
        nsubscriptions: wasm32::size_t,
        nevents: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t {
        trace!(
            "poll_oneoff(in={:#x?}, out={:#x?}, nsubscriptions={}, nevents={:#x?})",
            in_,
            out,
            nsubscriptions,
            nevents,
        );

        let memory = match get_memory(&mut *vmctx) {
            Ok(memory) => memory,
            Err(e) => return return_encoded_errno(e),
        };

        hostcalls::poll_oneoff(memory, in_, out, nsubscriptions, nevents)
    }

    pub unsafe extern "C" fn proc_exit(_vmctx: *mut VMContext, rval: u32,) -> () {
        trace!("proc_exit(rval={:?})", rval);

        hostcalls::proc_exit(rval)
    }

    pub unsafe extern "C" fn proc_raise(
        vmctx: *mut VMContext,
        sig: wasm32::__wasi_signal_t,
    ) -> wasm32::__wasi_errno_t {
        trace!("proc_raise(sig={:?})", sig);

        let wasi_ctx = match get_wasi_ctx(&mut *vmctx) {
            Ok(ctx) => ctx,
            Err(e) => return return_encoded_errno(e),
        };

        let memory = match get_memory(&mut *vmctx) {
            Ok(memory) => memory,
            Err(e) => return return_encoded_errno(e),
        };

        hostcalls::proc_raise(wasi_ctx, memory, sig)
    }

    pub unsafe extern "C" fn random_get(
        vmctx: *mut VMContext,
        buf: wasm32::uintptr_t,
        buf_len: wasm32::size_t,
    ) -> wasm32::__wasi_errno_t {
        trace!("random_get(buf={:#x?}, buf_len={:?})", buf, buf_len);

        let memory = match get_memory(&mut *vmctx) {
            Ok(memory) => memory,
            Err(e) => return return_encoded_errno(e),
        };

        hostcalls::random_get(memory, buf, buf_len)
    }

    pub unsafe extern "C" fn sched_yield(_vmctx: *mut VMContext,) -> wasm32::__wasi_errno_t {
        trace!("sched_yield(void)");

        hostcalls::sched_yield()
    }

    pub unsafe extern "C" fn sock_recv(
        vmctx: *mut VMContext,
        sock: wasm32::__wasi_fd_t,
        ri_data: wasm32::uintptr_t,
        ri_data_len: wasm32::size_t,
        ri_flags: wasm32::__wasi_riflags_t,
        ro_datalen: wasm32::uintptr_t,
        ro_flags: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t {
        trace!(
            "sock_recv(sock={:?}, ri_data={:#x?}, ri_data_len={}, ri_flags={:#x?}, ro_datalen={:#x?}, ro_flags={:#x?})",
            sock,
            ri_data, ri_data_len, ri_flags,
            ro_datalen, ro_flags
        );

        let wasi_ctx = match get_wasi_ctx(&mut *vmctx) {
            Ok(ctx) => ctx,
            Err(e) => return return_encoded_errno(e),
        };

        let memory = match get_memory(&mut *vmctx) {
            Ok(memory) => memory,
            Err(e) => return return_encoded_errno(e),
        };

        hostcalls::sock_recv(
            wasi_ctx,
            memory,
            sock,
            ri_data,
            ri_data_len,
            ri_flags,
            ro_datalen,
            ro_flags
        )
    }

    pub unsafe extern "C" fn sock_send(
        vmctx: *mut VMContext,
        sock: wasm32::__wasi_fd_t,
        si_data: wasm32::uintptr_t,
        si_data_len: wasm32::size_t,
        si_flags: wasm32::__wasi_siflags_t,
        so_datalen: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t {
        trace!(
            "sock_send(sock={:?}, si_data={:#x?}, si_data_len={}, si_flags={:#x?}, so_datalen={:#x?})",
            sock,
            si_data, si_data_len, si_flags, so_datalen,
        );

        let wasi_ctx = match get_wasi_ctx(&mut *vmctx) {
            Ok(ctx) => ctx,
            Err(e) => return return_encoded_errno(e),
        };

        let memory = match get_memory(&mut *vmctx) {
            Ok(memory) => memory,
            Err(e) => return return_encoded_errno(e),
        };

        hostcalls::sock_send(
            wasi_ctx,
            memory,
            sock,
            si_data,
            si_data_len,
            si_flags,
            so_datalen
        )
    }

    pub unsafe extern "C" fn sock_shutdown(
        vmctx: *mut VMContext,
        sock: wasm32::__wasi_fd_t,
        how: wasm32::__wasi_sdflags_t,
    ) -> wasm32::__wasi_errno_t {
        trace!("sock_shutdown(sock={:?}, how={:?})", sock, how);

        let wasi_ctx = match get_wasi_ctx(&mut *vmctx) {
            Ok(ctx) => ctx,
            Err(e) => return return_encoded_errno(e),
        };

        let memory = match get_memory(&mut *vmctx) {
            Ok(memory) => memory,
            Err(e) => return return_encoded_errno(e),
        };

        hostcalls::sock_shutdown(wasi_ctx, memory, sock, how)
    }
}
