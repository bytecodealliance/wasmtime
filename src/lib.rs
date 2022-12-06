#![allow(unused_variables)] // TODO: remove this when more things are implemented

use crate::bindings::{
    wasi_clocks, wasi_default_clocks, wasi_filesystem, wasi_logging, wasi_poll, wasi_random,
    wasi_tcp,
};
use core::arch::wasm32::unreachable;
use core::cell::{Cell, RefCell, UnsafeCell};
use core::ffi::c_void;
use core::mem::{self, forget, size_of, MaybeUninit};
use core::ptr::{self, copy_nonoverlapping, null_mut};
use core::slice;
use wasi::*;
use wasi_poll::WasiFuture;

mod bindings {
    wit_bindgen_guest_rust::generate!({
        path: "wit/wasi.wit",
        no_std,
        raw_strings,
        unchecked,
        // The generated definition of command will pull in std, so we are defining it
        // manually below instead
        skip: ["command"],
    });
}

#[export_name = "command"]
unsafe extern "C" fn command_entrypoint(
    stdin: u32,
    stdout: u32,
    args_ptr: *const WasmStr,
    args_len: usize,
) {
    State::with_mut(|state| {
        state.push_desc(Descriptor::File(File {
            fd: stdin,
            position: Cell::new(0),
        }))?;
        state.push_desc(Descriptor::File(File {
            fd: stdout,
            position: Cell::new(0),
        }))?;
        state.push_desc(Descriptor::Log)?;
        state.args = Some(slice::from_raw_parts(args_ptr, args_len));
        Ok(())
    });

    #[link(wasm_import_module = "__main_module__")]
    extern "C" {
        fn _start();
    }
    _start();
}

// We're avoiding static initializers, so replace the standard assert macros
// with simpler implementations.
macro_rules! assert {
    ($cond:expr $(,)?) => {
        if !$cond {
            unreachable()
        }
    };
}
macro_rules! assert_eq {
    ($left:expr, $right:expr $(,)?) => {
        assert!($left == $right);
    };
}

fn unwrap<T>(maybe: Option<T>) -> T {
    if let Some(value) = maybe {
        value
    } else {
        unreachable()
    }
}

#[no_mangle]
pub unsafe extern "C" fn cabi_import_realloc(
    old_ptr: *mut u8,
    old_size: usize,
    _align: usize,
    new_size: usize,
) -> *mut u8 {
    if !old_ptr.is_null() || old_size != 0 {
        unreachable();
    }
    let mut ptr = null_mut::<u8>();
    State::with(|state| {
        ptr = state.buffer_ptr.replace(null_mut());
        if ptr.is_null() {
            unreachable();
        }
        let len = state.buffer_len.replace(0);
        if len < new_size {
            unreachable();
        }
        Ok(())
    });
    ptr
}

/// This allocator is only used for the `command` entrypoint.
///
/// The implementation here is a bump allocator into `State::command_data` which
/// traps when it runs out of data. This means that the total size of
/// arguments/env/etc coming into a component is bounded by the current 64k
/// (ish) limit. That's just an implementation limit though which can be lifted
/// by dynamically calling `memory.grow` as necessary for more data.
#[no_mangle]
pub unsafe extern "C" fn cabi_export_realloc(
    old_ptr: *mut u8,
    old_size: usize,
    align: usize,
    new_size: usize,
) -> *mut u8 {
    if !old_ptr.is_null() || old_size != 0 {
        unreachable();
    }
    let mut ret = null_mut::<u8>();
    State::with_mut(|state| {
        let data = state.command_data.as_mut_ptr();
        let ptr = usize::try_from(state.command_data_next).unwrap();

        // "oom" as too much argument data tried to flow into the component.
        // Ideally this would have a better error message?
        if ptr + new_size > (*data).len() {
            unreachable();
        }
        state.command_data_next += new_size as u16;
        ret = (*data).as_mut_ptr().add(ptr);
        Ok(())
    });
    ret
}

/// Read command-line argument data.
/// The size of the array should match that returned by `args_sizes_get`
#[no_mangle]
pub unsafe extern "C" fn args_get(mut argv: *mut *mut u8, mut argv_buf: *mut u8) -> Errno {
    State::with(|state| {
        if let Some(args) = state.args {
            for arg in args {
                // Copy the argument into `argv_buf` which must be sized
                // appropriately by the caller.
                ptr::copy_nonoverlapping(arg.ptr, argv_buf, arg.len);
                *argv_buf.add(arg.len) = 0;

                // Copy the argument pointer into the `argv` buf
                *argv = argv_buf;

                // Update our pointers past what's written to prepare for the
                // next argument.
                argv = argv.add(1);
                argv_buf = argv_buf.add(arg.len + 1);
            }
        }
        Ok(())
    })
}

/// Return command-line argument data sizes.
#[no_mangle]
pub unsafe extern "C" fn args_sizes_get(argc: *mut Size, argv_buf_size: *mut Size) -> Errno {
    State::with(|state| {
        match state.args {
            Some(args) => {
                *argc = args.len();
                // Add one to each length for the terminating nul byte added by
                // the `args_get` function.
                *argv_buf_size = args.iter().map(|s| s.len + 1).sum();
            }
            None => {
                *argc = 0;
                *argv_buf_size = 0;
            }
        }
        Ok(())
    })
}

/// Read environment variable data.
/// The sizes of the buffers should match that returned by `environ_sizes_get`.
#[no_mangle]
pub unsafe extern "C" fn environ_get(environ: *mut *mut u8, environ_buf: *mut u8) -> Errno {
    // TODO: Use real env vars.
    *environ = null_mut();
    let _ = environ_buf;
    ERRNO_SUCCESS
}

/// Return environment variable data sizes.
#[no_mangle]
pub unsafe extern "C" fn environ_sizes_get(
    environc: *mut Size,
    environ_buf_size: *mut Size,
) -> Errno {
    // TODO: Use real env vars.
    *environc = 0;
    *environ_buf_size = 0;
    ERRNO_SUCCESS
}

/// Return the resolution of a clock.
/// Implementations are required to provide a non-zero value for supported clocks. For unsupported clocks,
/// return `errno::inval`.
/// Note: This is similar to `clock_getres` in POSIX.
#[no_mangle]
pub extern "C" fn clock_res_get(id: Clockid, resolution: &mut Timestamp) -> Errno {
    match id {
        CLOCKID_MONOTONIC => {
            let res = wasi_clocks::monotonic_clock_resolution(
                wasi_default_clocks::default_monotonic_clock(),
            );
            *resolution = res;
        }
        CLOCKID_REALTIME => {
            let res = wasi_clocks::wall_clock_resolution(wasi_default_clocks::default_wall_clock());
            *resolution = u64::from(res.nanoseconds) + res.seconds * 1_000_000_000;
        }
        _ => unreachable(),
    }
    ERRNO_SUCCESS
}

/// Return the time value of a clock.
/// Note: This is similar to `clock_gettime` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn clock_time_get(
    id: Clockid,
    _precision: Timestamp,
    time: &mut Timestamp,
) -> Errno {
    match id {
        CLOCKID_MONOTONIC => {
            *time =
                wasi_clocks::monotonic_clock_now(wasi_default_clocks::default_monotonic_clock());
        }
        CLOCKID_REALTIME => {
            let res = wasi_clocks::wall_clock_now(wasi_default_clocks::default_wall_clock());
            *time = u64::from(res.nanoseconds) + res.seconds * 1_000_000_000;
        }
        _ => unreachable(),
    }
    ERRNO_SUCCESS
}

/// Provide file advisory information on a file descriptor.
/// Note: This is similar to `posix_fadvise` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn fd_advise(
    fd: Fd,
    offset: Filesize,
    len: Filesize,
    advice: Advice,
) -> Errno {
    let advice = match advice {
        ADVICE_NORMAL => wasi_filesystem::Advice::Normal,
        ADVICE_SEQUENTIAL => wasi_filesystem::Advice::Sequential,
        ADVICE_RANDOM => wasi_filesystem::Advice::Random,
        ADVICE_WILLNEED => wasi_filesystem::Advice::WillNeed,
        ADVICE_DONTNEED => wasi_filesystem::Advice::DontNeed,
        ADVICE_NOREUSE => wasi_filesystem::Advice::NoReuse,
        _ => return ERRNO_INVAL,
    };
    State::with(|state| {
        let file = state.get_seekable_file(fd)?;
        wasi_filesystem::fadvise(file.fd, offset, len, advice)?;
        Ok(())
    })
}

/// Force the allocation of space in a file.
/// Note: This is similar to `posix_fallocate` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn fd_allocate(fd: Fd, offset: Filesize, len: Filesize) -> Errno {
    unreachable()
}

/// Close a file descriptor.
/// Note: This is similar to `close` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn fd_close(fd: Fd) -> Errno {
    unreachable()
}

/// Synchronize the data of a file to disk.
/// Note: This is similar to `fdatasync` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn fd_datasync(fd: Fd) -> Errno {
    State::with(|state| {
        let file = state.get_file(fd)?;
        wasi_filesystem::datasync(file.fd)?;
        Ok(())
    })
}

/// Get the attributes of a file descriptor.
/// Note: This returns similar flags to `fsync(fd, F_GETFL)` in POSIX, as well as additional fields.
#[no_mangle]
pub unsafe extern "C" fn fd_fdstat_get(fd: Fd, stat: *mut Fdstat) -> Errno {
    State::with(|state| match state.get(fd)? {
        Descriptor::File(file) => {
            let flags = wasi_filesystem::flags(file.fd)?;
            let type_ = wasi_filesystem::todo_type(file.fd)?;

            let fs_filetype = match type_ {
                wasi_filesystem::DescriptorType::RegularFile => FILETYPE_REGULAR_FILE,
                wasi_filesystem::DescriptorType::Directory => FILETYPE_DIRECTORY,
                wasi_filesystem::DescriptorType::BlockDevice => FILETYPE_BLOCK_DEVICE,
                wasi_filesystem::DescriptorType::CharacterDevice => FILETYPE_CHARACTER_DEVICE,
                wasi_filesystem::DescriptorType::Fifo => FILETYPE_UNKNOWN,
                wasi_filesystem::DescriptorType::Socket => FILETYPE_SOCKET_STREAM,
                wasi_filesystem::DescriptorType::SymbolicLink => FILETYPE_SYMBOLIC_LINK,
                wasi_filesystem::DescriptorType::Unknown => FILETYPE_UNKNOWN,
            };

            let mut fs_flags = 0;
            let mut fs_rights_base = !0;
            if !flags.contains(wasi_filesystem::DescriptorFlags::READ) {
                fs_rights_base &= !RIGHTS_FD_READ;
            }
            if !flags.contains(wasi_filesystem::DescriptorFlags::WRITE) {
                fs_rights_base &= !RIGHTS_FD_WRITE;
            }
            if flags.contains(wasi_filesystem::DescriptorFlags::APPEND) {
                fs_flags |= FDFLAGS_APPEND;
            }
            if flags.contains(wasi_filesystem::DescriptorFlags::DSYNC) {
                fs_flags |= FDFLAGS_DSYNC;
            }
            if flags.contains(wasi_filesystem::DescriptorFlags::NONBLOCK) {
                fs_flags |= FDFLAGS_NONBLOCK;
            }
            if flags.contains(wasi_filesystem::DescriptorFlags::RSYNC) {
                fs_flags |= FDFLAGS_RSYNC;
            }
            if flags.contains(wasi_filesystem::DescriptorFlags::SYNC) {
                fs_flags |= FDFLAGS_SYNC;
            }
            let fs_rights_inheriting = fs_rights_base;

            stat.write(Fdstat {
                fs_filetype,
                fs_flags,
                fs_rights_base,
                fs_rights_inheriting,
            });
            Ok(())
        }
        Descriptor::Log => {
            let fs_filetype = FILETYPE_UNKNOWN;
            let fs_flags = 0;
            let fs_rights_base = !RIGHTS_FD_READ;
            let fs_rights_inheriting = fs_rights_base;
            stat.write(Fdstat {
                fs_filetype,
                fs_flags,
                fs_rights_base,
                fs_rights_inheriting,
            });
            Ok(())
        }
        // TODO: Handle socket case here once `wasi-tcp` has been fleshed out
        Descriptor::Socket(_) => unreachable(),
        Descriptor::Closed => Err(ERRNO_BADF),
    })
}

/// Adjust the flags associated with a file descriptor.
/// Note: This is similar to `fcntl(fd, F_SETFL, flags)` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn fd_fdstat_set_flags(fd: Fd, flags: Fdflags) -> Errno {
    let mut new_flags = wasi_filesystem::DescriptorFlags::empty();
    if flags & FDFLAGS_APPEND == FDFLAGS_APPEND {
        new_flags |= wasi_filesystem::DescriptorFlags::APPEND;
    }
    if flags & FDFLAGS_DSYNC == FDFLAGS_DSYNC {
        new_flags |= wasi_filesystem::DescriptorFlags::DSYNC;
    }
    if flags & FDFLAGS_NONBLOCK == FDFLAGS_NONBLOCK {
        new_flags |= wasi_filesystem::DescriptorFlags::NONBLOCK;
    }
    if flags & FDFLAGS_RSYNC == FDFLAGS_RSYNC {
        new_flags |= wasi_filesystem::DescriptorFlags::RSYNC;
    }
    if flags & FDFLAGS_SYNC == FDFLAGS_SYNC {
        new_flags |= wasi_filesystem::DescriptorFlags::SYNC;
    }

    State::with(|state| {
        let file = state.get_file(fd)?;
        wasi_filesystem::set_flags(file.fd, new_flags)?;
        Ok(())
    })
}

/// Adjust the rights associated with a file descriptor.
/// This can only be used to remove rights, and returns `errno::notcapable` if called in a way that would attempt to add rights
#[no_mangle]
pub unsafe extern "C" fn fd_fdstat_set_rights(
    fd: Fd,
    fs_rights_base: Rights,
    fs_rights_inheriting: Rights,
) -> Errno {
    unreachable()
}

/// Return the attributes of an open file.
#[no_mangle]
pub unsafe extern "C" fn fd_filestat_get(fd: Fd, buf: *mut Filestat) -> Errno {
    State::with(|state| {
        let file = state.get_file(fd)?;
        let stat = wasi_filesystem::stat(file.fd)?;
        let filetype = match stat.type_ {
            wasi_filesystem::DescriptorType::Unknown => FILETYPE_UNKNOWN,
            wasi_filesystem::DescriptorType::Directory => FILETYPE_DIRECTORY,
            wasi_filesystem::DescriptorType::BlockDevice => FILETYPE_BLOCK_DEVICE,
            wasi_filesystem::DescriptorType::RegularFile => FILETYPE_REGULAR_FILE,
            // TODO: Add a way to disginguish between FILETYPE_SOCKET_STREAM and
            // FILETYPE_SOCKET_DGRAM.
            wasi_filesystem::DescriptorType::Socket => unreachable(),
            wasi_filesystem::DescriptorType::SymbolicLink => FILETYPE_SYMBOLIC_LINK,
            wasi_filesystem::DescriptorType::CharacterDevice => FILETYPE_CHARACTER_DEVICE,
            // preview1 never had a FIFO code.
            wasi_filesystem::DescriptorType::Fifo => FILETYPE_UNKNOWN,
        };
        *buf = Filestat {
            dev: stat.dev,
            ino: stat.ino,
            filetype,
            nlink: stat.nlink,
            size: stat.size,
            atim: stat.atim,
            mtim: stat.mtim,
            ctim: stat.ctim,
        };
        Ok(())
    })
}

/// Adjust the size of an open file. If this increases the file's size, the extra bytes are filled with zeros.
/// Note: This is similar to `ftruncate` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn fd_filestat_set_size(fd: Fd, size: Filesize) -> Errno {
    State::with(|state| {
        let file = state.get_file(fd)?;
        wasi_filesystem::set_size(file.fd, size)?;
        Ok(())
    })
}

/// Adjust the timestamps of an open file or directory.
/// Note: This is similar to `futimens` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn fd_filestat_set_times(
    fd: Fd,
    atim: Timestamp,
    mtim: Timestamp,
    fst_flags: Fstflags,
) -> Errno {
    let atim =
        if fst_flags & (FSTFLAGS_ATIM | FSTFLAGS_ATIM_NOW) == (FSTFLAGS_ATIM | FSTFLAGS_ATIM_NOW) {
            wasi_filesystem::NewTimestamp::Now
        } else if fst_flags & FSTFLAGS_ATIM == FSTFLAGS_ATIM {
            wasi_filesystem::NewTimestamp::Timestamp(atim)
        } else {
            wasi_filesystem::NewTimestamp::NoChange
        };
    let mtim =
        if fst_flags & (FSTFLAGS_MTIM | FSTFLAGS_MTIM_NOW) == (FSTFLAGS_MTIM | FSTFLAGS_MTIM_NOW) {
            wasi_filesystem::NewTimestamp::Now
        } else if fst_flags & FSTFLAGS_MTIM == FSTFLAGS_MTIM {
            wasi_filesystem::NewTimestamp::Timestamp(mtim)
        } else {
            wasi_filesystem::NewTimestamp::NoChange
        };

    State::with(|state| {
        let file = state.get_file(fd)?;
        wasi_filesystem::set_times(file.fd, atim, mtim)?;
        Ok(())
    })
}

/// Read from a file descriptor, without using and updating the file descriptor's offset.
/// Note: This is similar to `preadv` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn fd_pread(
    fd: Fd,
    mut iovs_ptr: *const Iovec,
    mut iovs_len: usize,
    offset: Filesize,
    nread: *mut Size,
) -> Errno {
    // Advance to the first non-empty buffer.
    while iovs_len != 0 && (*iovs_ptr).buf_len == 0 {
        iovs_ptr = iovs_ptr.add(1);
        iovs_len -= 1;
    }
    if iovs_len == 0 {
        *nread = 0;
        return ERRNO_SUCCESS;
    }

    State::with(|state| {
        let ptr = (*iovs_ptr).buf;
        let len = (*iovs_ptr).buf_len;
        state.register_buffer(ptr, len);

        let read_len = u32::try_from(len).unwrap();
        let file = state.get_file(fd)?;
        let data = wasi_filesystem::pread(file.fd, read_len, offset)?;
        assert_eq!(data.as_ptr(), ptr);
        assert!(data.len() <= len);
        *nread = data.len();
        forget(data);
        Ok(())
    })
}

/// Return a description of the given preopened file descriptor.
#[no_mangle]
pub unsafe extern "C" fn fd_prestat_get(fd: Fd, buf: *mut Prestat) -> Errno {
    unreachable()
}

/// Return a description of the given preopened file descriptor.
#[no_mangle]
pub unsafe extern "C" fn fd_prestat_dir_name(fd: Fd, path: *mut u8, path_len: Size) -> Errno {
    unreachable()
}

/// Write to a file descriptor, without using and updating the file descriptor's offset.
/// Note: This is similar to `pwritev` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn fd_pwrite(
    fd: Fd,
    mut iovs_ptr: *const Ciovec,
    mut iovs_len: usize,
    offset: Filesize,
    nwritten: *mut Size,
) -> Errno {
    // Advance to the first non-empty buffer.
    while iovs_len != 0 && (*iovs_ptr).buf_len == 0 {
        iovs_ptr = iovs_ptr.add(1);
        iovs_len -= 1;
    }
    if iovs_len == 0 {
        *nwritten = 0;
        return ERRNO_SUCCESS;
    }

    let ptr = (*iovs_ptr).buf;
    let len = (*iovs_ptr).buf_len;

    State::with(|state| match state.get(fd)? {
        Descriptor::File(file) => {
            let bytes = wasi_filesystem::pwrite(file.fd, slice::from_raw_parts(ptr, len), offset)?;

            *nwritten = bytes as usize;
            Ok(())
        }
        Descriptor::Log => {
            let bytes = slice::from_raw_parts(ptr, len);
            let context: [u8; 3] = [b'I', b'/', b'O'];
            wasi_logging::log(wasi_logging::Level::Info, &context, bytes);
            *nwritten = len;
            Ok(())
        }
        // TODO: Handle socket case here once `wasi-tcp` has been fleshed out
        Descriptor::Socket(_) => unreachable(),
        Descriptor::Closed => Err(ERRNO_BADF),
    })
}

/// Read from a file descriptor.
/// Note: This is similar to `readv` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn fd_read(
    fd: Fd,
    mut iovs_ptr: *const Iovec,
    mut iovs_len: usize,
    nread: *mut Size,
) -> Errno {
    // Advance to the first non-empty buffer.
    while iovs_len != 0 && (*iovs_ptr).buf_len == 0 {
        iovs_ptr = iovs_ptr.add(1);
        iovs_len -= 1;
    }
    if iovs_len == 0 {
        *nread = 0;
        return ERRNO_SUCCESS;
    }

    State::with(|state| {
        let ptr = (*iovs_ptr).buf;
        let len = (*iovs_ptr).buf_len;

        state.register_buffer(ptr, len);

        let read_len = u32::try_from(len).unwrap();
        let file = state.get_file(fd)?;
        let data = wasi_filesystem::pread(file.fd, read_len, file.position.get())?;
        assert_eq!(data.as_ptr(), ptr);
        assert!(data.len() <= len);
        *nread = data.len();
        file.position.set(file.position.get() + data.len() as u64);
        forget(data);
        Ok(())
    })
}

/// Read directory entries from a directory.
/// When successful, the contents of the output buffer consist of a sequence of
/// directory entries. Each directory entry consists of a `dirent` object,
/// followed by `dirent::d_namlen` bytes holding the name of the directory
/// entry.
/// This function fills the output buffer as much as possible, potentially
/// truncating the last directory entry. This allows the caller to grow its
/// read buffer size in case it's too small to fit a single large directory
/// entry, or skip the oversized directory entry.
#[no_mangle]
pub unsafe extern "C" fn fd_readdir(
    fd: Fd,
    buf: *mut u8,
    buf_len: Size,
    cookie: Dircookie,
    bufused: *mut Size,
) -> Errno {
    unreachable()
}

/// Atomically replace a file descriptor by renumbering another file descriptor.
/// Due to the strong focus on thread safety, this environment does not provide
/// a mechanism to duplicate or renumber a file descriptor to an arbitrary
/// number, like `dup2()`. This would be prone to race conditions, as an actual
/// file descriptor with the same number could be allocated by a different
/// thread at the same time.
/// This function provides a way to atomically renumber file descriptors, which
/// would disappear if `dup2()` were to be removed entirely.
#[no_mangle]
pub unsafe extern "C" fn fd_renumber(fd: Fd, to: Fd) -> Errno {
    unreachable()
}

/// Move the offset of a file descriptor.
/// Note: This is similar to `lseek` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn fd_seek(
    fd: Fd,
    offset: Filedelta,
    whence: Whence,
    newoffset: *mut Filesize,
) -> Errno {
    State::with(|state| {
        let file = state.get_seekable_file(fd)?;
        // It's ok to cast these indices; the WASI API will fail if
        // the resulting values are out of range.
        let from = match whence {
            WHENCE_SET => wasi_filesystem::SeekFrom::Set(offset as _),
            WHENCE_CUR => wasi_filesystem::SeekFrom::Cur(offset),
            WHENCE_END => wasi_filesystem::SeekFrom::End(offset as _),
            _ => return Err(ERRNO_INVAL),
        };
        let result = wasi_filesystem::seek(file.fd, from)?;
        *newoffset = result;
        Ok(())
    })
}

/// Synchronize the data and metadata of a file to disk.
/// Note: This is similar to `fsync` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn fd_sync(fd: Fd) -> Errno {
    State::with(|state| {
        let file = state.get_file(fd)?;
        wasi_filesystem::sync(file.fd)?;
        Ok(())
    })
}

/// Return the current offset of a file descriptor.
/// Note: This is similar to `lseek(fd, 0, SEEK_CUR)` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn fd_tell(fd: Fd, offset: *mut Filesize) -> Errno {
    State::with(|state| {
        let file = state.get_seekable_file(fd)?;
        *offset = wasi_filesystem::tell(file.fd)?;
        Ok(())
    })
}

/// Write to a file descriptor.
/// Note: This is similar to `writev` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn fd_write(
    fd: Fd,
    mut iovs_ptr: *const Ciovec,
    mut iovs_len: usize,
    nwritten: *mut Size,
) -> Errno {
    // Advance to the first non-empty buffer.
    while iovs_len != 0 && (*iovs_ptr).buf_len == 0 {
        iovs_ptr = iovs_ptr.add(1);
        iovs_len -= 1;
    }
    if iovs_len == 0 {
        *nwritten = 0;
        return ERRNO_SUCCESS;
    }

    let ptr = (*iovs_ptr).buf;
    let len = (*iovs_ptr).buf_len;

    State::with(|state| match state.get(fd)? {
        Descriptor::File(file) => {
            let bytes = wasi_filesystem::pwrite(
                file.fd,
                slice::from_raw_parts(ptr, len),
                file.position.get(),
            )?;

            *nwritten = bytes as usize;
            file.position.set(file.position.get() + u64::from(bytes));
            Ok(())
        }
        Descriptor::Log => {
            let bytes = slice::from_raw_parts(ptr, len);
            let context: [u8; 3] = [b'I', b'/', b'O'];
            wasi_logging::log(wasi_logging::Level::Info, &context, bytes);
            *nwritten = len;
            Ok(())
        }
        // TODO: Handle socket case here once `wasi-tcp` has been fleshed out
        Descriptor::Socket(_) => unreachable(),
        Descriptor::Closed => Err(ERRNO_BADF),
    })
}

/// Create a directory.
/// Note: This is similar to `mkdirat` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn path_create_directory(
    fd: Fd,
    path_ptr: *const u8,
    path_len: usize,
) -> Errno {
    let path = slice::from_raw_parts(path_ptr, path_len);

    State::with(|state| {
        let file = state.get_dir(fd)?;
        wasi_filesystem::create_directory_at(file.fd, path)?;
        Ok(())
    })
}

/// Return the attributes of a file or directory.
/// Note: This is similar to `stat` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn path_filestat_get(
    fd: Fd,
    flags: Lookupflags,
    path_ptr: *const u8,
    path_len: usize,
    buf: *mut Filestat,
) -> Errno {
    let path = slice::from_raw_parts(path_ptr, path_len);
    let at_flags = at_flags_from_lookupflags(flags);

    State::with(|state| {
        let file = state.get_dir(fd)?;
        let stat = wasi_filesystem::stat_at(file.fd, at_flags, path)?;
        let filetype = match stat.type_ {
            wasi_filesystem::DescriptorType::Unknown => FILETYPE_UNKNOWN,
            wasi_filesystem::DescriptorType::Directory => FILETYPE_DIRECTORY,
            wasi_filesystem::DescriptorType::BlockDevice => FILETYPE_BLOCK_DEVICE,
            wasi_filesystem::DescriptorType::RegularFile => FILETYPE_REGULAR_FILE,
            // TODO: Add a way to disginguish between FILETYPE_SOCKET_STREAM and
            // FILETYPE_SOCKET_DGRAM.
            wasi_filesystem::DescriptorType::Socket => unreachable(),
            wasi_filesystem::DescriptorType::SymbolicLink => FILETYPE_SYMBOLIC_LINK,
            wasi_filesystem::DescriptorType::CharacterDevice => FILETYPE_CHARACTER_DEVICE,
            // preview1 never had a FIFO code.
            wasi_filesystem::DescriptorType::Fifo => FILETYPE_UNKNOWN,
        };
        *buf = Filestat {
            dev: stat.dev,
            ino: stat.ino,
            filetype,
            nlink: stat.nlink,
            size: stat.size,
            atim: stat.atim,
            mtim: stat.mtim,
            ctim: stat.ctim,
        };
        Ok(())
    })
}

/// Adjust the timestamps of a file or directory.
/// Note: This is similar to `utimensat` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn path_filestat_set_times(
    fd: Fd,
    flags: Lookupflags,
    path_ptr: *const u8,
    path_len: usize,
    atim: Timestamp,
    mtim: Timestamp,
    fst_flags: Fstflags,
) -> Errno {
    let atim =
        if fst_flags & (FSTFLAGS_ATIM | FSTFLAGS_ATIM_NOW) == (FSTFLAGS_ATIM | FSTFLAGS_ATIM_NOW) {
            wasi_filesystem::NewTimestamp::Now
        } else if fst_flags & FSTFLAGS_ATIM == FSTFLAGS_ATIM {
            wasi_filesystem::NewTimestamp::Timestamp(atim)
        } else {
            wasi_filesystem::NewTimestamp::NoChange
        };
    let mtim =
        if fst_flags & (FSTFLAGS_MTIM | FSTFLAGS_MTIM_NOW) == (FSTFLAGS_MTIM | FSTFLAGS_MTIM_NOW) {
            wasi_filesystem::NewTimestamp::Now
        } else if fst_flags & FSTFLAGS_MTIM == FSTFLAGS_MTIM {
            wasi_filesystem::NewTimestamp::Timestamp(mtim)
        } else {
            wasi_filesystem::NewTimestamp::NoChange
        };

    let path = slice::from_raw_parts(path_ptr, path_len);
    let at_flags = at_flags_from_lookupflags(flags);

    State::with(|state| {
        let file = state.get_dir(fd)?;
        wasi_filesystem::set_times_at(file.fd, at_flags, path, atim, mtim)?;
        Ok(())
    })
}

/// Create a hard link.
/// Note: This is similar to `linkat` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn path_link(
    old_fd: Fd,
    old_flags: Lookupflags,
    old_path_ptr: *const u8,
    old_path_len: usize,
    new_fd: Fd,
    new_path_ptr: *const u8,
    new_path_len: usize,
) -> Errno {
    let old_path = slice::from_raw_parts(old_path_ptr, old_path_len);
    let new_path = slice::from_raw_parts(new_path_ptr, new_path_len);
    let at_flags = at_flags_from_lookupflags(old_flags);

    State::with(|state| {
        let old = state.get_dir(old_fd)?.fd;
        let new = state.get_dir(new_fd)?.fd;
        wasi_filesystem::link_at(old, at_flags, old_path, new, new_path)?;
        Ok(())
    })
}

/// Open a file or directory.
/// The returned file descriptor is not guaranteed to be the lowest-numbered
/// file descriptor not currently open; it is randomized to prevent
/// applications from depending on making assumptions about indexes, since this
/// is error-prone in multi-threaded contexts. The returned file descriptor is
/// guaranteed to be less than 2**31.
/// Note: This is similar to `openat` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn path_open(
    fd: Fd,
    dirflags: Lookupflags,
    path_ptr: *const u8,
    path_len: usize,
    oflags: Oflags,
    fs_rights_base: Rights,
    fs_rights_inheriting: Rights,
    fdflags: Fdflags,
    opened_fd: *mut Fd,
) -> Errno {
    unreachable()
}

/// Read the contents of a symbolic link.
/// Note: This is similar to `readlinkat` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn path_readlink(
    fd: Fd,
    path_ptr: *const u8,
    path_len: usize,
    buf: *mut u8,
    buf_len: Size,
    bufused: *mut Size,
) -> Errno {
    let path = slice::from_raw_parts(path_ptr, path_len);

    State::with(|state| {
        // If the user gave us a buffer shorter than `PATH_MAX`, it may not be
        // long enough to accept the actual path. `cabi_realloc` can't fail,
        // so instead we handle this case specially.
        let use_state_buf = buf_len < PATH_MAX;

        if use_state_buf {
            state.register_buffer(state.path_buf.get().cast(), PATH_MAX);
        } else {
            state.register_buffer(buf, buf_len);
        }

        let file = state.get_dir(fd)?;
        let path = wasi_filesystem::readlink_at(file.fd, path)?;

        assert_eq!(path.as_ptr(), buf);
        assert!(path.len() <= buf_len);

        *bufused = path.len();
        if use_state_buf {
            // Preview1 follows POSIX in truncating the returned path if it
            // doesn't fit.
            let len = core::cmp::min(path.len(), buf_len);
            copy_nonoverlapping(path.as_ptr().cast(), buf, len);
        }

        // The returned string's memory was allocated in `buf`, so don't separately
        // free it.
        forget(path);

        Ok(())
    })
}

/// Remove a directory.
/// Return `errno::notempty` if the directory is not empty.
/// Note: This is similar to `unlinkat(fd, path, AT_REMOVEDIR)` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn path_remove_directory(
    fd: Fd,
    path_ptr: *const u8,
    path_len: usize,
) -> Errno {
    let path = slice::from_raw_parts(path_ptr, path_len);

    State::with(|state| {
        let file = state.get_dir(fd)?;
        wasi_filesystem::remove_directory_at(file.fd, path)?;
        Ok(())
    })
}

/// Rename a file or directory.
/// Note: This is similar to `renameat` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn path_rename(
    old_fd: Fd,
    old_path_ptr: *const u8,
    old_path_len: usize,
    new_fd: Fd,
    new_path_ptr: *const u8,
    new_path_len: usize,
) -> Errno {
    let old_path = slice::from_raw_parts(old_path_ptr, old_path_len);
    let new_path = slice::from_raw_parts(new_path_ptr, new_path_len);

    State::with(|state| {
        let old = state.get_dir(old_fd)?.fd;
        let new = state.get_dir(new_fd)?.fd;
        wasi_filesystem::rename_at(old, old_path, new, new_path)?;
        Ok(())
    })
}

/// Create a symbolic link.
/// Note: This is similar to `symlinkat` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn path_symlink(
    old_path_ptr: *const u8,
    old_path_len: usize,
    fd: Fd,
    new_path_ptr: *const u8,
    new_path_len: usize,
) -> Errno {
    let old_path = slice::from_raw_parts(old_path_ptr, old_path_len);
    let new_path = slice::from_raw_parts(new_path_ptr, new_path_len);

    State::with(|state| {
        let file = state.get_dir(fd)?;
        wasi_filesystem::symlink_at(file.fd, old_path, new_path)?;
        Ok(())
    })
}

/// Unlink a file.
/// Return `errno::isdir` if the path refers to a directory.
/// Note: This is similar to `unlinkat(fd, path, 0)` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn path_unlink_file(fd: Fd, path_ptr: *const u8, path_len: usize) -> Errno {
    let path = slice::from_raw_parts(path_ptr, path_len);

    State::with(|state| {
        let file = state.get_dir(fd)?;
        wasi_filesystem::unlink_file_at(file.fd, path)?;
        Ok(())
    })
}

struct Futures {
    pointer: *mut WasiFuture,
    index: usize,
    length: usize,
}

impl Futures {
    unsafe fn push(&mut self, future: WasiFuture) {
        assert!(self.index < self.length);
        *self.pointer.add(self.index) = future;
        self.index += 1;
    }
}

impl Drop for Futures {
    fn drop(&mut self) {
        for i in 0..self.index {
            wasi_poll::drop_future(unsafe { *self.pointer.add(i) })
        }
    }
}

impl From<wasi_tcp::Error> for Errno {
    fn from(error: wasi_tcp::Error) -> Errno {
        use wasi_tcp::Error::*;

        match error {
            ConnectionAborted => black_box(ERRNO_CONNABORTED),
            ConnectionRefused => ERRNO_CONNREFUSED,
            ConnectionReset => ERRNO_CONNRESET,
            HostUnreachable => ERRNO_HOSTUNREACH,
            NetworkDown => ERRNO_NETDOWN,
            NetworkUnreachable => ERRNO_NETUNREACH,
            Timeout => ERRNO_TIMEDOUT,
        }
    }
}

/// Concurrently poll for the occurrence of a set of events.
#[no_mangle]
pub unsafe extern "C" fn poll_oneoff(
    r#in: *const Subscription,
    out: *mut Event,
    nsubscriptions: Size,
    nevents: *mut Size,
) -> Errno {
    *nevents = 0;

    let subscriptions = slice::from_raw_parts(r#in, nsubscriptions);

    // We're going to split the `nevents` buffer into two non-overlapping buffers: one to store the future handles,
    // and the other to store the bool results.
    //
    // First, we assert that this is possible:
    assert!(mem::align_of::<Event>() >= mem::align_of::<WasiFuture>());
    assert!(mem::align_of::<WasiFuture>() >= mem::align_of::<u8>());
    assert!(
        unwrap(nsubscriptions.checked_mul(mem::size_of::<Event>()))
            > unwrap(
                unwrap(nsubscriptions.checked_mul(mem::size_of::<WasiFuture>()))
                    .checked_add(unwrap(nsubscriptions.checked_mul(mem::size_of::<u8>())))
            )
    );

    let futures = out as *mut c_void as *mut WasiFuture;
    let results = futures.add(nsubscriptions) as *mut c_void as *mut u8;

    State::with(|state| {
        state.register_buffer(
            results,
            unwrap(nsubscriptions.checked_mul(mem::size_of::<bool>())),
        );

        let mut futures = Futures {
            pointer: futures,
            index: 0,
            length: nsubscriptions,
        };

        for subscription in subscriptions {
            futures.push(match subscription.u.tag {
                0 => {
                    let clock = &subscription.u.u.clock;
                    let absolute = (clock.flags & SUBCLOCKFLAGS_SUBSCRIPTION_CLOCK_ABSTIME) == 0;
                    match clock.id {
                        CLOCKID_REALTIME => wasi_clocks::subscribe_wall_clock(
                            wasi_clocks::Datetime {
                                seconds: unwrap((clock.timeout / 1_000_000_000).try_into().ok()),
                                nanoseconds: unwrap(
                                    (clock.timeout % 1_000_000_000).try_into().ok(),
                                ),
                            },
                            absolute,
                        ),

                        CLOCKID_MONOTONIC => {
                            wasi_clocks::subscribe_monotonic_clock(clock.timeout, absolute)
                        }

                        _ => return Err(ERRNO_INVAL),
                    }
                }

                1 => wasi_tcp::subscribe_read(
                    state.get_socket(subscription.u.u.fd_read.file_descriptor)?,
                ),

                2 => wasi_tcp::subscribe_write(
                    state.get_socket(subscription.u.u.fd_write.file_descriptor)?,
                ),

                _ => return Err(ERRNO_INVAL),
            });
        }

        let vec = wasi_poll::poll_oneoff(slice::from_raw_parts(futures.pointer, futures.length));

        assert!(vec.len() == nsubscriptions);
        assert_eq!(vec.as_ptr(), results);
        mem::forget(vec);

        drop(futures);

        let ready = subscriptions
            .iter()
            .enumerate()
            .filter_map(|(i, s)| (*results.add(i) != 0).then_some(s));

        let mut count = 0;

        for subscription in ready {
            let error;
            let type_;
            let nbytes;
            let flags;

            match subscription.u.tag {
                0 => {
                    error = ERRNO_SUCCESS;
                    type_ = EVENTTYPE_CLOCK;
                    nbytes = 0;
                    flags = 0;
                }

                1 => {
                    type_ = EVENTTYPE_FD_READ;
                    match wasi_tcp::bytes_readable(subscription.u.u.fd_read.file_descriptor) {
                        Ok(result) => {
                            error = ERRNO_SUCCESS;
                            nbytes = result.nbytes;
                            flags = if result.is_closed {
                                EVENTRWFLAGS_FD_READWRITE_HANGUP
                            } else {
                                0
                            };
                        }
                        Err(e) => {
                            error = e.into();
                            nbytes = 0;
                            flags = 0;
                        }
                    }
                }

                2 => {
                    type_ = EVENTTYPE_FD_WRITE;
                    match wasi_tcp::bytes_writable(subscription.u.u.fd_write.file_descriptor) {
                        Ok(result) => {
                            error = ERRNO_SUCCESS;
                            nbytes = result.nbytes;
                            flags = if result.is_closed {
                                EVENTRWFLAGS_FD_READWRITE_HANGUP
                            } else {
                                0
                            };
                        }
                        Err(e) => {
                            error = e.into();
                            nbytes = 0;
                            flags = 0;
                        }
                    }
                }

                _ => unreachable(),
            }

            *out.add(count) = Event {
                userdata: subscription.userdata,
                error,
                type_,
                fd_readwrite: EventFdReadwrite { nbytes, flags },
            };

            count += 1;
        }

        *nevents = count;

        Ok(())
    })
}

/// Terminate the process normally. An exit code of 0 indicates successful
/// termination of the program. The meanings of other values is dependent on
/// the environment.
#[no_mangle]
pub unsafe extern "C" fn proc_exit(rval: Exitcode) -> ! {
    unreachable()
}

/// Send a signal to the process of the calling thread.
/// Note: This is similar to `raise` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn proc_raise(sig: Signal) -> Errno {
    unreachable()
}

/// Temporarily yield execution of the calling thread.
/// Note: This is similar to `sched_yield` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn sched_yield() -> Errno {
    // TODO: This is not yet covered in Preview2.

    ERRNO_SUCCESS
}

/// Write high-quality random data into a buffer.
/// This function blocks when the implementation is unable to immediately
/// provide sufficient high-quality random data.
/// This function may execute slowly, so when large mounts of random data are
/// required, it's advisable to use this function to seed a pseudo-random
/// number generator, rather than to provide the random data directly.
#[no_mangle]
pub unsafe extern "C" fn random_get(buf: *mut u8, buf_len: Size) -> Errno {
    State::with(|state| {
        state.register_buffer(buf, buf_len);

        assert_eq!(buf_len as u32 as Size, buf_len);
        let result = wasi_random::getrandom(buf_len as u32);
        assert_eq!(result.as_ptr(), buf);

        // The returned buffer's memory was allocated in `buf`, so don't separately
        // free it.
        forget(result);

        Ok(())
    })
}

/// Receive a message from a socket.
/// Note: This is similar to `recv` in POSIX, though it also supports reading
/// the data into multiple buffers in the manner of `readv`.
#[no_mangle]
pub unsafe extern "C" fn sock_recv(
    fd: Fd,
    ri_data_ptr: *const Iovec,
    ri_data_len: usize,
    ri_flags: Riflags,
    ro_datalen: *mut Size,
    ro_flags: *mut Roflags,
) -> Errno {
    unreachable()
}

/// Send a message on a socket.
/// Note: This is similar to `send` in POSIX, though it also supports writing
/// the data from multiple buffers in the manner of `writev`.
#[no_mangle]
pub unsafe extern "C" fn sock_send(
    fd: Fd,
    si_data_ptr: *const Ciovec,
    si_data_len: usize,
    si_flags: Siflags,
    so_datalen: *mut Size,
) -> Errno {
    unreachable()
}

/// Shut down socket send and receive channels.
/// Note: This is similar to `shutdown` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn sock_shutdown(fd: Fd, how: Sdflags) -> Errno {
    unreachable()
}

fn at_flags_from_lookupflags(flags: Lookupflags) -> wasi_filesystem::AtFlags {
    if flags & LOOKUPFLAGS_SYMLINK_FOLLOW == LOOKUPFLAGS_SYMLINK_FOLLOW {
        wasi_filesystem::AtFlags::SYMLINK_FOLLOW
    } else {
        wasi_filesystem::AtFlags::empty()
    }
}

impl From<wasi_filesystem::Errno> for Errno {
    #[inline(never)] // Disable inlining as this is bulky and relatively cold.
    fn from(err: wasi_filesystem::Errno) -> Errno {
        match err {
            wasi_filesystem::Errno::Toobig => black_box(ERRNO_2BIG),
            wasi_filesystem::Errno::Access => ERRNO_ACCES,
            wasi_filesystem::Errno::Addrinuse => ERRNO_ADDRINUSE,
            wasi_filesystem::Errno::Addrnotavail => ERRNO_ADDRNOTAVAIL,
            wasi_filesystem::Errno::Afnosupport => ERRNO_AFNOSUPPORT,
            wasi_filesystem::Errno::Again => ERRNO_AGAIN,
            wasi_filesystem::Errno::Already => ERRNO_ALREADY,
            wasi_filesystem::Errno::Badmsg => ERRNO_BADMSG,
            wasi_filesystem::Errno::Busy => ERRNO_BUSY,
            wasi_filesystem::Errno::Canceled => ERRNO_CANCELED,
            wasi_filesystem::Errno::Child => ERRNO_CHILD,
            wasi_filesystem::Errno::Connaborted => ERRNO_CONNABORTED,
            wasi_filesystem::Errno::Connrefused => ERRNO_CONNREFUSED,
            wasi_filesystem::Errno::Connreset => ERRNO_CONNRESET,
            wasi_filesystem::Errno::Deadlk => ERRNO_DEADLK,
            wasi_filesystem::Errno::Destaddrreq => ERRNO_DESTADDRREQ,
            wasi_filesystem::Errno::Dquot => ERRNO_DQUOT,
            wasi_filesystem::Errno::Exist => ERRNO_EXIST,
            wasi_filesystem::Errno::Fault => ERRNO_FAULT,
            wasi_filesystem::Errno::Fbig => ERRNO_FBIG,
            wasi_filesystem::Errno::Hostunreach => ERRNO_HOSTUNREACH,
            wasi_filesystem::Errno::Idrm => ERRNO_IDRM,
            wasi_filesystem::Errno::Ilseq => ERRNO_ILSEQ,
            wasi_filesystem::Errno::Inprogress => ERRNO_INPROGRESS,
            wasi_filesystem::Errno::Intr => ERRNO_INTR,
            wasi_filesystem::Errno::Inval => ERRNO_INVAL,
            wasi_filesystem::Errno::Io => ERRNO_IO,
            wasi_filesystem::Errno::Isconn => ERRNO_ISCONN,
            wasi_filesystem::Errno::Isdir => ERRNO_ISDIR,
            wasi_filesystem::Errno::Loop => ERRNO_LOOP,
            wasi_filesystem::Errno::Mfile => ERRNO_MFILE,
            wasi_filesystem::Errno::Mlink => ERRNO_MLINK,
            wasi_filesystem::Errno::Msgsize => ERRNO_MSGSIZE,
            wasi_filesystem::Errno::Multihop => ERRNO_MULTIHOP,
            wasi_filesystem::Errno::Nametoolong => ERRNO_NAMETOOLONG,
            wasi_filesystem::Errno::Netdown => ERRNO_NETDOWN,
            wasi_filesystem::Errno::Netreset => ERRNO_NETRESET,
            wasi_filesystem::Errno::Netunreach => ERRNO_NETUNREACH,
            wasi_filesystem::Errno::Nfile => ERRNO_NFILE,
            wasi_filesystem::Errno::Nobufs => ERRNO_NOBUFS,
            wasi_filesystem::Errno::Nodev => ERRNO_NODEV,
            wasi_filesystem::Errno::Noent => ERRNO_NOENT,
            wasi_filesystem::Errno::Noexec => ERRNO_NOEXEC,
            wasi_filesystem::Errno::Nolck => ERRNO_NOLCK,
            wasi_filesystem::Errno::Nolink => ERRNO_NOLINK,
            wasi_filesystem::Errno::Nomem => ERRNO_NOMEM,
            wasi_filesystem::Errno::Nomsg => ERRNO_NOMSG,
            wasi_filesystem::Errno::Noprotoopt => ERRNO_NOPROTOOPT,
            wasi_filesystem::Errno::Nospc => ERRNO_NOSPC,
            wasi_filesystem::Errno::Nosys => ERRNO_NOSYS,
            wasi_filesystem::Errno::Notdir => ERRNO_NOTDIR,
            wasi_filesystem::Errno::Notempty => ERRNO_NOTEMPTY,
            wasi_filesystem::Errno::Notrecoverable => ERRNO_NOTRECOVERABLE,
            wasi_filesystem::Errno::Notsup => ERRNO_NOTSUP,
            wasi_filesystem::Errno::Notty => ERRNO_NOTTY,
            wasi_filesystem::Errno::Nxio => ERRNO_NXIO,
            wasi_filesystem::Errno::Overflow => ERRNO_OVERFLOW,
            wasi_filesystem::Errno::Ownerdead => ERRNO_OWNERDEAD,
            wasi_filesystem::Errno::Perm => ERRNO_PERM,
            wasi_filesystem::Errno::Pipe => ERRNO_PIPE,
            wasi_filesystem::Errno::Range => ERRNO_RANGE,
            wasi_filesystem::Errno::Rofs => ERRNO_ROFS,
            wasi_filesystem::Errno::Spipe => ERRNO_SPIPE,
            wasi_filesystem::Errno::Srch => ERRNO_SRCH,
            wasi_filesystem::Errno::Stale => ERRNO_STALE,
            wasi_filesystem::Errno::Timedout => ERRNO_TIMEDOUT,
            wasi_filesystem::Errno::Txtbsy => ERRNO_TXTBSY,
            wasi_filesystem::Errno::Xdev => ERRNO_XDEV,
        }
    }
}

// A black box to prevent the optimizer from generating a lookup table
// from the match above, which would require a static initializer.
fn black_box(x: Errno) -> Errno {
    core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
    x
}

#[repr(C)]
pub enum Descriptor {
    Closed,
    File(File),
    Socket(wasi_tcp::Socket),
    Log,
}

#[repr(C)]
pub struct File {
    fd: wasi_filesystem::Descriptor,
    position: Cell<u64>,
}

const PAGE_SIZE: usize = 65536;

/// The maximum path length. WASI doesn't explicitly guarantee this, but all
/// popular OS's have a `PATH_MAX` of at most 4096, so that's enough for this
/// polyfill.
const PATH_MAX: usize = 4096;

const MAX_DESCRIPTORS: usize = 128;

struct State {
    /// Used by `register_buffer` to coordinate allocations with
    /// `cabi_import_realloc`.
    buffer_ptr: Cell<*mut u8>,
    buffer_len: Cell<usize>,

    /// Storage of mapping from preview1 file descriptors to preview2 file
    /// descriptors.
    ndescriptors: u16,
    descriptors: MaybeUninit<[Descriptor; MAX_DESCRIPTORS]>,

    /// Auxiliary storage to handle the `path_readlink` function.
    path_buf: UnsafeCell<MaybeUninit<[u8; PATH_MAX]>>,

    /// Storage area for data passed to the `command` entrypoint. The
    /// `command_data` is a block of memory which is dynamically allocated from
    /// in `cabi_export_realloc`. The `command_data_next` is the
    /// bump-allocated-pointer of where to allocate from next.
    command_data: MaybeUninit<[u8; command_data_size()]>,
    command_data_next: u16,

    /// Arguments passed to the `command` entrypoint
    args: Option<&'static [WasmStr]>,
}

struct WasmStr {
    ptr: *const u8,
    len: usize,
}

const fn command_data_size() -> usize {
    // The total size of the struct should be a page, so start there
    let mut start = PAGE_SIZE;

    // Remove the big chunks of the struct, the `path_buf` and `descriptors`
    // fields.
    start -= PATH_MAX;
    start -= size_of::<Descriptor>() * MAX_DESCRIPTORS;

    // Remove miscellaneous metadata also stored in state.
    start -= 5 * size_of::<usize>();

    // Everything else is the `command_data` allocation.
    start
}

// Statically assert that the `State` structure is the size of a wasm page. This
// mostly guarantees that it's not larger than one page which is relied upon
// below.
const _: () = {
    let _size_assert: [(); PAGE_SIZE] = [(); size_of::<State>()];
};

#[allow(improper_ctypes)]
extern "C" {
    fn get_global_ptr() -> *const RefCell<State>;
    fn set_global_ptr(a: *const RefCell<State>);
}

impl State {
    fn with(f: impl FnOnce(&State) -> Result<(), Errno>) -> Errno {
        let ptr = State::ptr();
        let ptr = ptr.try_borrow().unwrap_or_else(|_| unreachable());
        let ret = f(&*ptr);
        match ret {
            Ok(()) => ERRNO_SUCCESS,
            Err(err) => err,
        }
    }

    fn with_mut(f: impl FnOnce(&mut State) -> Result<(), Errno>) -> Errno {
        let ptr = State::ptr();
        let mut ptr = ptr.try_borrow_mut().unwrap_or_else(|_| unreachable());
        let ret = f(&mut *ptr);
        match ret {
            Ok(()) => ERRNO_SUCCESS,
            Err(err) => err,
        }
    }

    fn ptr() -> &'static RefCell<State> {
        unsafe {
            let mut ptr = get_global_ptr();
            if ptr.is_null() {
                ptr = State::new();
                set_global_ptr(ptr);
            }
            &*ptr
        }
    }

    #[cold]
    fn new() -> &'static RefCell<State> {
        let grew = core::arch::wasm32::memory_grow(0, 1);
        if grew == usize::MAX {
            unreachable();
        }
        let ret = (grew * PAGE_SIZE) as *mut RefCell<State>;
        unsafe {
            ret.write(RefCell::new(State {
                buffer_ptr: Cell::new(null_mut()),
                buffer_len: Cell::new(0),
                ndescriptors: 0,
                descriptors: MaybeUninit::uninit(),
                path_buf: UnsafeCell::new(MaybeUninit::uninit()),
                command_data: MaybeUninit::uninit(),
                command_data_next: 0,
                args: None,
            }));
            &*ret
        }
    }

    fn push_desc(&mut self, desc: Descriptor) -> Result<Fd, Errno> {
        unsafe {
            let descriptors = self.descriptors.as_mut_ptr();
            let ndescriptors = usize::try_from(self.ndescriptors).unwrap();
            if ndescriptors >= (*descriptors).len() {
                return Err(ERRNO_INVAL);
            }
            ptr::addr_of_mut!((*descriptors)[ndescriptors]).write(desc);
            self.ndescriptors += 1;
            Ok(Fd::from(self.ndescriptors - 1))
        }
    }

    fn get(&self, fd: Fd) -> Result<&Descriptor, Errno> {
        let index = usize::try_from(fd).unwrap();
        if index < usize::try_from(self.ndescriptors).unwrap() {
            unsafe { (*self.descriptors.as_ptr()).get(index).ok_or(ERRNO_BADF) }
        } else {
            Err(ERRNO_BADF)
        }
    }

    fn get_file_with_error(&self, fd: Fd, error: Errno) -> Result<&File, Errno> {
        match self.get(fd)? {
            Descriptor::File(file) => Ok(file),
            Descriptor::Log | Descriptor::Socket(_) => Err(error),
            Descriptor::Closed => Err(ERRNO_BADF),
        }
    }

    fn get_socket(&self, fd: Fd) -> Result<wasi_tcp::Socket, Errno> {
        match self.get(fd)? {
            Descriptor::Socket(socket) => Ok(*socket),
            Descriptor::Log | Descriptor::File(_) => Err(ERRNO_INVAL),
            Descriptor::Closed => Err(ERRNO_BADF),
        }
    }

    fn get_file(&self, fd: Fd) -> Result<&File, Errno> {
        self.get_file_with_error(fd, ERRNO_INVAL)
    }

    fn get_dir(&self, fd: Fd) -> Result<&File, Errno> {
        self.get_file_with_error(fd, ERRNO_NOTDIR)
    }

    fn get_seekable_file(&self, fd: Fd) -> Result<&File, Errno> {
        self.get_file_with_error(fd, ERRNO_SPIPE)
    }

    /// Register `buf` and `buf_len` to be used by `cabi_realloc` to satisfy
    /// the next request.
    fn register_buffer(&self, buf: *mut u8, buf_len: usize) {
        self.buffer_ptr.set(buf);
        self.buffer_len.set(buf_len);
    }
}
