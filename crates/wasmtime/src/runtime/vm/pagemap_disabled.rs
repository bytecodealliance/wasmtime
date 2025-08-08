use crate::runtime::vm::HostAlignedByteCount;
use core::slice;

#[derive(Debug)]
pub enum PageMap {}

impl PageMap {
    #[allow(dead_code, reason = "not used on linux64")]
    pub fn new() -> Option<PageMap> {
        None
    }
}

/// Resets `ptr` for `len` bytes.
///
/// # Safety
///
/// Requires that `ptr` is valid to read and write for `len` bytes.
pub unsafe fn reset_with_pagemap(
    _pagemap: Option<&PageMap>,
    mut ptr: *mut u8,
    mut len: HostAlignedByteCount,
    mut keep_resident: HostAlignedByteCount,
    mut reset_manually: impl FnMut(&mut [u8]),
    mut decommit: impl FnMut(*mut u8, usize),
) {
    keep_resident = keep_resident.min(len);

    // `memset` the first `keep_resident` bytes.
    //
    // SAFETY: it's a contract of this function that `ptr` is valid to write for
    // `len` bytes, and `keep_resident` is less than `len` here.
    unsafe {
        reset_manually(slice::from_raw_parts_mut(ptr, keep_resident.byte_count()));
    }

    // SAFETY: It's a contract of this function that the parameters are valid to
    // always produce an in-bounds pointer.
    unsafe {
        ptr = ptr.add(keep_resident.byte_count());
    }
    len = len.checked_sub(keep_resident).unwrap();

    // decommit the rest of it.
    decommit(ptr, len.byte_count())
}
