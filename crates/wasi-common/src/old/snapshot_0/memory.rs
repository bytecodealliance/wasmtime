//! Functions to store and load data to and from wasm linear memory,
//! transforming them from and to host data types.
//!
//! Endianness concerns are completely encapsulated in this file, so
//! that users outside this file holding a `wasi::*` value never need
//! to consider what endianness it's in. Inside this file,
//! wasm linear-memory-ordered values are called "raw" values, and
//! are not held for long durations.

#![allow(unused)]
use crate::old::snapshot_0::{host, wasi, wasi32, Error, Result};
use num::PrimInt;
use std::convert::TryFrom;
use std::mem::{align_of, size_of};
use std::{ptr, slice};

fn dec_ptr(memory: &[u8], ptr: wasi32::uintptr_t, len: usize) -> Result<*const u8> {
    // check for overflow
    let checked_len = (ptr as usize).checked_add(len).ok_or(Error::EFAULT)?;

    // translate the pointer
    memory
        .get(ptr as usize..checked_len)
        .ok_or(Error::EFAULT)
        .map(|mem| mem.as_ptr())
}

fn dec_ptr_mut(memory: &mut [u8], ptr: wasi32::uintptr_t, len: usize) -> Result<*mut u8> {
    // check for overflow
    let checked_len = (ptr as usize).checked_add(len).ok_or(Error::EFAULT)?;

    // translate the pointer
    memory
        .get_mut(ptr as usize..checked_len)
        .ok_or(Error::EFAULT)
        .map(|mem| mem.as_mut_ptr())
}

fn dec_ptr_to<'memory, T>(memory: &'memory [u8], ptr: wasi32::uintptr_t) -> Result<&'memory T> {
    // check that the ptr is aligned
    if ptr as usize % align_of::<T>() != 0 {
        return Err(Error::EINVAL);
    }

    dec_ptr(memory, ptr, size_of::<T>()).map(|p| unsafe { &*(p as *const T) })
}

fn dec_ptr_to_mut<'memory, T>(
    memory: &'memory mut [u8],
    ptr: wasi32::uintptr_t,
) -> Result<&'memory mut T> {
    // check that the ptr is aligned
    if ptr as usize % align_of::<T>() != 0 {
        return Err(Error::EINVAL);
    }

    dec_ptr_mut(memory, ptr, size_of::<T>()).map(|p| unsafe { &mut *(p as *mut T) })
}

/// This function does not perform endianness conversions!
fn dec_raw_byref<T>(memory: &[u8], ptr: wasi32::uintptr_t) -> Result<T> {
    dec_ptr_to::<T>(memory, ptr).map(|p| unsafe { ptr::read(p) })
}

/// This function does not perform endianness conversions!
fn enc_raw_byref<T>(memory: &mut [u8], ptr: wasi32::uintptr_t, t: T) -> Result<()> {
    dec_ptr_to_mut::<T>(memory, ptr).map(|p| unsafe { ptr::write(p, t) })
}

pub(crate) fn dec_int_byref<T>(memory: &[u8], ptr: wasi32::uintptr_t) -> Result<T>
where
    T: PrimInt,
{
    dec_raw_byref::<T>(memory, ptr).map(|i| PrimInt::from_le(i))
}

pub(crate) fn enc_int_byref<T>(memory: &mut [u8], ptr: wasi32::uintptr_t, t: T) -> Result<()>
where
    T: PrimInt,
{
    enc_raw_byref::<T>(memory, ptr, PrimInt::to_le(t))
}

fn check_slice_of<T>(ptr: wasi32::uintptr_t, len: wasi32::size_t) -> Result<(usize, usize)> {
    // check alignment, and that length doesn't overflow
    if ptr as usize % align_of::<T>() != 0 {
        return Err(Error::EINVAL);
    }
    let len = dec_usize(len);
    let len_bytes = if let Some(len) = size_of::<T>().checked_mul(len) {
        len
    } else {
        return Err(Error::EOVERFLOW);
    };

    Ok((len, len_bytes))
}

fn dec_raw_slice_of<'memory, T>(
    memory: &'memory [u8],
    ptr: wasi32::uintptr_t,
    len: wasi32::size_t,
) -> Result<&'memory [T]> {
    let (len, len_bytes) = check_slice_of::<T>(ptr, len)?;
    let ptr = dec_ptr(memory, ptr, len_bytes)? as *const T;
    Ok(unsafe { slice::from_raw_parts(ptr, len) })
}

fn dec_raw_slice_of_mut<'memory, T>(
    memory: &'memory mut [u8],
    ptr: wasi32::uintptr_t,
    len: wasi32::size_t,
) -> Result<&'memory mut [T]> {
    let (len, len_bytes) = check_slice_of::<T>(ptr, len)?;
    let ptr = dec_ptr_mut(memory, ptr, len_bytes)? as *mut T;
    Ok(unsafe { slice::from_raw_parts_mut(ptr, len) })
}

fn raw_slice_for_enc<'memory, T>(
    memory: &'memory mut [u8],
    slice: &[T],
    ptr: wasi32::uintptr_t,
) -> Result<&'memory mut [T]> {
    // check alignment
    if ptr as usize % align_of::<T>() != 0 {
        return Err(Error::EINVAL);
    }
    // check that length doesn't overflow
    let len_bytes = if let Some(len) = size_of::<T>().checked_mul(slice.len()) {
        len
    } else {
        return Err(Error::EOVERFLOW);
    };

    // get the pointer into guest memory
    let ptr = dec_ptr_mut(memory, ptr, len_bytes)? as *mut T;

    Ok(unsafe { slice::from_raw_parts_mut(ptr, slice.len()) })
}

pub(crate) fn dec_slice_of_u8<'memory>(
    memory: &'memory [u8],
    ptr: wasi32::uintptr_t,
    len: wasi32::size_t,
) -> Result<&'memory [u8]> {
    dec_raw_slice_of::<u8>(memory, ptr, len)
}

pub(crate) fn dec_slice_of_mut_u8<'memory>(
    memory: &'memory mut [u8],
    ptr: wasi32::uintptr_t,
    len: wasi32::size_t,
) -> Result<&'memory mut [u8]> {
    dec_raw_slice_of_mut::<u8>(memory, ptr, len)
}

pub(crate) fn enc_slice_of_u8(
    memory: &mut [u8],
    slice: &[u8],
    ptr: wasi32::uintptr_t,
) -> Result<()> {
    let output = raw_slice_for_enc::<u8>(memory, slice, ptr)?;

    output.copy_from_slice(slice);

    Ok(())
}

pub(crate) fn enc_slice_of_wasi32_uintptr(
    memory: &mut [u8],
    slice: &[wasi32::uintptr_t],
    ptr: wasi32::uintptr_t,
) -> Result<()> {
    let mut output_iter = raw_slice_for_enc::<wasi32::uintptr_t>(memory, slice, ptr)?.into_iter();

    for p in slice {
        *output_iter.next().unwrap() = PrimInt::to_le(*p);
    }

    Ok(())
}

macro_rules! dec_enc_scalar {
    ($ty:ident, $dec_byref:ident, $enc_byref:ident) => {
        pub(crate) fn $dec_byref(memory: &mut [u8], ptr: wasi32::uintptr_t) -> Result<wasi::$ty> {
            dec_int_byref::<wasi::$ty>(memory, ptr)
        }

        pub(crate) fn $enc_byref(
            memory: &mut [u8],
            ptr: wasi32::uintptr_t,
            x: wasi::$ty,
        ) -> Result<()> {
            enc_int_byref::<wasi::$ty>(memory, ptr, x)
        }
    };
}

pub(crate) fn dec_ciovec_slice(
    memory: &[u8],
    ptr: wasi32::uintptr_t,
    len: wasi32::size_t,
) -> Result<Vec<host::__wasi_ciovec_t>> {
    let raw_slice = dec_raw_slice_of::<wasi32::__wasi_ciovec_t>(memory, ptr, len)?;

    raw_slice
        .iter()
        .map(|raw_iov| {
            let len = dec_usize(PrimInt::from_le(raw_iov.buf_len));
            let buf = PrimInt::from_le(raw_iov.buf);
            Ok(host::__wasi_ciovec_t {
                buf: dec_ptr(memory, buf, len)? as *const u8,
                buf_len: len,
            })
        })
        .collect()
}

pub(crate) fn dec_iovec_slice(
    memory: &[u8],
    ptr: wasi32::uintptr_t,
    len: wasi32::size_t,
) -> Result<Vec<host::__wasi_iovec_t>> {
    let raw_slice = dec_raw_slice_of::<wasi32::__wasi_iovec_t>(memory, ptr, len)?;

    raw_slice
        .iter()
        .map(|raw_iov| {
            let len = dec_usize(PrimInt::from_le(raw_iov.buf_len));
            let buf = PrimInt::from_le(raw_iov.buf);
            Ok(host::__wasi_iovec_t {
                buf: dec_ptr(memory, buf, len)? as *mut u8,
                buf_len: len,
            })
        })
        .collect()
}

dec_enc_scalar!(__wasi_clockid_t, dec_clockid_byref, enc_clockid_byref);
dec_enc_scalar!(__wasi_errno_t, dec_errno_byref, enc_errno_byref);
dec_enc_scalar!(__wasi_exitcode_t, dec_exitcode_byref, enc_exitcode_byref);
dec_enc_scalar!(__wasi_fd_t, dec_fd_byref, enc_fd_byref);
dec_enc_scalar!(__wasi_fdflags_t, dec_fdflags_byref, enc_fdflags_byref);
dec_enc_scalar!(__wasi_device_t, dev_device_byref, enc_device_byref);
dec_enc_scalar!(__wasi_inode_t, dev_inode_byref, enc_inode_byref);
dec_enc_scalar!(__wasi_linkcount_t, dev_linkcount_byref, enc_linkcount_byref);

pub(crate) fn dec_filestat_byref(
    memory: &mut [u8],
    filestat_ptr: wasi32::uintptr_t,
) -> Result<wasi::__wasi_filestat_t> {
    let raw = dec_raw_byref::<wasi::__wasi_filestat_t>(memory, filestat_ptr)?;

    Ok(wasi::__wasi_filestat_t {
        dev: PrimInt::from_le(raw.dev),
        ino: PrimInt::from_le(raw.ino),
        filetype: PrimInt::from_le(raw.filetype),
        nlink: PrimInt::from_le(raw.nlink),
        size: PrimInt::from_le(raw.size),
        atim: PrimInt::from_le(raw.atim),
        mtim: PrimInt::from_le(raw.mtim),
        ctim: PrimInt::from_le(raw.ctim),
    })
}

pub(crate) fn enc_filestat_byref(
    memory: &mut [u8],
    filestat_ptr: wasi32::uintptr_t,
    filestat: wasi::__wasi_filestat_t,
) -> Result<()> {
    let raw = wasi::__wasi_filestat_t {
        dev: PrimInt::to_le(filestat.dev),
        ino: PrimInt::to_le(filestat.ino),
        filetype: PrimInt::to_le(filestat.filetype),
        nlink: PrimInt::to_le(filestat.nlink),
        size: PrimInt::to_le(filestat.size),
        atim: PrimInt::to_le(filestat.atim),
        mtim: PrimInt::to_le(filestat.mtim),
        ctim: PrimInt::to_le(filestat.ctim),
    };

    enc_raw_byref::<wasi::__wasi_filestat_t>(memory, filestat_ptr, raw)
}

pub(crate) fn dec_fdstat_byref(
    memory: &mut [u8],
    fdstat_ptr: wasi32::uintptr_t,
) -> Result<wasi::__wasi_fdstat_t> {
    let raw = dec_raw_byref::<wasi::__wasi_fdstat_t>(memory, fdstat_ptr)?;

    Ok(wasi::__wasi_fdstat_t {
        fs_filetype: PrimInt::from_le(raw.fs_filetype),
        fs_flags: PrimInt::from_le(raw.fs_flags),
        fs_rights_base: PrimInt::from_le(raw.fs_rights_base),
        fs_rights_inheriting: PrimInt::from_le(raw.fs_rights_inheriting),
    })
}

pub(crate) fn enc_fdstat_byref(
    memory: &mut [u8],
    fdstat_ptr: wasi32::uintptr_t,
    fdstat: wasi::__wasi_fdstat_t,
) -> Result<()> {
    let raw = wasi::__wasi_fdstat_t {
        fs_filetype: PrimInt::to_le(fdstat.fs_filetype),
        fs_flags: PrimInt::to_le(fdstat.fs_flags),
        fs_rights_base: PrimInt::to_le(fdstat.fs_rights_base),
        fs_rights_inheriting: PrimInt::to_le(fdstat.fs_rights_inheriting),
    };

    enc_raw_byref::<wasi::__wasi_fdstat_t>(memory, fdstat_ptr, raw)
}

dec_enc_scalar!(__wasi_filedelta_t, dec_filedelta_byref, enc_filedelta_byref);
dec_enc_scalar!(__wasi_filesize_t, dec_filesize_byref, enc_filesize_byref);
dec_enc_scalar!(__wasi_filetype_t, dec_filetype_byref, enc_filetype_byref);

dec_enc_scalar!(
    __wasi_lookupflags_t,
    dec_lookupflags_byref,
    enc_lookupflags_byref
);

dec_enc_scalar!(__wasi_oflags_t, dec_oflags_byref, enc_oflags_byref);

pub(crate) fn dec_prestat_byref(
    memory: &mut [u8],
    prestat_ptr: wasi32::uintptr_t,
) -> Result<host::__wasi_prestat_t> {
    let raw = dec_raw_byref::<wasi32::__wasi_prestat_t>(memory, prestat_ptr)?;

    match PrimInt::from_le(raw.tag) {
        wasi::__WASI_PREOPENTYPE_DIR => Ok(host::__wasi_prestat_t {
            tag: wasi::__WASI_PREOPENTYPE_DIR,
            u: host::__wasi_prestat_u_t {
                dir: host::__wasi_prestat_dir_t {
                    pr_name_len: dec_usize(PrimInt::from_le(unsafe { raw.u.dir.pr_name_len })),
                },
            },
        }),
        _ => Err(Error::EINVAL),
    }
}

pub(crate) fn enc_prestat_byref(
    memory: &mut [u8],
    prestat_ptr: wasi32::uintptr_t,
    prestat: host::__wasi_prestat_t,
) -> Result<()> {
    let raw = match prestat.tag {
        wasi::__WASI_PREOPENTYPE_DIR => Ok(wasi32::__wasi_prestat_t {
            tag: PrimInt::to_le(wasi::__WASI_PREOPENTYPE_DIR),
            u: wasi32::__wasi_prestat_u_t {
                dir: wasi32::__wasi_prestat_dir_t {
                    pr_name_len: enc_usize(unsafe { prestat.u.dir.pr_name_len }),
                },
            },
        }),
        _ => Err(Error::EINVAL),
    }?;

    enc_raw_byref::<wasi32::__wasi_prestat_t>(memory, prestat_ptr, raw)
}

dec_enc_scalar!(__wasi_rights_t, dec_rights_byref, enc_rights_byref);
dec_enc_scalar!(__wasi_timestamp_t, dec_timestamp_byref, enc_timestamp_byref);

pub(crate) fn dec_usize(size: wasi32::size_t) -> usize {
    usize::try_from(size).unwrap()
}

pub(crate) fn enc_usize(size: usize) -> wasi32::size_t {
    wasi32::size_t::try_from(size).unwrap()
}

pub(crate) fn enc_usize_byref(
    memory: &mut [u8],
    usize_ptr: wasi32::uintptr_t,
    host_usize: usize,
) -> Result<()> {
    enc_int_byref::<wasi32::size_t>(memory, usize_ptr, enc_usize(host_usize))
}

dec_enc_scalar!(__wasi_whence_t, dec_whence_byref, enc_whence_byref);

dec_enc_scalar!(
    __wasi_subclockflags_t,
    dec_subclockflags_byref,
    enc_subclockflags_byref
);

dec_enc_scalar!(
    __wasi_eventrwflags_t,
    dec_eventrwflags_byref,
    enc_eventrwflags_byref
);

dec_enc_scalar!(__wasi_eventtype_t, dec_eventtype_byref, enc_eventtype_byref);
dec_enc_scalar!(__wasi_userdata_t, dec_userdata_byref, enc_userdata_byref);

pub(crate) fn dec_subscriptions(
    memory: &mut [u8],
    input: wasi32::uintptr_t,
    nsubscriptions: wasi32::size_t,
) -> Result<Vec<wasi::__wasi_subscription_t>> {
    let raw_input_slice =
        dec_raw_slice_of::<wasi::__wasi_subscription_t>(memory, input, nsubscriptions)?;

    raw_input_slice
        .into_iter()
        .map(|raw_subscription| {
            let userdata = PrimInt::from_le(raw_subscription.userdata);
            let tag = PrimInt::from_le(raw_subscription.u.tag);
            let raw_u = raw_subscription.u.u;
            let u = match tag {
                wasi::__WASI_EVENTTYPE_CLOCK => wasi::__wasi_subscription_u_u_t {
                    clock: unsafe {
                        wasi::__wasi_subscription_clock_t {
                            identifier: PrimInt::from_le(raw_u.clock.identifier),
                            id: PrimInt::from_le(raw_u.clock.id),
                            timeout: PrimInt::from_le(raw_u.clock.timeout),
                            precision: PrimInt::from_le(raw_u.clock.precision),
                            flags: PrimInt::from_le(raw_u.clock.flags),
                        }
                    },
                },
                wasi::__WASI_EVENTTYPE_FD_READ => wasi::__wasi_subscription_u_u_t {
                    fd_read: wasi::__wasi_subscription_fd_readwrite_t {
                        file_descriptor: PrimInt::from_le(unsafe { raw_u.fd_read.file_descriptor }),
                    },
                },
                wasi::__WASI_EVENTTYPE_FD_WRITE => wasi::__wasi_subscription_u_u_t {
                    fd_write: wasi::__wasi_subscription_fd_readwrite_t {
                        file_descriptor: PrimInt::from_le(unsafe {
                            raw_u.fd_write.file_descriptor
                        }),
                    },
                },
                _ => return Err(Error::EINVAL),
            };
            Ok(wasi::__wasi_subscription_t {
                userdata,
                u: wasi::__wasi_subscription_u_t { tag, u },
            })
        })
        .collect::<Result<Vec<_>>>()
}

pub(crate) fn enc_events(
    memory: &mut [u8],
    output: wasi32::uintptr_t,
    nsubscriptions: wasi32::size_t,
    events: Vec<wasi::__wasi_event_t>,
) -> Result<()> {
    let mut raw_output_iter =
        dec_raw_slice_of_mut::<wasi::__wasi_event_t>(memory, output, nsubscriptions)?.into_iter();

    for event in events.iter() {
        *raw_output_iter
            .next()
            .expect("the number of events cannot exceed the number of subscriptions") = {
            let userdata = PrimInt::to_le(event.userdata);
            let error = PrimInt::to_le(event.error);
            let r#type = PrimInt::to_le(event.r#type);
            let flags = PrimInt::to_le(event.fd_readwrite.flags);
            let nbytes = PrimInt::to_le(event.fd_readwrite.nbytes);
            wasi::__wasi_event_t {
                userdata,
                error,
                r#type,
                fd_readwrite: wasi::__wasi_event_fd_readwrite_t { flags, nbytes },
            }
        };
    }

    Ok(())
}

dec_enc_scalar!(__wasi_advice_t, dec_advice_byref, enc_advice_byref);
dec_enc_scalar!(__wasi_fstflags_t, dec_fstflags_byref, enc_fstflags_byref);
dec_enc_scalar!(__wasi_dircookie_t, dec_dircookie_byref, enc_dircookie_byref);
