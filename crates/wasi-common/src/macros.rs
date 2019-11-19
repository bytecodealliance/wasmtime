macro_rules! hostcalls {
    ($(pub unsafe fn $name:ident($($arg:ident: $ty:ty,)*) -> $ret:ty;)*) => ($(
            #[wasi_common_cbindgen::wasi_common_cbindgen]
            pub unsafe fn $name($($arg: $ty,)*) -> $ret {
                let ret = match hostcalls_impl::$name($($arg,)*) {
                    Ok(()) => wasi::__WASI_ERRNO_SUCCESS,
                    Err(e) => e.as_wasi_errno(),
                };

                ret
            }
    )*)
}

// Like `hostcalls`, but uses `wasi_common_cbindgen_old`, which means
// it doesn't declare a non-mangled function name.
macro_rules! hostcalls_old {
    ($(pub unsafe fn $name:ident($($arg:ident: $ty:ty,)*) -> $ret:ty;)*) => ($(
            #[wasi_common_cbindgen::wasi_common_cbindgen_old]
            pub unsafe fn $name($($arg: $ty,)*) -> $ret {
                let ret = match hostcalls_impl::$name($($arg,)*) {
                    Ok(()) => wasi::__WASI_ERRNO_SUCCESS,
                    Err(e) => e.as_wasi_errno(),
                };

                ret
            }
    )*)
}
