//! WASI host types. These are types that contain raw pointers and `usize`
//! values, and so are platform-specific.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use crate::wasi::*;
use std::{io, slice};
use wig::witx_host_types;

witx_host_types!("snapshot" "wasi_snapshot_preview1");

pub(crate) unsafe fn ciovec_to_host(ciovec: &__wasi_ciovec_t) -> io::IoSlice {
    let slice = slice::from_raw_parts(ciovec.buf as *const u8, ciovec.buf_len);
    io::IoSlice::new(slice)
}

pub(crate) unsafe fn iovec_to_host_mut(iovec: &mut __wasi_iovec_t) -> io::IoSliceMut {
    let slice = slice::from_raw_parts_mut(iovec.buf as *mut u8, iovec.buf_len);
    io::IoSliceMut::new(slice)
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn bindgen_test_layout___wasi_prestat_t() {
        assert_eq!(
            ::std::mem::size_of::<__wasi_prestat_t>(),
            16usize,
            concat!("Size of: ", stringify!(__wasi_prestat_t))
        );
        assert_eq!(
            ::std::mem::align_of::<__wasi_prestat_t>(),
            8usize,
            concat!("Alignment of ", stringify!(__wasi_prestat_t))
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_prestat_t>())).pr_type as *const _ as usize },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_prestat_t),
                "::",
                stringify!(pr_type)
            )
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_prestat_t>())).u as *const _ as usize },
            8usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_prestat_t),
                "::",
                stringify!(u)
            )
        );
    }

    #[test]
    fn bindgen_test_layout___wasi_prestat_t___wasi_prestat_u___wasi_prestat_u_dir_t() {
        assert_eq!(
            ::std::mem::size_of::<__wasi_prestat_dir_t>(),
            8usize,
            concat!("Size of: ", stringify!(__wasi_prestat_dir_t))
        );
        assert_eq!(
            ::std::mem::align_of::<__wasi_prestat_dir_t>(),
            8usize,
            concat!("Alignment of ", stringify!(__wasi_prestat_dir_t))
        );
        assert_eq!(
            unsafe {
                &(*(::std::ptr::null::<__wasi_prestat_dir_t>())).pr_name_len as *const _ as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_prestat_dir_t),
                "::",
                stringify!(pr_name_len)
            )
        );
    }

    #[test]
    fn bindgen_test_layout___wasi_prestat_t___wasi_prestat_u() {
        assert_eq!(
            ::std::mem::size_of::<__wasi_prestat_u_t>(),
            8usize,
            concat!("Size of: ", stringify!(__wasi_prestat_u))
        );
        assert_eq!(
            ::std::mem::align_of::<__wasi_prestat_u_t>(),
            8usize,
            concat!("Alignment of ", stringify!(__wasi_prestat_u))
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_prestat_u_t>())).dir as *const _ as usize },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_prestat_u_t),
                "::",
                stringify!(dir)
            )
        );
    }
}
