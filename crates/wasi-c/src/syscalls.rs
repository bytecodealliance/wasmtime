use crate::host::{argv_environ_values, fd_prestats, fd_table};
use crate::instantiate::WASIState;
use crate::translate::*;
use crate::{host, wasm32};
use cranelift_codegen::ir::types::{Type, I32, I64};
use log::{log_enabled, trace};
use std::convert::TryFrom;
use std::{mem, ptr, slice, str};
use wasmtime_runtime::VMContext;

fn str_for_trace<'str>(ptr: *const i8, len: usize) -> Result<&'str str, str::Utf8Error> {
    str::from_utf8(unsafe { slice::from_raw_parts(ptr as *const u8, len) })
}

fn return_encoded_errno(e: host::__wasi_errno_t) -> wasm32::__wasi_errno_t {
    let errno = encode_errno(e);
    trace!("    -> errno={}", wasm32::strerror(errno));
    errno
}

unsafe fn get_curfds(vmctx: *mut VMContext) -> *mut fd_table {
    (&mut *(&mut *vmctx)
        .host_state()
        .downcast_mut::<WASIState>()
        .unwrap()
        .curfds) as *mut fd_table
}

unsafe fn get_prestats(vmctx: *mut VMContext) -> *mut fd_prestats {
    (&mut *(&mut *vmctx)
        .host_state()
        .downcast_mut::<WASIState>()
        .unwrap()
        .prestats) as *mut fd_prestats
}

unsafe fn get_argv_environ(vmctx: *mut VMContext) -> *mut argv_environ_values {
    (&mut *(&mut *vmctx)
        .host_state()
        .downcast_mut::<WASIState>()
        .unwrap()
        .argv_environ) as *mut argv_environ_values
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

        let vmctx = &mut *vmctx;
        let argv_environ = get_argv_environ(vmctx);
        let argc = match u32::try_from((*argv_environ).argc) {
            Ok(argc) => argc,
            Err(_) => return wasm32::__WASI_ENOMEM,
        };
        let argv_buf_size = match u32::try_from((*argv_environ).argv_buf_size) {
            Ok(argc) => argc,
            Err(_) => return wasm32::__WASI_ENOMEM,
        };

        let (host_argv_buf, _argv_buf_size) = match decode_char_slice(vmctx, argv_buf, argv_buf_size) {
            Ok((argv_buf, argv_buf_size)) => (argv_buf, argv_buf_size),
            Err(e) => return return_encoded_errno(e),
        };
        // Add 1 so that we can add an extra NULL pointer at the end.
        let (argv, _argc) = match decode_charstar_slice(vmctx, argv, argc + 1) {
            Ok((argv, argc)) => (argv, argc),
            Err(e) => return return_encoded_errno(e),
        };
        let mut host_argv = Vec::new();
        host_argv.resize((*argv_environ).argc + 1, ptr::null_mut());

        let e = host::wasmtime_ssp_args_get(argv_environ, host_argv.as_mut_ptr(), host_argv_buf);

        encode_charstar_slice(argv, host_argv, argv_buf, host_argv_buf);

        return_encoded_errno(e)
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

        let vmctx = &mut *vmctx;
        let mut host_argc = 0;
        if let Err(e) = decode_usize_byref(vmctx, argc) {
            return return_encoded_errno(e);
        }
        let mut host_argv_buf_size = 0;
        if let Err(e) = decode_usize_byref(vmctx, argv_buf_size) {
            return return_encoded_errno(e);
        }

        let vmctx = &mut *vmctx;
        let argv_environ = get_argv_environ(vmctx);

        let e = host::wasmtime_ssp_args_sizes_get(argv_environ, &mut host_argc, &mut host_argv_buf_size);

        if u32::from(e) == host::__WASI_ESUCCESS {
            trace!("     | *argc={:?}", host_argc);
            encode_usize_byref(vmctx, argc, host_argc);

            trace!("     | *argv_buf_size={:?}", host_argv_buf_size);
            encode_usize_byref(vmctx, argv_buf_size, host_argv_buf_size);
        }

        return_encoded_errno(e)
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

        let vmctx = &mut *vmctx;
        let clock_id = decode_clockid(clock_id);
        let mut host_resolution = 0;
        if let Err(e) = decode_timestamp_byref(vmctx, resolution) {
            return return_encoded_errno(e);
        }

        let e = host::wasmtime_ssp_clock_res_get(clock_id, &mut host_resolution);

        if u32::from(e) == host::__WASI_ESUCCESS {
            trace!("     | *resolution={:?}", host_resolution);
            encode_timestamp_byref(vmctx, resolution, host_resolution);
        }

        return_encoded_errno(e)
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

        let vmctx = &mut *vmctx;
        let clock_id = decode_clockid(clock_id);
        let precision = decode_timestamp(precision);
        let mut host_time = 0;
        if let Err(e) = decode_timestamp_byref(vmctx, time) {
            return return_encoded_errno(e);
        }

        let e = host::wasmtime_ssp_clock_time_get(clock_id, precision, &mut host_time);

        if u32::from(e) == host::__WASI_ESUCCESS {
            trace!("     | *time={:?}", host_time);
            encode_timestamp_byref(vmctx, time, host_time);
        }

        return_encoded_errno(e)
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

        let vmctx = &mut *vmctx;
        let argv_environ = get_argv_environ(vmctx);
        let environ_count = match u32::try_from((*argv_environ).environ_count) {
            Ok(host_environ_count) => host_environ_count,
            Err(_) => return wasm32::__WASI_ENOMEM,
        };
        let environ_buf_size = match u32::try_from((*argv_environ).environ_buf_size) {
            Ok(host_environ_buf_size) => host_environ_buf_size,
            Err(_) => return wasm32::__WASI_ENOMEM,
        };

        let (host_environ_buf, _environ_buf_len) = match decode_char_slice(vmctx, environ_buf, environ_buf_size) {
            Ok((environ_buf, environ_buf_len)) => (environ_buf, environ_buf_len),
            Err(e) => return return_encoded_errno(e),
        };
        // Add 1 so that we can add an extra NULL pointer at the end.
        let (environ, _environ_count) = match decode_charstar_slice(vmctx, environ, environ_count + 1) {
            Ok((environ, environ_count)) => (environ, environ_count),
            Err(e) => return return_encoded_errno(e),
        };
        let mut host_environ = Vec::new();
        host_environ.resize((*argv_environ).environ_count + 1, ptr::null_mut());

        let e = host::wasmtime_ssp_environ_get(argv_environ, host_environ.as_mut_ptr(), host_environ_buf);

        encode_charstar_slice(environ, host_environ, environ_buf, host_environ_buf);

        return_encoded_errno(e)
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

        let vmctx = &mut *vmctx;
        let mut host_environ_count = 0;
        if let Err(e) = decode_usize_byref(vmctx, environ_count) {
            return return_encoded_errno(e);
        }
        let mut host_environ_buf_size = 0;
        if let Err(e) = decode_usize_byref(vmctx, environ_buf_size) {
            return return_encoded_errno(e);
        }

        let vmctx = &mut *vmctx;
        let argv_environ = get_argv_environ(vmctx);

        let e = host::wasmtime_ssp_environ_sizes_get(argv_environ, &mut host_environ_count, &mut host_environ_buf_size);

        if u32::from(e) == host::__WASI_ESUCCESS {
            trace!("     | *environ_count={:?}", host_environ_count);
            encode_usize_byref(vmctx, environ_count, host_environ_count);

            trace!("     | *environ_buf_size={:?}", host_environ_buf_size);
            encode_usize_byref(vmctx, environ_buf_size, host_environ_buf_size);
        }

        return_encoded_errno(e)
    }

    pub unsafe extern "C" fn fd_prestat_get(
        vmctx: *mut VMContext,
        fd: wasm32::__wasi_fd_t,
        buf: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t {
        trace!("fd_prestat_get(fd={:?}, buf={:#x?})", fd, buf);

        let vmctx = &mut *vmctx;
        let prestats = get_prestats(vmctx);
        let fd = decode_fd(fd);
        let mut host_buf = std::mem::zeroed();
        if let Err(e) = decode_prestat_byref(vmctx, buf) {
            return return_encoded_errno(e);
        }

        let e = host::wasmtime_ssp_fd_prestat_get(prestats, fd, &mut host_buf);

        encode_prestat_byref(vmctx, buf, host_buf);

        return_encoded_errno(e)
    }

    pub unsafe extern "C" fn fd_prestat_dir_name(
        vmctx: *mut VMContext,
        fd: wasm32::__wasi_fd_t,
        path: wasm32::uintptr_t,
        path_len: wasm32::size_t,
    ) -> wasm32::__wasi_errno_t {
        trace!("fd_prestat_dir_name(fd={:?}, path={:#x?}, path_len={})", fd, path, path_len);

        let vmctx = &mut *vmctx;
        let prestats = get_prestats(vmctx);
        let (path, path_len) = match decode_char_slice(vmctx, path, path_len) {
            Ok((path, path_len)) => (path, path_len),
            Err(e) => return return_encoded_errno(e),
        };

        trace!("     | (path,path_len)={:?}", str_for_trace(path, path_len));

        let e = host::wasmtime_ssp_fd_prestat_dir_name(prestats, fd, path, path_len);

        return_encoded_errno(e)
    }

    pub unsafe extern "C" fn fd_close(
        vmctx: *mut VMContext,
        fd: wasm32::__wasi_fd_t,
    ) -> wasm32::__wasi_errno_t {
        trace!("fd_close(fd={:?})", fd);

        let vmctx = &mut *vmctx;
        let curfds = get_curfds(vmctx);
        let prestats = get_prestats(vmctx);
        let fd = decode_fd(fd);

        let e = host::wasmtime_ssp_fd_close(curfds, prestats, fd);

        return_encoded_errno(e)
    }

    pub unsafe extern "C" fn fd_datasync(
        vmctx: *mut VMContext,
        fd: wasm32::__wasi_fd_t,
    ) -> wasm32::__wasi_errno_t {
        trace!("fd_datasync(fd={:?})", fd);

        let vmctx = &mut *vmctx;
        let curfds = get_curfds(vmctx);
        let fd = decode_fd(fd);

        let e = host::wasmtime_ssp_fd_datasync(curfds, fd);

        return_encoded_errno(e)
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

        let vmctx = &mut *vmctx;
        let curfds = get_curfds(vmctx);
        let fd = decode_fd(fd);
        let iovs = match decode_iovec_slice(vmctx, iovs, iovs_len) {
            Ok(iovs) => iovs,
            Err(e) => return return_encoded_errno(e),
        };
        let offset = decode_filesize(offset);
        let mut host_nread = 0;
        if let Err(e) = decode_usize_byref(vmctx, nread) {
            return return_encoded_errno(e);
        }

        let e = host::wasmtime_ssp_fd_pread(
            curfds,
            fd,
            iovs.as_ptr(),
            iovs.len(),
            offset,
            &mut host_nread,
        );

        if u32::from(e) == host::__WASI_ESUCCESS {
            trace!("     | *nread={:?}", host_nread);
            encode_usize_byref(vmctx, nread, host_nread);
        }

        return_encoded_errno(e)
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

        let vmctx = &mut *vmctx;
        let curfds = get_curfds(vmctx);
        let fd = decode_fd(fd);
        let iovs = match decode_ciovec_slice(vmctx, iovs, iovs_len) {
            Ok(iovs) => iovs,
            Err(e) => return return_encoded_errno(e),
        };
        let offset = decode_filesize(offset);
        let mut host_nwritten = 0;
        if let Err(e) = decode_usize_byref(vmctx, nwritten) {
            return return_encoded_errno(e);
        }

        let e = host::wasmtime_ssp_fd_pwrite(
            curfds,
            fd,
            iovs.as_ptr(),
            iovs.len(),
            offset,
            &mut host_nwritten,
        );

        if u32::from(e) == host::__WASI_ESUCCESS {
            trace!("     | *nwritten={:?}", host_nwritten);
            encode_usize_byref(vmctx, nwritten, host_nwritten);
        }

        return_encoded_errno(e)
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

        let vmctx = &mut *vmctx;
        let curfds = get_curfds(vmctx);
        let fd = decode_fd(fd);
        let iovs = match decode_iovec_slice(vmctx, iovs, iovs_len) {
            Ok(iovs) => iovs,
            Err(e) => return return_encoded_errno(e),
        };
        let mut host_nread = 0;
        if let Err(e) = decode_usize_byref(vmctx, nread) {
            return return_encoded_errno(e);
        }

        let e = host::wasmtime_ssp_fd_read(curfds, fd, iovs.as_ptr(), iovs.len(), &mut host_nread);

        if u32::from(e) == host::__WASI_ESUCCESS {
            trace!("     | *nread={:?}", host_nread);
            encode_usize_byref(vmctx, nread, host_nread);
        }

        return_encoded_errno(e)
    }

    pub unsafe extern "C" fn fd_renumber(
        vmctx: *mut VMContext,
        from: wasm32::__wasi_fd_t,
        to: wasm32::__wasi_fd_t,
    ) -> wasm32::__wasi_errno_t {
        trace!("fd_renumber(from={:?}, to={:?})", from, to);

        let vmctx = &mut *vmctx;
        let curfds = get_curfds(vmctx);
        let prestats = get_prestats(vmctx);
        let from = decode_fd(from);
        let to = decode_fd(to);

        let e = host::wasmtime_ssp_fd_renumber(curfds, prestats, from, to);

        return_encoded_errno(e)
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

        let vmctx = &mut *vmctx;
        let curfds = get_curfds(vmctx);
        let fd = decode_fd(fd);
        let offset = decode_filedelta(offset);
        let whence = decode_whence(whence);
        let mut host_newoffset = 0;
        if let Err(e) = decode_filesize_byref(vmctx, newoffset) {
            return return_encoded_errno(e);
        }

        let e = host::wasmtime_ssp_fd_seek(curfds, fd, offset, whence, &mut host_newoffset);

        if u32::from(e) == host::__WASI_ESUCCESS {
            trace!("     | *newoffset={:?}", host_newoffset);
            encode_filesize_byref(vmctx, newoffset, host_newoffset);
        }

        return_encoded_errno(e)
    }

    pub unsafe extern "C" fn fd_tell(
        vmctx: *mut VMContext,
        fd: wasm32::__wasi_fd_t,
        newoffset: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t {
        trace!("fd_tell(fd={:?}, newoffset={:#x?})", fd, newoffset);

        let vmctx = &mut *vmctx;
        let curfds = get_curfds(vmctx);
        let fd = decode_fd(fd);
        let mut host_newoffset = 0;
        if let Err(e) = decode_filesize_byref(vmctx, newoffset) {
            return return_encoded_errno(e);
        }

        let e = host::wasmtime_ssp_fd_tell(curfds, fd, &mut host_newoffset);

        if u32::from(e) == host::__WASI_ESUCCESS {
            trace!("     | *newoffset={:?}", host_newoffset);
            encode_filesize_byref(vmctx, newoffset, host_newoffset);
        }

        return_encoded_errno(e)
    }

    pub unsafe extern "C" fn fd_fdstat_get(
        vmctx: *mut VMContext,
        fd: wasm32::__wasi_fd_t,
        buf: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t {
        trace!("fd_fdstat_get(fd={:?}, buf={:#x?})", fd, buf);

        let vmctx = &mut *vmctx;
        let curfds = get_curfds(vmctx);
        let fd = decode_fd(fd);
        let mut host_buf = std::mem::zeroed();
        if let Err(e) = decode_fdstat_byref(vmctx, buf) {
            return return_encoded_errno(e);
        }

        let e = host::wasmtime_ssp_fd_fdstat_get(curfds, fd, &mut host_buf);

        if u32::from(e) == host::__WASI_ESUCCESS {
            trace!("     | *buf={:?}", host_buf);
            encode_fdstat_byref(vmctx, buf, host_buf);
        }

        return_encoded_errno(e)
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

        let vmctx = &mut *vmctx;
        let curfds = get_curfds(vmctx);
        let fd = decode_fd(fd);
        let flags = decode_fdflags(flags);

        let e = host::wasmtime_ssp_fd_fdstat_set_flags(curfds, fd, flags);

        return_encoded_errno(e)
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

        let vmctx = &mut *vmctx;
        let curfds = get_curfds(vmctx);
        let fd = decode_fd(fd);
        let fs_rights_base = decode_rights(fs_rights_base);
        let fs_rights_inheriting = decode_rights(fs_rights_inheriting);

        let e = host::wasmtime_ssp_fd_fdstat_set_rights(curfds, fd, fs_rights_base, fs_rights_inheriting);

        return_encoded_errno(e)
    }

    pub unsafe extern "C" fn fd_sync(
        vmctx: *mut VMContext,
        fd: wasm32::__wasi_fd_t,
    ) -> wasm32::__wasi_errno_t {
        trace!("fd_sync(fd={:?})", fd);

        let vmctx = &mut *vmctx;
        let curfds = get_curfds(vmctx);
        let fd = decode_fd(fd);

        let e = host::wasmtime_ssp_fd_sync(curfds, fd);

        return_encoded_errno(e)
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

        let vmctx = &mut *vmctx;
        let curfds = get_curfds(vmctx);
        let fd = decode_fd(fd);
        let iovs = match decode_ciovec_slice(vmctx, iovs, iovs_len) {
            Ok(iovs) => iovs,
            Err(e) => return return_encoded_errno(e),
        };
        let mut host_nwritten = 0;
        if let Err(e) = decode_usize_byref(vmctx, nwritten) {
            return return_encoded_errno(e);
        }

        let e = host::wasmtime_ssp_fd_write(curfds, fd, iovs.as_ptr(), iovs.len(), &mut host_nwritten);

        if u32::from(e) == host::__WASI_ESUCCESS {
            trace!("     | *nwritten={:?}", host_nwritten);
            encode_usize_byref(vmctx, nwritten, host_nwritten);
        }

        return_encoded_errno(e)
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

        let vmctx = &mut *vmctx;
        let curfds = get_curfds(vmctx);
        let fd = decode_fd(fd);
        let offset = decode_filesize(offset);
        let len = decode_filesize(len);
        let advice = decode_advice(advice);

        let e = host::wasmtime_ssp_fd_advise(curfds, fd, offset, len, advice);

        return_encoded_errno(e)
    }

    pub unsafe extern "C" fn fd_allocate(
        vmctx: *mut VMContext,
        fd: wasm32::__wasi_fd_t,
        offset: wasm32::__wasi_filesize_t,
        len: wasm32::__wasi_filesize_t,
    ) -> wasm32::__wasi_errno_t {
        trace!("fd_allocate(fd={:?}, offset={}, len={})", fd, offset, len);

        let vmctx = &mut *vmctx;
        let curfds = get_curfds(vmctx);
        let fd = decode_fd(fd);
        let offset = decode_filesize(offset);
        let len = decode_filesize(len);

        let e = host::wasmtime_ssp_fd_allocate(curfds, fd, offset, len);

        return_encoded_errno(e)
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

        let vmctx = &mut *vmctx;
        let curfds = get_curfds(vmctx);
        let fd = decode_fd(fd);
        let (path, path_len) = match decode_char_slice(vmctx, path, path_len) {
            Ok((path, path_len)) => (path, path_len),
            Err(e) => return return_encoded_errno(e),
        };

        trace!("     | (path,path_len)={:?}", str_for_trace(path, path_len));

        let e = host::wasmtime_ssp_path_create_directory(curfds, fd, path, path_len);

        return_encoded_errno(e)
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

        let vmctx = &mut *vmctx;
        let curfds = get_curfds(vmctx);
        let fd0 = decode_fd(fd0);
        let flags0 = decode_lookupflags(flags0);
        let (path0, path_len0) = match decode_char_slice(vmctx, path0, path_len0) {
            Ok((path0, path_len0)) => (path0, path_len0),
            Err(e) => return return_encoded_errno(e),
        };
        let fd1 = decode_fd(fd1);
        let (path1, path_len1) = match decode_char_slice(vmctx, path1, path_len1) {
            Ok((path1, path_len1)) => (path1, path_len1),
            Err(e) => return return_encoded_errno(e),
        };

        trace!("     | (path0,path_len0)={:?}", str_for_trace(path0, path_len0));
        trace!("     | (path1,path_len1)={:?}", str_for_trace(path1, path_len1));

        let e =
            host::wasmtime_ssp_path_link(curfds, fd0, flags0, path0, path_len0, fd1, path1, path_len1);

        return_encoded_errno(e)
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

        let vmctx = &mut *vmctx;
        let curfds = get_curfds(vmctx);
        let dirfd = decode_fd(dirfd);
        let dirflags = decode_lookupflags(dirflags);
        let (path, path_len) = match decode_char_slice(vmctx, path, path_len) {
            Ok((path, path_len)) => (path, path_len),
            Err(e) => return return_encoded_errno(e),
        };
        let oflags = decode_oflags(oflags);
        let fs_rights_base = decode_rights(fs_rights_base);
        let fs_rights_inheriting = decode_rights(fs_rights_inheriting);
        let fs_flags = decode_fdflags(fs_flags);
        let mut host_fd = wasm32::__wasi_fd_t::max_value();
        if let Err(e) = decode_fd_byref(vmctx, fd) {
            return return_encoded_errno(e);
        }

        trace!("     | (path,path_len)={:?}", str_for_trace(path, path_len));

        let e = host::wasmtime_ssp_path_open(
            curfds,
            dirfd,
            dirflags,
            path,
            path_len,
            oflags,
            fs_rights_base,
            fs_rights_inheriting,
            fs_flags,
            &mut host_fd,
        );

        trace!("     | *fd={:?}", host_fd);
        encode_fd_byref(vmctx, fd, host_fd);

        return_encoded_errno(e)
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

        let vmctx = &mut *vmctx;
        let curfds = get_curfds(vmctx);
        let fd = decode_fd(fd);
        let (buf, buf_len) = match decode_char_slice(vmctx, buf, buf_len) {
            Ok((buf, buf_len)) => (buf, buf_len),
            Err(e) => return return_encoded_errno(e),
        };
        let cookie = decode_dircookie(cookie);
        let mut host_buf_used = 0;
        if let Err(e) = decode_usize_byref(vmctx, buf_used) {
            return return_encoded_errno(e);
        }

        trace!("     | (buf,buf_len)={:?}", str_for_trace(buf, buf_len));

        let e = host::wasmtime_ssp_fd_readdir(
            curfds,
            fd,
            buf as *mut host::void,
            buf_len,
            cookie,
            &mut host_buf_used,
        );

        if u32::from(e) == host::__WASI_ESUCCESS {
            trace!("     | *buf_used={:?}", host_buf_used);
            encode_usize_byref(vmctx, buf_used, host_buf_used);
        }

        return_encoded_errno(e)
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

        let vmctx = &mut *vmctx;
        let curfds = get_curfds(vmctx);
        let fd = decode_fd(fd);
        let (path, path_len) = match decode_char_slice(vmctx, path, path_len) {
            Ok((path, path_len)) => (path, path_len),
            Err(e) => return return_encoded_errno(e),
        };
        let (buf, buf_len) = match decode_char_slice(vmctx, buf, buf_len) {
            Ok((buf, buf_len)) => (buf, buf_len),
            Err(e) => return return_encoded_errno(e),
        };
        let mut host_buf_used = 0;
        if let Err(e) = decode_usize_byref(vmctx, buf_used) {
            return return_encoded_errno(e);
        }

        trace!("     | (path,path_len)={:?}", str_for_trace(path, path_len));

        let e = host::wasmtime_ssp_path_readlink(
            curfds,
            fd,
            path,
            path_len,
            buf,
            buf_len,
            &mut host_buf_used,
        );

        if u32::from(e) == host::__WASI_ESUCCESS {
            trace!("     | (buf,*buf_used)={:?}", str_for_trace(buf, host_buf_used));
            trace!("     | *buf_used={:?}", host_buf_used);
            encode_usize_byref(vmctx, buf_used, host_buf_used);
        }

        return_encoded_errno(e)
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

        let vmctx = &mut *vmctx;
        let curfds = get_curfds(vmctx);
        let fd0 = decode_fd(fd0);
        let (path0, path_len0) = match decode_char_slice(vmctx, path0, path_len0) {
            Ok((path0, path_len0)) => (path0, path_len0),
            Err(e) => return return_encoded_errno(e),
        };
        let fd1 = decode_fd(fd1);
        let (path1, path_len1) = match decode_char_slice(vmctx, path1, path_len1) {
            Ok((path1, path_len1)) => (path1, path_len1),
            Err(e) => return return_encoded_errno(e),
        };

        trace!("     | (path0,path_len0)={:?}", str_for_trace(path0, path_len0));
        trace!("     | (path1,path_len1)={:?}", str_for_trace(path1, path_len1));

        let e = host::wasmtime_ssp_path_rename(curfds, fd0, path0, path_len0, fd1, path1, path_len1);

        return_encoded_errno(e)
    }

    pub unsafe extern "C" fn fd_filestat_get(
        vmctx: *mut VMContext,
        fd: wasm32::__wasi_fd_t,
        buf: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t {
        trace!("fd_filestat_get(fd={:?}, buf={:#x?})", fd, buf);

        let vmctx = &mut *vmctx;
        let curfds = get_curfds(vmctx);
        let fd = decode_fd(fd);
        let mut host_buf = std::mem::zeroed();
        if let Err(e) = decode_filestat_byref(vmctx, buf) {
            return return_encoded_errno(e);
        }

        let e = host::wasmtime_ssp_fd_filestat_get(curfds, fd, &mut host_buf);

        if u32::from(e) == host::__WASI_ESUCCESS {
            trace!("     | *buf={:?}", host_buf);
            encode_filestat_byref(vmctx, buf, host_buf);
        }

        return_encoded_errno(e)
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

        let vmctx = &mut *vmctx;
        let curfds = get_curfds(vmctx);
        let fd = decode_fd(fd);
        let st_atim = decode_timestamp(st_atim);
        let st_mtim = decode_timestamp(st_mtim);
        let fstflags = decode_fstflags(fstflags);

        let e = host::wasmtime_ssp_fd_filestat_set_times(curfds, fd, st_atim, st_mtim, fstflags);

        return_encoded_errno(e)
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

        let vmctx = &mut *vmctx;
        let curfds = get_curfds(vmctx);
        let fd = decode_fd(fd);
        let size = decode_filesize(size);

        let e = host::wasmtime_ssp_fd_filestat_set_size(curfds, fd, size);

        return_encoded_errno(e)
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

        let vmctx = &mut *vmctx;
        let curfds = get_curfds(vmctx);
        let fd = decode_fd(fd);
        let flags = decode_lookupflags(flags);
        let (path, path_len) = match decode_char_slice(vmctx, path, path_len) {
            Ok((path, path_len)) => (path, path_len),
            Err(e) => return return_encoded_errno(e),
        };
        let mut host_buf = std::mem::zeroed();
        if let Err(e) = decode_filestat_byref(vmctx, buf) {
            return return_encoded_errno(e);
        }

        trace!("     | (path,path_len)={:?}", str_for_trace(path, path_len));

        let e = host::wasmtime_ssp_path_filestat_get(curfds, fd, flags, path, path_len, &mut host_buf);

        if u32::from(e) == host::__WASI_ESUCCESS {
            trace!("     | *buf={:?}", host_buf);
            encode_filestat_byref(vmctx, buf, host_buf);
        }

        return_encoded_errno(e)
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

        let vmctx = &mut *vmctx;
        let curfds = get_curfds(vmctx);
        let fd = decode_fd(fd);
        let flags = decode_lookupflags(flags);
        let (path, path_len) = match decode_char_slice(vmctx, path, path_len) {
            Ok((path, path_len)) => (path, path_len),
            Err(e) => return return_encoded_errno(e),
        };
        let st_atim = decode_timestamp(st_atim);
        let st_mtim = decode_timestamp(st_mtim);
        let fstflags = decode_fstflags(fstflags);

        trace!("     | (path,path_len)={:?}", str_for_trace(path, path_len));

        let e = host::wasmtime_ssp_path_filestat_set_times(curfds, fd, flags, path, path_len, st_atim, st_mtim, fstflags);

        return_encoded_errno(e)
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

        let vmctx = &mut *vmctx;
        let curfds = get_curfds(vmctx);
        let (path0, path_len0) = match decode_char_slice(vmctx, path0, path_len0) {
            Ok((path0, path_len0)) => (path0, path_len0),
            Err(e) => return return_encoded_errno(e),
        };
        let fd = decode_fd(fd);
        let (path1, path_len1) = match decode_char_slice(vmctx, path1, path_len1) {
            Ok((path1, path_len1)) => (path1, path_len1),
            Err(e) => return return_encoded_errno(e),
        };

        trace!("     | (path0,path_len0)={:?}", str_for_trace(path0, path_len0));
        trace!("     | (path1,path_len1)={:?}", str_for_trace(path1, path_len1));

        let e = host::wasmtime_ssp_path_symlink(curfds, path0, path_len0, fd, path1, path_len1);

        return_encoded_errno(e)
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

        let vmctx = &mut *vmctx;
        let curfds = get_curfds(vmctx);
        let fd = decode_fd(fd);
        let (path, path_len) = match decode_char_slice(vmctx, path, path_len) {
            Ok((path, path_len)) => (path, path_len),
            Err(e) => return return_encoded_errno(e),
        };

        trace!("     | (path,path_len)={:?}", str_for_trace(path, path_len));

        let e = host::wasmtime_ssp_path_unlink_file(curfds, fd, path, path_len);

        return_encoded_errno(e)
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

        let vmctx = &mut *vmctx;
        let curfds = get_curfds(vmctx);
        let fd = decode_fd(fd);
        let (path, path_len) = match decode_char_slice(vmctx, path, path_len) {
            Ok((path, path_len)) => (path, path_len),
            Err(e) => return return_encoded_errno(e),
        };

        trace!("     | (path,path_len)={:?}", str_for_trace(path, path_len));

        let e = host::wasmtime_ssp_path_remove_directory(curfds, fd, path, path_len);

        return_encoded_errno(e)
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

        let vmctx = &mut *vmctx;
        let curfds = get_curfds(vmctx);
        let in_ = match decode_subscription_slice(vmctx, in_, nsubscriptions) {
            Ok(in_) => in_,
            Err(e) => return return_encoded_errno(e),
        };
        let (out, out_len) = match decode_event_slice(vmctx, out, nsubscriptions) {
            Ok((out, out_len)) => (out, out_len),
            Err(e) => return return_encoded_errno(e),
        };
        let mut host_out = Vec::new();
        host_out.resize(out_len, mem::zeroed());
        let mut host_nevents = 0;
        if let Err(e) = decode_usize_byref(vmctx, nevents) {
            return return_encoded_errno(e);
        }

        assert_eq!(in_.len(), host_out.len());

        let e = host::wasmtime_ssp_poll_oneoff(
            curfds,
            in_.as_ptr(),
            host_out.as_mut_ptr(),
            in_.len(),
            &mut host_nevents,
        );

        if u32::from(e) == host::__WASI_ESUCCESS {
            trace!("     | *nevents={:?}", host_nevents);
            encode_usize_byref(vmctx, nevents, host_nevents);

            host_out.truncate(host_nevents);
            if log_enabled!(log::Level::Trace) {
                for (index, _event) in host_out.iter().enumerate() {
                    // TODO: Format the output for tracing.
                    trace!("     | *out[{}]=...", index);
                }
            }
            encode_event_slice(out, host_out);
        }

        return_encoded_errno(e)
    }

    pub unsafe extern "C" fn proc_exit(_vmctx: *mut VMContext, rval: u32,) -> () {
        trace!("proc_exit(rval={:?})", rval);

        let rval = decode_exitcode(rval);

        // TODO: Rather than call __wasi_proc_exit here, we should trigger a
        // stack unwind similar to a trap.
        host::wasmtime_ssp_proc_exit(rval);
    }

    pub unsafe extern "C" fn proc_raise(
        _vmctx: *mut VMContext,
        _sig: wasm32::__wasi_signal_t,
    ) -> wasm32::__wasi_errno_t {
        unimplemented!("__wasi_proc_raise");
    }

    pub unsafe extern "C" fn random_get(
        vmctx: *mut VMContext,
        buf: wasm32::uintptr_t,
        buf_len: wasm32::size_t,
    ) -> wasm32::__wasi_errno_t {
        trace!("random_get(buf={:#x?}, buf_len={:?})", buf, buf_len);

        let vmctx = &mut *vmctx;
        let (buf, buf_len) = match decode_char_slice(vmctx, buf, buf_len) {
            Ok((buf, buf_len)) => (buf, buf_len),
            Err(e) => return return_encoded_errno(e),
        };

        let e = host::wasmtime_ssp_random_get(buf as *mut host::void, buf_len);

        return_encoded_errno(e)
    }

    pub unsafe extern "C" fn sched_yield(_vmctx: *mut VMContext,) -> wasm32::__wasi_errno_t {
        let e = host::wasmtime_ssp_sched_yield();

        return_encoded_errno(e)
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

        let vmctx = &mut *vmctx;
        let curfds = get_curfds(vmctx);
        let sock = decode_fd(sock);
        let ri_data = match decode_iovec_slice(vmctx, ri_data, ri_data_len) {
            Ok(ri_data) => ri_data,
            Err(e) => return return_encoded_errno(e),
        };
        let ri_flags = decode_riflags(ri_flags);
        let mut host_ro_datalen = 0;
        if let Err(e) = decode_usize_byref(vmctx, ro_datalen) {
            return return_encoded_errno(e);
        }
        let mut host_ro_flags = 0;
        if let Err(e) = decode_roflags_byref(vmctx, ro_flags) {
            return return_encoded_errno(e);
        }

        let e = host::wasmtime_ssp_sock_recv(curfds, sock, ri_data.as_ptr(), ri_data.len(), ri_flags,
                                             &mut host_ro_datalen, &mut host_ro_flags);

        if u32::from(e) == host::__WASI_ESUCCESS {
            // TODO: Format the output for tracing.
            trace!("     | *ro_datalen={}", host_ro_datalen);
            trace!("     | *ro_flags={}", host_ro_flags);
            encode_usize_byref(vmctx, ro_datalen, host_ro_datalen);
            encode_roflags_byref(vmctx, ro_flags, host_ro_flags);
        }

        return_encoded_errno(e)
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

        let vmctx = &mut *vmctx;
        let curfds = get_curfds(vmctx);
        let sock = decode_fd(sock);
        let si_data = match decode_ciovec_slice(vmctx, si_data, si_data_len) {
            Ok(si_data) => si_data,
            Err(e) => return return_encoded_errno(e),
        };
        let si_flags = decode_siflags(si_flags);
        let mut host_so_datalen = 0;
        if let Err(e) = decode_usize_byref(vmctx, so_datalen) {
            return return_encoded_errno(e);
        }

        let e = host::wasmtime_ssp_sock_send(curfds, sock, si_data.as_ptr(), si_data.len(), si_flags, &mut host_so_datalen);

        if u32::from(e) == host::__WASI_ESUCCESS {
            trace!("     | *so_datalen={:?}", host_so_datalen);
            encode_usize_byref(vmctx, so_datalen, host_so_datalen);
        }

        return_encoded_errno(e)
    }

    pub unsafe extern "C" fn sock_shutdown(
        vmctx: *mut VMContext,
        sock: wasm32::__wasi_fd_t,
        how: wasm32::__wasi_sdflags_t,
    ) -> wasm32::__wasi_errno_t {
        trace!("sock_shutdown(sock={:?}, how={:?})", sock, how);

        let vmctx = &mut *vmctx;
        let curfds = get_curfds(vmctx);
        let sock = decode_fd(sock);
        let how = decode_sdflags(how);

        let e = host::wasmtime_ssp_sock_shutdown(curfds, sock, how);

        return_encoded_errno(e)
    }
}
