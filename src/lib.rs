#![allow(unused_variables)] // TODO: remove this when more things are implemented

wit_bindgen_guest_rust::generate!({
    import: "wit/wasi-clocks.wit.md",
    import: "wit/wasi-default-clocks.wit.md",
    import: "wit/wasi-filesystem.wit.md",
    import: "wit/wasi-logging.wit.md",
    import: "wit/wasi-poll.wit.md",
    import: "wit/wasi-random.wit.md",
    default: "wit/command.wit.md",
    name: "wasi_command",
    no_std,
    raw_strings,
    unchecked,
    // The generated definition of command will pull in std, so we are defining it
    // manually below instead
    skip: ["command"],
});

use core::arch::wasm32::unreachable;
use core::mem::{forget, size_of};
use core::ptr::{copy_nonoverlapping, null_mut};
use core::slice;
use wasi::*;

#[export_name = "command"]
unsafe extern "C" fn command_entrypoint(stdin: i32, stdout: i32, _args_ptr: i32, _args_len: i32) {
    *Descriptor::get(0) = Descriptor::File(File {
        fd: stdin as u32,
        position: 0,
    });
    *Descriptor::get(1) = Descriptor::File(File {
        fd: stdout as u32,
        position: 0,
    });
    *Descriptor::get(2) = Descriptor::Log;

    #[link(wasm_import_module = "__main_module__")]
    extern "C" {
        fn _start();
    }
    _start()
}

/// The maximum path length. WASI doesn't explicitly guarantee this, but all
/// popular OS's have a `PATH_MAX` of at most 4096, so that's enough for this
/// polyfill.
const PATH_MAX: usize = 4096;

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

// These functions are defined by the object that the build.rs script produces.
extern "C" {
    fn replace_realloc_global_ptr(val: *mut u8) -> *mut u8;
    fn replace_realloc_global_len(val: usize) -> usize;
    fn replace_fds(val: *mut u8) -> *mut u8;
}

/// Register `buf` and `buf_len` to be used by `cabi_realloc` to satisfy the
/// next request.
unsafe fn register_buffer(buf: *mut u8, buf_len: usize) {
    let old_ptr = replace_realloc_global_ptr(buf);
    assert!(old_ptr.is_null());
    let old_len = replace_realloc_global_len(buf_len);
    assert_eq!(old_len, 0);
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
    let ptr = replace_realloc_global_ptr(null_mut());
    if ptr.is_null() {
        unreachable();
    }
    let len = replace_realloc_global_len(0);
    if len < new_size {
        unreachable();
    }
    ptr
}

#[no_mangle]
pub unsafe extern "C" fn cabi_export_realloc(
    old_ptr: *mut u8,
    old_size: usize,
    align: usize,
    new_size: usize,
) -> *mut u8 {
    if !old_ptr.is_null() {
        unreachable();
    }
    if new_size > PAGE_SIZE {
        unreachable();
    }
    let grew = core::arch::wasm32::memory_grow(0, 1);
    if grew == usize::MAX {
        unreachable();
    }
    (grew * PAGE_SIZE) as *mut u8
}

/// Read command-line argument data.
/// The size of the array should match that returned by `args_sizes_get`
#[no_mangle]
pub unsafe extern "C" fn args_get(argv: *mut *mut u8, argv_buf: *mut u8) -> Errno {
    // TODO: Use real arguments.
    // Store bytes one at a time to avoid needing a static init.
    argv_buf.add(0).write(b'w');
    argv_buf.add(1).write(b'a');
    argv_buf.add(2).write(b's');
    argv_buf.add(3).write(b'm');
    argv_buf.add(4).write(b'\0');
    argv.add(0).write(argv_buf);
    argv.add(1).write(null_mut());
    ERRNO_SUCCESS
}

/// Return command-line argument data sizes.
#[no_mangle]
pub unsafe extern "C" fn args_sizes_get(argc: *mut Size, argv_buf_size: *mut Size) -> Errno {
    // TODO: Use real arguments.
    *argc = 1;
    *argv_buf_size = 5;
    ERRNO_SUCCESS
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
pub unsafe extern "C" fn clock_res_get(id: Clockid, resolution: *mut Timestamp) -> Errno {
    unreachable()
}

/// Return the time value of a clock.
/// Note: This is similar to `clock_gettime` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn clock_time_get(
    id: Clockid,
    precision: Timestamp,
    time: *mut Timestamp,
) -> Errno {
    unreachable()
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
    unreachable()
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
    match Descriptor::get(fd) {
        Descriptor::File(file) => match wasi_filesystem::datasync(file.fd) {
            Ok(()) => ERRNO_SUCCESS,
            Err(err) => errno_from_wasi_filesystem(err),
        },
        Descriptor::Log => ERRNO_INVAL,
        Descriptor::Closed => ERRNO_BADF,
    }
}

/// Get the attributes of a file descriptor.
/// Note: This returns similar flags to `fsync(fd, F_GETFL)` in POSIX, as well as additional fields.
#[no_mangle]
pub unsafe extern "C" fn fd_fdstat_get(fd: Fd, stat: *mut Fdstat) -> Errno {
    match Descriptor::get(fd) {
        Descriptor::File(file) => {
            let info = match wasi_filesystem::fd_info(file.fd) {
                Ok(info) => info,
                Err(err) => return errno_from_wasi_filesystem(err),
            };

            let fs_filetype = match info.type_ {
                wasi_filesystem::Type::RegularFile => FILETYPE_REGULAR_FILE,
                wasi_filesystem::Type::Directory => FILETYPE_DIRECTORY,
                wasi_filesystem::Type::BlockDevice => FILETYPE_BLOCK_DEVICE,
                wasi_filesystem::Type::CharacterDevice => FILETYPE_CHARACTER_DEVICE,
                wasi_filesystem::Type::Fifo => FILETYPE_UNKNOWN,
                wasi_filesystem::Type::Socket => FILETYPE_SOCKET_STREAM,
                wasi_filesystem::Type::SymbolicLink => FILETYPE_SYMBOLIC_LINK,
                wasi_filesystem::Type::Unknown => FILETYPE_UNKNOWN,
            };

            let mut fs_flags = 0;
            let mut fs_rights_base = !0;
            if !info.flags.contains(wasi_filesystem::Flags::READ) {
                fs_rights_base &= !RIGHTS_FD_READ;
            }
            if !info.flags.contains(wasi_filesystem::Flags::WRITE) {
                fs_rights_base &= !RIGHTS_FD_WRITE;
            }
            if info.flags.contains(wasi_filesystem::Flags::APPEND) {
                fs_flags |= FDFLAGS_APPEND;
            }
            if info.flags.contains(wasi_filesystem::Flags::DSYNC) {
                fs_flags |= FDFLAGS_DSYNC;
            }
            if info.flags.contains(wasi_filesystem::Flags::NONBLOCK) {
                fs_flags |= FDFLAGS_NONBLOCK;
            }
            if info.flags.contains(wasi_filesystem::Flags::RSYNC) {
                fs_flags |= FDFLAGS_RSYNC;
            }
            if info.flags.contains(wasi_filesystem::Flags::SYNC) {
                fs_flags |= FDFLAGS_SYNC;
            }
            let fs_rights_inheriting = fs_rights_base;

            stat.write(Fdstat {
                fs_filetype,
                fs_flags,
                fs_rights_base,
                fs_rights_inheriting,
            });
            ERRNO_SUCCESS
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
            ERRNO_SUCCESS
        }
        Descriptor::Closed => ERRNO_BADF,
    }
}

/// Adjust the flags associated with a file descriptor.
/// Note: This is similar to `fcntl(fd, F_SETFL, flags)` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn fd_fdstat_set_flags(fd: Fd, flags: Fdflags) -> Errno {
    unreachable()
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
    unreachable()
}

/// Adjust the size of an open file. If this increases the file's size, the extra bytes are filled with zeros.
/// Note: This is similar to `ftruncate` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn fd_filestat_set_size(fd: Fd, size: Filesize) -> Errno {
    unreachable()
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
    unreachable()
}

/// Read from a file descriptor, without using and updating the file descriptor's offset.
/// Note: This is similar to `preadv` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn fd_pread(
    fd: Fd,
    iovs_ptr: *const Iovec,
    iovs_len: usize,
    offset: Filesize,
    nread: *mut Size,
) -> Errno {
    unreachable()
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
    iovs_ptr: *const Ciovec,
    iovs_len: usize,
    offset: Filesize,
    nwritten: *mut Size,
) -> Errno {
    unreachable()
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

    match Descriptor::get(fd) {
        Descriptor::File(file) => {
            let ptr = (*iovs_ptr).buf;
            let len = (*iovs_ptr).buf_len;

            register_buffer(ptr, len);

            // We can cast between a `usize`-sized value and `usize`.
            let read_len = len as _;
            let result = wasi_filesystem::pread(file.fd, read_len, file.position);
            match result {
                Ok(data) => {
                    assert_eq!(data.as_ptr(), ptr);
                    assert!(data.len() <= len);
                    *nread = data.len();
                    file.position += data.len() as u64;
                    forget(data);
                    ERRNO_SUCCESS
                }
                Err(err) => errno_from_wasi_filesystem(err),
            }
        }
        Descriptor::Closed | Descriptor::Log => ERRNO_BADF,
    }
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
    match Descriptor::get(fd) {
        Descriptor::File(file) => {
            // It's ok to cast these indices; the WASI API will fail if
            // the resulting values are out of range.
            let from = match whence {
                WHENCE_SET => wasi_filesystem::SeekFrom::Set(offset as _),
                WHENCE_CUR => wasi_filesystem::SeekFrom::Cur(offset),
                WHENCE_END => wasi_filesystem::SeekFrom::End(offset as _),
                _ => return ERRNO_INVAL,
            };
            match wasi_filesystem::seek(file.fd, from) {
                Ok(result) => {
                    *newoffset = result;
                    ERRNO_SUCCESS
                }
                Err(err) => errno_from_wasi_filesystem(err),
            }
        }
        Descriptor::Log => ERRNO_SPIPE,
        Descriptor::Closed => ERRNO_BADF,
    }
}

/// Synchronize the data and metadata of a file to disk.
/// Note: This is similar to `fsync` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn fd_sync(fd: Fd) -> Errno {
    match Descriptor::get(fd) {
        Descriptor::File(file) => match wasi_filesystem::sync(file.fd) {
            Ok(()) => ERRNO_SUCCESS,
            Err(err) => errno_from_wasi_filesystem(err),
        },
        Descriptor::Log => ERRNO_INVAL,
        Descriptor::Closed => ERRNO_BADF,
    }
}

/// Return the current offset of a file descriptor.
/// Note: This is similar to `lseek(fd, 0, SEEK_CUR)` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn fd_tell(fd: Fd, offset: *mut Filesize) -> Errno {
    match Descriptor::get(fd) {
        Descriptor::File(file) => match wasi_filesystem::tell(file.fd) {
            Ok(result) => {
                *offset = result;
                ERRNO_SUCCESS
            }
            Err(err) => errno_from_wasi_filesystem(err),
        },
        Descriptor::Log => ERRNO_SPIPE,
        Descriptor::Closed => ERRNO_BADF,
    }
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

    match Descriptor::get(fd) {
        Descriptor::File(file) => {
            let result =
                wasi_filesystem::pwrite(file.fd, slice::from_raw_parts(ptr, len), file.position);

            match result {
                Ok(bytes) => {
                    *nwritten = bytes as usize;
                    file.position += bytes as u64;
                    ERRNO_SUCCESS
                }
                Err(err) => errno_from_wasi_filesystem(err),
            }
        }
        Descriptor::Log => {
            let bytes = slice::from_raw_parts(ptr, len);
            let context: [u8; 3] = [b'I', b'/', b'O'];
            wasi_logging::log(wasi_logging::Level::Info, &context, bytes);
            *nwritten = len;
            ERRNO_SUCCESS
        }
        Descriptor::Closed => ERRNO_BADF,
    }
}

/// Create a directory.
/// Note: This is similar to `mkdirat` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn path_create_directory(
    fd: Fd,
    path_ptr: *const u8,
    path_len: usize,
) -> Errno {
    unreachable()
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
    unreachable()
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
    unreachable()
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
    unreachable()
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

    // If the user gave us a buffer shorter than `PATH_MAX`, it may not be
    // long enough to accept the actual path. `cabi_realloc` can't fail,
    // so instead we handle this case specially.
    if buf_len < PATH_MAX {
        return path_readlink_slow(fd, path, buf, buf_len, bufused);
    }

    register_buffer(buf, buf_len);

    let result = wasi_filesystem::readlink_at(fd, path);

    let return_value = match &result {
        Ok(path) => {
            assert_eq!(path.as_ptr(), buf);
            assert!(path.len() <= buf_len);

            *bufused = path.len();
            ERRNO_SUCCESS
        }
        Err(err) => errno_from_wasi_filesystem(*err),
    };

    // The returned string's memory was allocated in `buf`, so don't separately
    // free it.
    forget(result);

    return_value
}

/// Slow-path for `path_readlink` that allocates a buffer on the stack to
/// ensure that it has a big enough buffer.
#[inline(never)] // Disable inlining as this has a large stack buffer.
unsafe fn path_readlink_slow(
    fd: wasi_filesystem::Descriptor,
    path: &[u8],
    buf: *mut u8,
    buf_len: Size,
    bufused: *mut Size,
) -> Errno {
    let mut buffer = core::mem::MaybeUninit::<[u8; PATH_MAX]>::uninit();

    register_buffer(buffer.as_mut_ptr().cast(), PATH_MAX);

    let result = wasi_filesystem::readlink_at(fd, path);

    let return_value = match &result {
        Ok(path) => {
            assert_eq!(path.as_ptr(), buffer.as_ptr().cast());
            assert!(path.len() <= PATH_MAX);

            // Preview1 follows POSIX in truncating the returned path if
            // it doesn't fit.
            let len = core::cmp::min(path.len(), buf_len);
            copy_nonoverlapping(buffer.as_ptr().cast(), buf, len);
            *bufused = len;
            ERRNO_SUCCESS
        }
        Err(err) => errno_from_wasi_filesystem(*err),
    };

    // The returned string's memory was allocated in `buf`, so don't separately
    // free it.
    forget(result);

    return_value
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
    unreachable()
}

/// Rename a file or directory.
/// Note: This is similar to `renameat` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn path_rename(
    fd: Fd,
    old_path_ptr: *const u8,
    old_path_len: usize,
    new_fd: Fd,
    new_path_ptr: *const u8,
    new_path_len: usize,
) -> Errno {
    unreachable()
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
    unreachable()
}

/// Unlink a file.
/// Return `errno::isdir` if the path refers to a directory.
/// Note: This is similar to `unlinkat(fd, path, 0)` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn path_unlink_file(fd: Fd, path_ptr: *const u8, path_len: usize) -> Errno {
    unreachable()
}

/// Concurrently poll for the occurrence of a set of events.
#[no_mangle]
pub unsafe extern "C" fn poll_oneoff(
    r#in: *const Subscription,
    out: *mut Event,
    nsubscriptions: Size,
    nevents: *mut Size,
) -> Errno {
    unreachable()
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
    register_buffer(buf, buf_len);

    assert_eq!(buf_len as u32 as Size, buf_len);
    let result = wasi_random::getrandom(buf_len as u32);
    assert_eq!(result.as_ptr(), buf);

    // The returned buffer's memory was allocated in `buf`, so don't separately
    // free it.
    forget(result);

    ERRNO_SUCCESS
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

#[inline(never)] // Disable inlining as this is bulky and relatively cold.
fn errno_from_wasi_filesystem(err: wasi_filesystem::Errno) -> Errno {
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
    Log,
}

#[repr(C)]
pub struct File {
    fd: wasi_filesystem::Descriptor,
    position: u64,
}

const PAGE_SIZE: usize = 65536;

impl Descriptor {
    fn get(fd: Fd) -> &'static mut Descriptor {
        unsafe {
            let mut fds = replace_fds(null_mut());
            if fds.is_null() {
                let grew = core::arch::wasm32::memory_grow(0, 1);
                if grew == usize::MAX {
                    unreachable();
                }
                fds = (grew * PAGE_SIZE) as *mut u8;
            }
            // We allocated a page; abort if that's not enough.
            if fd as usize * size_of::<Descriptor>() >= PAGE_SIZE {
                unreachable()
            }
            let result = &mut *fds.cast::<Descriptor>().add(fd as usize);
            let fds = replace_fds(fds);
            assert!(fds.is_null());
            result
        }
    }
}
