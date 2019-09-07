macro_rules! hostcalls {
    ($(pub fn $name:ident($($arg:ident: $ty:ty,)*) -> $ret:ty;)*) => ($(
            #[wasi_common_cbindgen::wasi_common_cbindgen]
            pub fn $name($($arg: $ty,)*) -> $ret {
                let ret = match crate::hostcalls_impl::$name($($arg,)*) {
                    Ok(()) => crate::host::__WASI_ESUCCESS,
                    Err(e) => e.as_wasi_errno(),
                };

                crate::hostcalls::return_enc_errno(ret)
            }
    )*)
}
