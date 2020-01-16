macro_rules! hostcalls {
    ($(pub unsafe fn $name:ident($($arg:ident: $ty:ty,)*) -> $ret:ty;)*) => ($(
            #[wasi_common_cbindgen::wasi_common_cbindgen]
            pub unsafe fn $name($($arg: $ty,)*) -> $ret {
                let ret = crate::hostcalls_impl::$name($($arg,)*)
                    .err()
                    .unwrap_or(crate::Error::ESUCCESS)
                    .as_wasi_error();
                log::trace!("     | errno={}", ret);
                ret.as_raw_errno()
            }
    )*)
}

// Like `hostcalls`, but uses `wasi_common_cbindgen_old`, which means
// it doesn't declare a non-mangled function name.
macro_rules! hostcalls_old {
    ($(pub unsafe fn $name:ident($($arg:ident: $ty:ty,)*) -> $ret:ty;)*) => ($(
            #[wasi_common_cbindgen::wasi_common_cbindgen_old]
            pub unsafe fn $name($($arg: $ty,)*) -> $ret {
                let ret = crate::old::snapshot_0::hostcalls_impl::$name($($arg,)*)
                    .err()
                    .unwrap_or(crate::old::snapshot_0::Error::ESUCCESS)
                    .as_wasi_error();
                log::trace!("     | errno={}", ret);
                ret.as_raw_errno()
            }
    )*)
}
