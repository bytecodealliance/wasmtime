use crate::{host, wasm32};
use log::{debug, error};
use more_asserts::assert_le;
use std::convert::TryFrom;
use std::mem::{align_of, size_of, zeroed};
use std::slice;
use wasmtime_runtime::{Export, VMContext};

/// Translate a wasm pointer into a native pointer.
///
/// This is unsafe due to trusting the contents of vmctx. The pointer result
/// is bounds and alignment checked.
unsafe fn decode_ptr(
    vmctx: &mut VMContext,
    ptr: wasm32::uintptr_t,
    len: usize,
    align: usize,
) -> Result<*mut u8, host::__wasi_errno_t> {
    match vmctx.lookup_global_export("memory") {
        Some(Export::Memory {
            definition,
            vmctx: _,
            memory: _,
        }) => {
            if len > 0 {
                // Check for overflow within the access.
                let last = match (ptr as usize).checked_add(len - 1) {
                    Some(sum) => sum,
                    None => {
                        debug!("overflow in decode_ptr");
                        return Err(host::__WASI_EFAULT as host::__wasi_errno_t);
                    }
                };
                // Check for out of bounds.
                if last >= (*definition).current_length {
                    debug!("out of bounds in decode_ptr");
                    return Err(host::__WASI_EFAULT as host::__wasi_errno_t);
                }
            }
            // Check alignment.
            if (ptr as usize) % align != 0 {
                debug!("bad alignment in decode_ptr: {} % {}", ptr, align);
                return Err(host::__WASI_EINVAL as host::__wasi_errno_t);
            }
            // Ok, translate the address.
            Ok((((*definition).base as usize) + (ptr as usize)) as *mut u8)
        }
        // No export named "memory", or the export isn't a memory.
        // FIXME: Is EINVAL the best code here?
        x => {
            error!(
                "no export named \"memory\", or the export isn't a mem: {:?}",
                x
            );
            Err(host::__WASI_EINVAL as host::__wasi_errno_t)
        }
    }
}

unsafe fn decode_ptr_to<T>(
    vmctx: &mut VMContext,
    ptr: wasm32::uintptr_t,
) -> Result<*mut T, host::__wasi_errno_t> {
    decode_ptr(vmctx, ptr, size_of::<T>(), align_of::<T>()).map(|ptr| ptr as *mut T)
}

unsafe fn decode_pointee<T>(
    vmctx: &mut VMContext,
    ptr: wasm32::uintptr_t,
) -> Result<(), host::__wasi_errno_t> {
    // Check bounds and alignment.
    decode_ptr_to::<T>(vmctx, ptr)?;
    Ok(())
}

pub unsafe fn encode_pointee<T>(vmctx: &mut VMContext, ptr: wasm32::uintptr_t, t: T) {
    // Bounds and alignment are checked in `decode_pointee`.
    let ptr = decode_ptr_to::<T>(vmctx, ptr).unwrap();

    ptr.write(t);
}

pub unsafe fn decode_slice_of<T>(
    vmctx: &mut VMContext,
    ptr: wasm32::uintptr_t,
    len: wasm32::size_t,
) -> Result<(*mut T, usize), host::__wasi_errno_t> {
    let len = usize::try_from(len).unwrap();

    let ptr = decode_ptr(
        vmctx,
        ptr,
        size_of::<T>().checked_mul(len).unwrap(),
        align_of::<T>(),
    )? as *mut T;

    Ok((ptr, len))
}

pub fn encode_usize(len: usize) -> wasm32::size_t {
    u32::try_from(len).unwrap()
}

pub fn encode_device(device: host::__wasi_device_t) -> wasm32::__wasi_device_t {
    device
}

pub fn encode_inode(inode: host::__wasi_inode_t) -> wasm32::__wasi_inode_t {
    inode
}

pub fn encode_linkcount(linkcount: host::__wasi_linkcount_t) -> wasm32::__wasi_linkcount_t {
    linkcount
}

pub fn decode_userdata(userdata: wasm32::__wasi_userdata_t) -> host::__wasi_userdata_t {
    userdata
}

pub fn encode_userdata(userdata: host::__wasi_userdata_t) -> wasm32::__wasi_userdata_t {
    userdata
}

pub fn decode_eventtype(eventtype: wasm32::__wasi_eventtype_t) -> host::__wasi_eventtype_t {
    eventtype
}

pub fn encode_eventtype(eventtype: host::__wasi_eventtype_t) -> wasm32::__wasi_eventtype_t {
    eventtype
}

pub fn decode_filesize(filesize: wasm32::__wasi_filesize_t) -> host::__wasi_filesize_t {
    filesize
}

pub fn encode_filesize(filesize: host::__wasi_filesize_t) -> wasm32::__wasi_filesize_t {
    filesize
}

pub fn encode_eventrwflags(
    eventrwflags: host::__wasi_eventrwflags_t,
) -> wasm32::__wasi_eventrwflags_t {
    eventrwflags
}

pub fn decode_subclockflags(
    subclockflags: wasm32::__wasi_subclockflags_t,
) -> host::__wasi_subclockflags_t {
    subclockflags
}

pub fn decode_fd(fd: wasm32::__wasi_fd_t) -> host::__wasi_fd_t {
    fd
}

pub fn decode_filedelta(filedelta: wasm32::__wasi_filedelta_t) -> host::__wasi_filedelta_t {
    filedelta
}

pub fn decode_whence(whence: wasm32::__wasi_whence_t) -> host::__wasi_whence_t {
    whence
}

pub fn decode_clockid(clockid: wasm32::__wasi_clockid_t) -> host::__wasi_clockid_t {
    clockid
}

pub fn decode_timestamp(timestamp: wasm32::__wasi_timestamp_t) -> host::__wasi_timestamp_t {
    timestamp
}

pub fn encode_timestamp(timestamp: host::__wasi_timestamp_t) -> wasm32::__wasi_timestamp_t {
    timestamp
}

pub fn decode_exitcode(exitcode: wasm32::__wasi_exitcode_t) -> host::__wasi_exitcode_t {
    exitcode
}

pub fn decode_lookupflags(lookupflags: wasm32::__wasi_lookupflags_t) -> host::__wasi_lookupflags_t {
    lookupflags
}

pub fn decode_oflags(oflags: wasm32::__wasi_oflags_t) -> host::__wasi_oflags_t {
    oflags
}

pub fn decode_advice(advice: wasm32::__wasi_advice_t) -> host::__wasi_advice_t {
    advice
}

pub fn decode_dircookie(dircookie: wasm32::__wasi_dircookie_t) -> host::__wasi_dircookie_t {
    dircookie
}

pub fn encode_preopentype(preopentype: host::__wasi_preopentype_t) -> wasm32::__wasi_preopentype_t {
    preopentype
}

pub fn encode_filetype(filetype: host::__wasi_filetype_t) -> wasm32::__wasi_filetype_t {
    filetype
}

pub fn decode_fstflags(fstflags: wasm32::__wasi_fstflags_t) -> host::__wasi_fstflags_t {
    fstflags
}

#[allow(dead_code)]
pub fn encode_fstflags(fstflags: host::__wasi_fstflags_t) -> wasm32::__wasi_fstflags_t {
    fstflags
}

pub fn decode_fdflags(fdflags: wasm32::__wasi_fdflags_t) -> host::__wasi_fdflags_t {
    fdflags
}

pub fn encode_fdflags(fdflags: host::__wasi_fdflags_t) -> wasm32::__wasi_fdflags_t {
    fdflags
}

pub fn decode_sdflags(sdflags: wasm32::__wasi_sdflags_t) -> host::__wasi_sdflags_t {
    sdflags
}

pub fn decode_rights(rights: wasm32::__wasi_rights_t) -> host::__wasi_rights_t {
    rights
}

pub fn encode_rights(rights: host::__wasi_rights_t) -> wasm32::__wasi_rights_t {
    rights
}

pub fn decode_riflags(riflags: wasm32::__wasi_riflags_t) -> host::__wasi_riflags_t {
    riflags
}

pub fn decode_siflags(siflags: wasm32::__wasi_siflags_t) -> host::__wasi_siflags_t {
    siflags
}

pub unsafe fn decode_char_slice(
    vmctx: &mut VMContext,
    ptr: wasm32::uintptr_t,
    len: wasm32::size_t,
) -> Result<(*mut host::char, usize), host::__wasi_errno_t> {
    decode_slice_of::<wasm32::char>(vmctx, ptr, len)
}

pub unsafe fn decode_charstar_slice(
    vmctx: &mut VMContext,
    ptr: wasm32::uintptr_t,
    count: wasm32::size_t,
) -> Result<(*mut wasm32::uintptr_t, usize), host::__wasi_errno_t> {
    decode_slice_of::<wasm32::uintptr_t>(vmctx, ptr, count)
}

pub unsafe fn encode_charstar_slice(
    ptr: *mut wasm32::uintptr_t,
    host_vec: Vec<*mut libc::c_char>,
    guest_base: wasm32::uintptr_t,
    host_base: *mut libc::c_char,
) {
    for (i, host) in host_vec.iter().enumerate() {
        let guest = if host.is_null() {
            0
        } else {
            guest_base + (*host as usize - host_base as usize) as wasm32::uintptr_t
        };
        ptr.add(i).write(guest);
    }
}

pub unsafe fn decode_ciovec(
    vmctx: &mut VMContext,
    ciovec: &wasm32::__wasi_ciovec_t,
) -> Result<host::__wasi_ciovec_t, host::__wasi_errno_t> {
    let len = usize::try_from(ciovec.buf_len).unwrap();
    Ok(host::__wasi_ciovec_t {
        buf: decode_ptr(vmctx, ciovec.buf, len, 1)? as *const host::void,
        buf_len: len,
    })
}

pub unsafe fn decode_iovec(
    vmctx: &mut VMContext,
    iovec: &wasm32::__wasi_iovec_t,
) -> Result<host::__wasi_iovec_t, host::__wasi_errno_t> {
    let len = usize::try_from(iovec.buf_len).unwrap();
    Ok(host::__wasi_iovec_t {
        buf: decode_ptr(vmctx, iovec.buf, len, 1)? as *mut host::void,
        buf_len: len,
    })
}

pub unsafe fn decode_ciovec_slice(
    vmctx: &mut VMContext,
    ptr: wasm32::uintptr_t,
    len: wasm32::size_t,
) -> Result<Vec<host::__wasi_ciovec_t>, host::__wasi_errno_t> {
    let slice = decode_slice_of::<wasm32::__wasi_ciovec_t>(vmctx, ptr, len)?;
    let slice = slice::from_raw_parts(slice.0, slice.1);
    slice.iter().map(|iov| decode_ciovec(vmctx, iov)).collect()
}

pub unsafe fn decode_iovec_slice(
    vmctx: &mut VMContext,
    ptr: wasm32::uintptr_t,
    len: wasm32::size_t,
) -> Result<Vec<host::__wasi_iovec_t>, host::__wasi_errno_t> {
    let slice = decode_slice_of::<wasm32::__wasi_iovec_t>(vmctx, ptr, len)?;
    let slice = slice::from_raw_parts(slice.0, slice.1);
    slice.iter().map(|iov| decode_iovec(vmctx, iov)).collect()
}

pub fn decode_subscription(
    guest_subscription: wasm32::__wasi_subscription_t,
) -> Result<host::__wasi_subscription_t, host::__wasi_errno_t> {
    let mut host_subscription = host::__wasi_subscription_t {
        userdata: decode_userdata(guest_subscription.userdata),
        type_: decode_eventtype(guest_subscription.type_),
        u: unsafe { zeroed() },
    };

    match guest_subscription.type_ {
        wasm32::__WASI_EVENTTYPE_CLOCK => unsafe {
            host_subscription.u.clock =
                host::__wasi_subscription_t___wasi_subscription_u___wasi_subscription_u_clock_t {
                    identifier: decode_userdata(guest_subscription.u.clock.identifier),
                    clock_id: decode_clockid(guest_subscription.u.clock.clock_id),
                    timeout: decode_timestamp(guest_subscription.u.clock.timeout),
                    precision: decode_timestamp(guest_subscription.u.clock.precision),
                    flags: decode_subclockflags(guest_subscription.u.clock.flags),
                };
        },
        wasm32::__WASI_EVENTTYPE_FD_READ | wasm32::__WASI_EVENTTYPE_FD_WRITE => unsafe {
            host_subscription
            .u
            .fd_readwrite =
            host::__wasi_subscription_t___wasi_subscription_u___wasi_subscription_u_fd_readwrite_t {
                fd: decode_fd(guest_subscription.u.fd_readwrite.fd),
            }
        },
        _ => return Err(host::__WASI_EINVAL as host::__wasi_errno_t),
    };

    Ok(host_subscription)
}

pub unsafe fn decode_subscription_slice(
    vmctx: &mut VMContext,
    ptr: wasm32::uintptr_t,
    len: wasm32::size_t,
) -> Result<Vec<host::__wasi_subscription_t>, host::__wasi_errno_t> {
    let slice = decode_slice_of::<wasm32::__wasi_subscription_t>(vmctx, ptr, len)?;
    let slice = slice::from_raw_parts(slice.0, slice.1);
    slice
        .iter()
        .map(|subscription| decode_subscription(*subscription))
        .collect()
}

pub fn encode_event(host_event: host::__wasi_event_t) -> wasm32::__wasi_event_t {
    let mut guest_event = wasm32::__wasi_event_t {
        userdata: encode_userdata(host_event.userdata),
        error: encode_errno(host_event.error),
        type_: encode_eventtype(host_event.type_),
        u: unsafe { zeroed() },
        __bindgen_padding_0: 0,
    };

    match u32::from(host_event.type_) {
        host::__WASI_EVENTTYPE_CLOCK => {}
        host::__WASI_EVENTTYPE_FD_READ | host::__WASI_EVENTTYPE_FD_WRITE => unsafe {
            guest_event.u.fd_readwrite =
                wasm32::__wasi_event_t___wasi_event_u___wasi_event_u_fd_readwrite_t {
                    nbytes: encode_filesize(host_event.u.fd_readwrite.nbytes),
                    flags: encode_eventrwflags(host_event.u.fd_readwrite.flags),
                    __bindgen_padding_0: zeroed(),
                }
        },
        _ => panic!("unrecognized event type"),
    };

    guest_event
}

pub unsafe fn decode_event_slice(
    vmctx: &mut VMContext,
    ptr: wasm32::uintptr_t,
    len: wasm32::size_t,
) -> Result<(*mut wasm32::__wasi_event_t, usize), host::__wasi_errno_t> {
    decode_slice_of::<wasm32::__wasi_event_t>(vmctx, ptr, len)
}

pub unsafe fn encode_event_slice(
    ptr: *mut wasm32::__wasi_event_t,
    host_vec: Vec<host::__wasi_event_t>,
) {
    for (i, host) in host_vec.iter().enumerate() {
        let guest = encode_event(*host);

        ptr.add(i * size_of::<wasm32::__wasi_event_t>())
            .write(guest);
    }
}

pub unsafe fn decode_fd_byref(
    vmctx: &mut VMContext,
    fd_ptr: wasm32::uintptr_t,
) -> Result<(), host::__wasi_errno_t> {
    decode_pointee::<wasm32::__wasi_fd_t>(vmctx, fd_ptr)
}

pub unsafe fn encode_fd_byref(
    vmctx: &mut VMContext,
    fd_ptr: wasm32::uintptr_t,
    fd: host::__wasi_fd_t,
) {
    encode_pointee::<wasm32::__wasi_fd_t>(vmctx, fd_ptr, wasm32::size_t::try_from(fd).unwrap())
}

pub unsafe fn decode_timestamp_byref(
    vmctx: &mut VMContext,
    timestamp_ptr: wasm32::uintptr_t,
) -> Result<(), host::__wasi_errno_t> {
    decode_pointee::<wasm32::__wasi_timestamp_t>(vmctx, timestamp_ptr)
}

pub unsafe fn encode_timestamp_byref(
    vmctx: &mut VMContext,
    timestamp_ptr: wasm32::uintptr_t,
    host_timestamp: host::__wasi_timestamp_t,
) {
    encode_pointee::<wasm32::__wasi_timestamp_t>(
        vmctx,
        timestamp_ptr,
        wasm32::__wasi_timestamp_t::try_from(host_timestamp).unwrap(),
    )
}

pub unsafe fn decode_filesize_byref(
    vmctx: &mut VMContext,
    filesize_ptr: wasm32::uintptr_t,
) -> Result<(), host::__wasi_errno_t> {
    decode_pointee::<wasm32::__wasi_filesize_t>(vmctx, filesize_ptr)
}

pub unsafe fn encode_filesize_byref(
    vmctx: &mut VMContext,
    filesize_ptr: wasm32::uintptr_t,
    host_filesize: host::__wasi_filesize_t,
) {
    encode_pointee::<wasm32::__wasi_filesize_t>(
        vmctx,
        filesize_ptr,
        wasm32::__wasi_filesize_t::try_from(host_filesize).unwrap(),
    )
}

pub unsafe fn decode_roflags_byref(
    vmctx: &mut VMContext,
    roflags_ptr: wasm32::uintptr_t,
) -> Result<(), host::__wasi_errno_t> {
    decode_pointee::<wasm32::__wasi_roflags_t>(vmctx, roflags_ptr)
}

pub unsafe fn encode_roflags_byref(
    vmctx: &mut VMContext,
    roflags_ptr: wasm32::uintptr_t,
    host_roflags: host::__wasi_roflags_t,
) {
    encode_pointee::<wasm32::__wasi_roflags_t>(
        vmctx,
        roflags_ptr,
        wasm32::__wasi_roflags_t::try_from(host_roflags).unwrap(),
    )
}

pub unsafe fn decode_usize_byref(
    vmctx: &mut VMContext,
    usize_ptr: wasm32::uintptr_t,
) -> Result<(), host::__wasi_errno_t> {
    decode_pointee::<wasm32::size_t>(vmctx, usize_ptr)
}

pub unsafe fn encode_usize_byref(
    vmctx: &mut VMContext,
    usize_ptr: wasm32::uintptr_t,
    host_usize: usize,
) {
    encode_pointee::<wasm32::size_t>(
        vmctx,
        usize_ptr,
        wasm32::size_t::try_from(host_usize).unwrap(),
    )
}

pub unsafe fn decode_prestat_byref(
    vmctx: &mut VMContext,
    prestat_ptr: wasm32::uintptr_t,
) -> Result<(), host::__wasi_errno_t> {
    decode_pointee::<wasm32::__wasi_prestat_t>(vmctx, prestat_ptr)?;
    Ok(())
}

pub unsafe fn encode_prestat_byref(
    vmctx: &mut VMContext,
    prestat_ptr: wasm32::uintptr_t,
    host_prestat: host::__wasi_prestat_t,
) {
    let wasm32_prestat = wasm32::__wasi_prestat_t {
        pr_type: encode_preopentype(host_prestat.pr_type),
        u: wasm32::__wasi_prestat_t___wasi_prestat_u {
            dir: wasm32::__wasi_prestat_t___wasi_prestat_u___wasi_prestat_u_dir_t {
                pr_name_len: encode_usize(host_prestat.u.dir.pr_name_len),
            },
        },
    };

    encode_pointee::<wasm32::__wasi_prestat_t>(vmctx, prestat_ptr, wasm32_prestat)
}

pub unsafe fn decode_fdstat_byref(
    vmctx: &mut VMContext,
    fdstat_ptr: wasm32::uintptr_t,
) -> Result<(), host::__wasi_errno_t> {
    decode_pointee::<wasm32::__wasi_fdstat_t>(vmctx, fdstat_ptr)?;
    Ok(())
}

pub unsafe fn encode_fdstat_byref(
    vmctx: &mut VMContext,
    fdstat_ptr: wasm32::uintptr_t,
    host_fdstat: host::__wasi_fdstat_t,
) {
    let wasm32_fdstat = wasm32::__wasi_fdstat_t {
        fs_filetype: encode_filetype(host_fdstat.fs_filetype),
        fs_flags: encode_fdflags(host_fdstat.fs_flags),
        __bindgen_padding_0: 0,
        fs_rights_base: encode_rights(host_fdstat.fs_rights_base),
        fs_rights_inheriting: encode_rights(host_fdstat.fs_rights_inheriting),
    };

    encode_pointee::<wasm32::__wasi_fdstat_t>(vmctx, fdstat_ptr, wasm32_fdstat)
}

pub unsafe fn decode_filestat_byref(
    vmctx: &mut VMContext,
    filestat_ptr: wasm32::uintptr_t,
) -> Result<(), host::__wasi_errno_t> {
    decode_pointee::<wasm32::__wasi_filestat_t>(vmctx, filestat_ptr)?;
    Ok(())
}

pub unsafe fn encode_filestat_byref(
    vmctx: &mut VMContext,
    filestat_ptr: wasm32::uintptr_t,
    host_filestat: host::__wasi_filestat_t,
) {
    let wasm32_filestat = wasm32::__wasi_filestat_t {
        st_dev: encode_device(host_filestat.st_dev),
        st_ino: encode_inode(host_filestat.st_ino),
        st_filetype: encode_filetype(host_filestat.st_filetype),
        st_nlink: encode_linkcount(host_filestat.st_nlink),
        st_size: encode_filesize(host_filestat.st_size),
        st_atim: encode_timestamp(host_filestat.st_atim),
        st_mtim: encode_timestamp(host_filestat.st_mtim),
        st_ctim: encode_timestamp(host_filestat.st_ctim),
    };

    encode_pointee::<wasm32::__wasi_filestat_t>(vmctx, filestat_ptr, wasm32_filestat)
}

pub fn encode_errno(e: host::__wasi_errno_t) -> wasm32::__wasi_errno_t {
    assert_le!(e, wasm32::__WASI_ENOTCAPABLE);
    e
}
