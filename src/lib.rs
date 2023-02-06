#![allow(unused_variables)] // TODO: remove this when more things are implemented

use crate::bindings::{
    wasi_clocks, wasi_default_clocks, wasi_exit, wasi_filesystem, wasi_io, wasi_poll, wasi_random,
    wasi_stderr, wasi_tcp,
};
use core::arch::wasm32;
use core::cell::{Cell, RefCell, UnsafeCell};
use core::cmp::min;
use core::ffi::c_void;
use core::hint::black_box;
use core::mem::{self, align_of, forget, replace, size_of, ManuallyDrop, MaybeUninit};
use core::ptr::{self, null_mut};
use core::slice;
use wasi::*;
use wasi_poll::{InputStream, OutputStream, Pollable};

#[macro_use]
mod macros;

mod bindings {
    #[cfg(feature = "command")]
    wit_bindgen_guest_rust::generate!({
        world: "wasi-command",
        no_std,
        raw_strings,
        unchecked,
        // The generated definition of command will pull in std, so we are defining it
        // manually below instead
        skip: ["command"],
    });

    #[cfg(not(feature = "command"))]
    wit_bindgen_guest_rust::generate!({
        world: "wasi",
        no_std,
        raw_strings,
        unchecked,
    });
}

#[no_mangle]
#[cfg(feature = "command")]
pub unsafe extern "C" fn command(
    stdin: InputStream,
    stdout: OutputStream,
    args_ptr: *const WasmStr,
    args_len: usize,
    env_vars: StrTupleList,
    preopens: PreopenList,
) -> u32 {
    State::with_mut(|state| {
        // Initialization of `State` automatically fills in some dummy
        // structures for fds 0, 1, and 2. Overwrite the stdin/stdout slots of 0
        // and 1 with actual files.
        {
            let descriptors = state.descriptors_mut();
            if descriptors.len() < 3 {
                unreachable!("insufficient memory for stdio descriptors");
            }
            descriptors[0] = Descriptor::Streams(Streams {
                input: Cell::new(Some(stdin)),
                output: Cell::new(None),
                type_: StreamType::Unknown,
            });
            descriptors[1] = Descriptor::Streams(Streams {
                input: Cell::new(None),
                output: Cell::new(Some(stdout)),
                type_: StreamType::Unknown,
            });
        }
        state.args = Some(slice::from_raw_parts(args_ptr, args_len));
        state.env_vars = Some(slice::from_raw_parts(env_vars.base, env_vars.len));

        let preopens = slice::from_raw_parts(preopens.base, preopens.len);
        state.preopens = Some(preopens);

        for preopen in preopens {
            unwrap_result(state.push_desc(Descriptor::Streams(Streams {
                input: Cell::new(None),
                output: Cell::new(None),
                type_: StreamType::File(File {
                    fd: preopen.descriptor,
                    position: Cell::new(0),
                    append: false,
                }),
            })));
        }

        Ok(())
    });

    #[link(wasm_import_module = "__main_module__")]
    extern "C" {
        fn _start();
    }
    _start();
    0
}

fn unwrap<T>(maybe: Option<T>) -> T {
    if let Some(value) = maybe {
        value
    } else {
        unreachable!("unwrap failed")
    }
}

fn unwrap_result<T, E>(result: Result<T, E>) -> T {
    if let Ok(value) = result {
        value
    } else {
        unreachable!("unwrap result failed")
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
        unreachable!();
    }
    let mut ptr = null_mut::<u8>();
    State::with(|state| {
        ptr = state.buffer_ptr.replace(null_mut());
        if ptr.is_null() {
            unreachable!();
        }
        let len = state.buffer_len.replace(0);
        if len < new_size {
            unreachable!();
        }
        Ok(())
    });
    ptr
}

fn align_to(ptr: usize, align: usize) -> usize {
    (ptr + (align - 1)) & !(align - 1)
}

/// This allocator is only used for the `command` entrypoint.
///
/// The implementation here is a bump allocator into `State::command_data` which
/// traps when it runs out of data. This means that the total size of
/// arguments/env/etc coming into a component is bounded by the current 64k
/// (ish) limit. That's just an implementation limit though which can be lifted
/// by dynamically calling the main module's allocator as necessary for more data.
#[no_mangle]
pub unsafe extern "C" fn cabi_export_realloc(
    old_ptr: *mut u8,
    old_size: usize,
    align: usize,
    new_size: usize,
) -> *mut u8 {
    if !old_ptr.is_null() || old_size != 0 {
        unreachable!();
    }
    let mut ret = null_mut::<u8>();
    State::with_mut(|state| {
        let data = state.command_data.as_mut_ptr();
        let ptr = align_to(
            unwrap_result(usize::try_from(state.command_data_next)),
            align,
        );

        // "oom" as too much argument data tried to flow into the component.
        // Ideally this would have a better error message?
        if ptr + new_size > (*data).len() {
            unreachable!("out of memory");
        }
        state.command_data_next = (ptr + new_size)
            .try_into()
            .unwrap_or_else(|_| unreachable!());
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
    State::with(|state| {
        if let Some(list) = state.env_vars {
            let mut offsets = environ;
            let mut buffer = environ_buf;
            for pair in list {
                ptr::write(offsets, buffer);
                offsets = offsets.add(1);

                ptr::copy_nonoverlapping(pair.key.ptr, buffer, pair.key.len);
                buffer = buffer.add(pair.key.len);

                ptr::write(buffer, b'=');
                buffer = buffer.add(1);

                ptr::copy_nonoverlapping(pair.value.ptr, buffer, pair.value.len);
                buffer = buffer.add(pair.value.len);

                ptr::write(buffer, 0);
                buffer = buffer.add(1);
            }
        }

        Ok(())
    })
}

/// Return environment variable data sizes.
#[no_mangle]
pub unsafe extern "C" fn environ_sizes_get(
    environc: *mut Size,
    environ_buf_size: *mut Size,
) -> Errno {
    State::with(|state| {
        if let Some(list) = state.env_vars {
            *environc = list.len();
            *environ_buf_size = {
                let mut sum = 0;
                for pair in list {
                    sum += pair.key.len + pair.value.len + 2;
                }
                sum
            };
        } else {
            *environc = 0;
            *environ_buf_size = 0;
        }

        Ok(())
    })
}

/// Return the resolution of a clock.
/// Implementations are required to provide a non-zero value for supported clocks. For unsupported clocks,
/// return `errno::inval`.
/// Note: This is similar to `clock_getres` in POSIX.
#[no_mangle]
pub extern "C" fn clock_res_get(id: Clockid, resolution: &mut Timestamp) -> Errno {
    State::with(|state| {
        match id {
            CLOCKID_MONOTONIC => {
                let res = wasi_clocks::monotonic_clock_resolution(state.default_monotonic_clock());
                *resolution = res;
            }
            CLOCKID_REALTIME => {
                let res = wasi_clocks::wall_clock_resolution(state.default_wall_clock());
                *resolution = Timestamp::from(res.nanoseconds)
                    .checked_add(res.seconds)
                    .and_then(|secs| secs.checked_mul(1_000_000_000))
                    .ok_or(ERRNO_OVERFLOW)?;
            }
            _ => unreachable!(),
        }
        Ok(())
    })
}

/// Return the time value of a clock.
/// Note: This is similar to `clock_gettime` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn clock_time_get(
    id: Clockid,
    _precision: Timestamp,
    time: &mut Timestamp,
) -> Errno {
    State::with(|state| {
        match id {
            CLOCKID_MONOTONIC => {
                *time = wasi_clocks::monotonic_clock_now(state.default_monotonic_clock());
            }
            CLOCKID_REALTIME => {
                let res = wasi_clocks::wall_clock_now(state.default_wall_clock());
                *time = Timestamp::from(res.nanoseconds)
                    .checked_add(res.seconds)
                    .and_then(|secs| secs.checked_mul(1_000_000_000))
                    .ok_or(ERRNO_OVERFLOW)?;
            }
            _ => unreachable!(),
        }
        Ok(())
    })
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
    unreachable!("fd_allocate")
}

/// Close a file descriptor.
/// Note: This is similar to `close` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn fd_close(fd: Fd) -> Errno {
    State::with_mut(|state| {
        // If there's a dirent cache entry for this file descriptor then drop
        // it since the descriptor is being closed and future calls to
        // `fd_readdir` should return an error.
        if fd == state.dirent_cache.for_fd.get() {
            drop(state.dirent_cache.stream.replace(None));
        }

        let closed = state.closed;
        let desc = state.get_mut(fd)?;
        *desc = Descriptor::Closed(closed);
        state.closed = Some(fd);
        Ok(())
    })
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
        Descriptor::Streams(Streams {
            type_: StreamType::File(file),
            ..
        }) => {
            let flags = wasi_filesystem::flags(file.fd)?;
            let type_ = wasi_filesystem::todo_type(file.fd)?;

            let fs_filetype = type_.into();

            let mut fs_flags = 0;
            let mut fs_rights_base = !0;
            if !flags.contains(wasi_filesystem::DescriptorFlags::READ) {
                fs_rights_base &= !RIGHTS_FD_READ;
            }
            if !flags.contains(wasi_filesystem::DescriptorFlags::WRITE) {
                fs_rights_base &= !RIGHTS_FD_WRITE;
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
        Descriptor::Stderr => {
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
        Descriptor::Streams(Streams {
            input,
            output,
            type_: StreamType::Socket(_),
        })
        | Descriptor::Streams(Streams {
            input,
            output,
            type_: StreamType::Unknown,
        }) => {
            let fs_filetype = FILETYPE_UNKNOWN;
            let fs_flags = 0;
            let mut fs_rights_base = 0;
            if input.get().is_some() {
                fs_rights_base |= RIGHTS_FD_READ;
            }
            if output.get().is_some() {
                fs_rights_base |= RIGHTS_FD_WRITE;
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
        Descriptor::Streams(Streams {
            input,
            output,
            type_: StreamType::EmptyStdin,
        }) => {
            let fs_filetype = FILETYPE_UNKNOWN;
            let fs_flags = 0;
            let fs_rights_base = RIGHTS_FD_READ;
            let fs_rights_inheriting = fs_rights_base;
            stat.write(Fdstat {
                fs_filetype,
                fs_flags,
                fs_rights_base,
                fs_rights_inheriting,
            });
            Ok(())
        }
        Descriptor::Closed(_) => Err(ERRNO_BADF),
    })
}

/// Adjust the flags associated with a file descriptor.
/// Note: This is similar to `fcntl(fd, F_SETFL, flags)` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn fd_fdstat_set_flags(fd: Fd, flags: Fdflags) -> Errno {
    let mut new_flags = wasi_filesystem::DescriptorFlags::empty();
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
    unreachable!()
}

/// Return the attributes of an open file.
#[no_mangle]
pub unsafe extern "C" fn fd_filestat_get(fd: Fd, buf: *mut Filestat) -> Errno {
    State::with(|state| {
        let file = state.get_file(fd)?;
        let stat = wasi_filesystem::stat(file.fd)?;
        let filetype = stat.type_.into();
        *buf = Filestat {
            dev: stat.dev,
            ino: stat.ino,
            filetype,
            nlink: stat.nlink,
            size: stat.size,
            atim: datetime_to_timestamp(stat.atim),
            mtim: datetime_to_timestamp(stat.mtim),
            ctim: datetime_to_timestamp(stat.ctim),
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
            wasi_filesystem::NewTimestamp::Timestamp(wasi_filesystem::Datetime {
                seconds: atim / 1_000_000_000,
                nanoseconds: (atim % 1_000_000_000) as _,
            })
        } else {
            wasi_filesystem::NewTimestamp::NoChange
        };
    let mtim =
        if fst_flags & (FSTFLAGS_MTIM | FSTFLAGS_MTIM_NOW) == (FSTFLAGS_MTIM | FSTFLAGS_MTIM_NOW) {
            wasi_filesystem::NewTimestamp::Now
        } else if fst_flags & FSTFLAGS_MTIM == FSTFLAGS_MTIM {
            wasi_filesystem::NewTimestamp::Timestamp(wasi_filesystem::Datetime {
                seconds: mtim / 1_000_000_000,
                nanoseconds: (mtim % 1_000_000_000) as _,
            })
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

        let read_len = unwrap_result(u32::try_from(len));
        let file = state.get_file(fd)?;
        let (data, end) = wasi_filesystem::pread(file.fd, read_len, offset)?;
        assert_eq!(data.as_ptr(), ptr);
        assert!(data.len() <= len);

        let len = data.len();
        forget(data);
        if !end && len == 0 {
            Err(ERRNO_INTR)
        } else {
            *nread = len;
            Ok(())
        }
    })
}

fn get_preopen(state: &State, fd: Fd) -> Option<&Preopen> {
    state.preopens?.get(fd.checked_sub(3)? as usize)
}

/// Return a description of the given preopened file descriptor.
#[no_mangle]
pub unsafe extern "C" fn fd_prestat_get(fd: Fd, buf: *mut Prestat) -> Errno {
    State::with(|state| {
        if let Some(preopen) = get_preopen(state, fd) {
            buf.write(Prestat {
                tag: 0,
                u: PrestatU {
                    dir: PrestatDir {
                        pr_name_len: preopen.path.len,
                    },
                },
            });

            Ok(())
        } else {
            Err(ERRNO_BADF)
        }
    })
}

/// Return a description of the given preopened file descriptor.
#[no_mangle]
pub unsafe extern "C" fn fd_prestat_dir_name(fd: Fd, path: *mut u8, path_len: Size) -> Errno {
    State::with(|state| {
        if let Some(preopen) = get_preopen(state, fd) {
            if preopen.path.len < path_len as usize {
                Err(ERRNO_NAMETOOLONG)
            } else {
                ptr::copy_nonoverlapping(preopen.path.ptr, path, preopen.path.len);
                Ok(())
            }
        } else {
            Err(ERRNO_NOTDIR)
        }
    })
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

    State::with(|state| {
        let file = state.get_seekable_file(fd)?;
        let bytes = wasi_filesystem::pwrite(file.fd, slice::from_raw_parts(ptr, len), offset)?;
        *nwritten = bytes as usize;
        Ok(())
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

    let ptr = (*iovs_ptr).buf;
    let len = (*iovs_ptr).buf_len;

    State::with(|state| {
        state.register_buffer(ptr, len);

        match state.get(fd)? {
            Descriptor::Streams(streams) => {
                let wasi_stream = streams.get_read_stream()?;

                let read_len = unwrap_result(u64::try_from(len));
                let wasi_stream = streams.get_read_stream()?;
                let (data, end) = wasi_io::read(wasi_stream, read_len).map_err(|_| ERRNO_IO)?;

                assert_eq!(data.as_ptr(), ptr);
                assert!(data.len() <= len);

                // If this is a file, keep the current-position pointer up to date.
                if let StreamType::File(file) = &streams.type_ {
                    file.position
                        .set(file.position.get() + data.len() as wasi_filesystem::Filesize);
                }

                let len = data.len();
                forget(data);
                if !end && len == 0 {
                    Err(ERRNO_INTR)
                } else {
                    *nread = len;
                    Ok(())
                }
            }
            Descriptor::Stderr | Descriptor::Closed(_) => Err(ERRNO_BADF),
        }
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
    let mut buf = slice::from_raw_parts_mut(buf, buf_len);
    return State::with(|state| {
        // First determine if there's an entry in the dirent cache to use. This
        // is done to optimize the use case where a large directory is being
        // used with a fixed-sized buffer to avoid re-invoking the `readdir`
        // function and continuing to use the same iterator.
        //
        // This is a bit tricky since the requested state in this function call
        // must match the prior state of the dirent stream, if any, so that's
        // all validated here as well.
        //
        // Note that for the duration of this function the `cookie` specifier is
        // the `n`th iteration of the `readdir` stream return value.
        let prev_stream = state.dirent_cache.stream.replace(None);
        let stream =
            if state.dirent_cache.for_fd.get() == fd && state.dirent_cache.cookie.get() == cookie {
                prev_stream
            } else {
                None
            };
        let mut iter;
        match stream {
            // All our checks passed and a dirent cache was available with a
            // prior stream. Construct an iterator which will yield its first
            // entry from cache and is additionally resuming at the `cookie`
            // specified.
            Some(stream) => {
                iter = DirEntryIterator {
                    stream,
                    state,
                    cookie,
                    use_cache: true,
                }
            }

            // Either a dirent stream wasn't previously available, a different
            // cookie was requested, or a brand new directory is now being read.
            // In these situations fall back to resuming reading the directory
            // from scratch, and the `cookie` value indicates how many items
            // need skipping.
            None => {
                let dir = state.get_dir(fd)?;
                iter = DirEntryIterator {
                    state,
                    cookie: wasi::DIRCOOKIE_START,
                    use_cache: false,
                    stream: DirEntryStream(wasi_filesystem::readdir(dir.fd)?),
                };

                // Skip to the entry that is requested by the `cookie`
                // parameter.
                for _ in wasi::DIRCOOKIE_START..cookie {
                    match iter.next() {
                        Some(Ok(_)) => {}
                        Some(Err(e)) => return Err(e),
                        None => return Ok(()),
                    }
                }
            }
        };

        while buf.len() > 0 {
            let (dirent, name) = match iter.next() {
                Some(Ok(pair)) => pair,
                Some(Err(e)) => return Err(e),
                None => break,
            };

            // Copy a `dirent` describing this entry into the destination `buf`,
            // truncating it if it doesn't fit entirely.
            let bytes = slice::from_raw_parts(
                (&dirent as *const wasi::Dirent).cast::<u8>(),
                size_of::<Dirent>(),
            );
            let dirent_bytes_to_copy = buf.len().min(bytes.len());
            buf[..dirent_bytes_to_copy].copy_from_slice(&bytes[..dirent_bytes_to_copy]);
            buf = &mut buf[dirent_bytes_to_copy..];

            // Copy the name bytes into the output `buf`, truncating it if it
            // doesn't fit.
            //
            // Note that this might be a 0-byte copy if the `dirent` was
            // truncated or fit entirely into the destination.
            let name_bytes_to_copy = buf.len().min(name.len());
            ptr::copy_nonoverlapping(name.as_ptr().cast(), buf.as_mut_ptr(), name_bytes_to_copy);

            buf = &mut buf[name_bytes_to_copy..];

            // If the buffer is empty then that means the value may be
            // truncated, so save the state of the iterator in our dirent cache
            // and return.
            //
            // Note that `cookie - 1` is stored here since `iter.cookie` stores
            // the address of the next item, and we're rewinding one item since
            // the current item is truncated and will want to resume from that
            // in the future.
            //
            // Additionally note that this caching step is skipped if the name
            // to store doesn't actually fit in the dirent cache's path storage.
            // In that case there's not much we can do and let the next call to
            // `fd_readdir` start from scratch.
            if buf.len() == 0 && name.len() <= DIRENT_CACHE {
                let DirEntryIterator { stream, cookie, .. } = iter;
                state.dirent_cache.stream.set(Some(stream));
                state.dirent_cache.for_fd.set(fd);
                state.dirent_cache.cookie.set(cookie - 1);
                state.dirent_cache.cached_dirent.set(dirent);
                ptr::copy(
                    name.as_ptr().cast::<u8>(),
                    (*state.dirent_cache.path_data.get()).as_mut_ptr() as *mut u8,
                    name.len(),
                );
                break;
            }
        }

        *bufused = buf_len - buf.len();
        Ok(())
    });

    struct DirEntryIterator<'a> {
        state: &'a State,
        use_cache: bool,
        cookie: Dircookie,
        stream: DirEntryStream,
    }

    impl<'a> Iterator for DirEntryIterator<'a> {
        // Note the usage of `UnsafeCell<u8>` here to indicate that the data can
        // alias the storage within `state`.
        type Item = Result<(wasi::Dirent, &'a [UnsafeCell<u8>]), Errno>;

        fn next(&mut self) -> Option<Self::Item> {
            self.cookie += 1;

            if self.use_cache {
                self.use_cache = false;
                return Some(unsafe {
                    let dirent = self.state.dirent_cache.cached_dirent.as_ptr().read();
                    let ptr = (*(*self.state.dirent_cache.path_data.get()).as_ptr())
                        .as_ptr()
                        .cast();
                    let buffer = slice::from_raw_parts(ptr, dirent.d_namlen as usize);
                    Ok((dirent, buffer))
                });
            }
            self.state
                .register_buffer(self.state.path_buf.get().cast(), PATH_MAX);
            let entry = match wasi_filesystem::read_dir_entry(self.stream.0) {
                Ok(Some(entry)) => entry,
                Ok(None) => return None,
                Err(e) => return Some(Err(e.into())),
            };

            let wasi_filesystem::DirEntry { ino, type_, name } = entry;
            let name = ManuallyDrop::new(name);
            let dirent = wasi::Dirent {
                d_next: self.cookie,
                d_ino: ino.unwrap_or(0),
                d_namlen: unwrap_result(u32::try_from(name.len())),
                d_type: type_.into(),
            };
            // Extend the lifetime of `name` to the `self.state` lifetime for
            // this iterator since the data for the name lives within state.
            let name = unsafe {
                assert_eq!(name.as_ptr(), self.state.path_buf.get().cast());
                slice::from_raw_parts(name.as_ptr().cast(), name.len())
            };
            Some(Ok((dirent, name)))
        }
    }
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
    State::with_mut(|state| {
        let closed = state.closed;

        // Ensure the table is big enough to contain `to`. Do this before
        // looking up `fd` as it can fail due to `NOMEM`.
        while Fd::from(state.ndescriptors) <= to {
            let old_closed = state.closed;
            let new_closed = state.push_desc(Descriptor::Closed(old_closed))?;
            state.closed = Some(new_closed);
        }

        let fd_desc = state.get_mut(fd)?;
        let desc = replace(fd_desc, Descriptor::Closed(closed));

        let to_desc = unwrap_result(state.get_mut(to));
        *to_desc = desc;
        state.closed = Some(fd);
        Ok(())
    })
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
        let stream = state.get_seekable_stream(fd)?;

        // Seeking only works on files.
        if let StreamType::File(file) = &stream.type_ {
            // It's ok to cast these indices; the WASI API will fail if
            // the resulting values are out of range.
            let from = match whence {
                WHENCE_SET => offset,
                WHENCE_CUR => (file.position.get() as i64).wrapping_add(offset),
                WHENCE_END => (wasi_filesystem::stat(file.fd)?.size as i64) + offset,
                _ => return Err(ERRNO_INVAL),
            };
            stream.input.set(None);
            stream.output.set(None);
            file.position.set(from as wasi_filesystem::Filesize);
            *newoffset = from as wasi_filesystem::Filesize;
            Ok(())
        } else {
            Err(ERRNO_SPIPE)
        }
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
        *offset = file.position.get() as Filesize;
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
    let bytes = slice::from_raw_parts(ptr, len);

    State::with(|state| match state.get(fd)? {
        Descriptor::Streams(streams) => {
            let wasi_stream = streams.get_write_stream()?;
            let bytes = wasi_io::write(wasi_stream, bytes).map_err(|_| ERRNO_IO)?;

            // If this is a file, keep the current-position pointer up to date.
            if let StreamType::File(file) = &streams.type_ {
                // But don't update if we're in append mode. Strictly speaking,
                // we should set the position to the new end of the file, but
                // we don't have an API to do that atomically.
                if !file.append {
                    file.position
                        .set(file.position.get() + wasi_filesystem::Filesize::from(bytes));
                }
            }

            *nwritten = bytes as usize;
            Ok(())
        }
        Descriptor::Stderr => {
            wasi_stderr::print(bytes);
            *nwritten = len;
            Ok(())
        }
        Descriptor::Closed(_) => Err(ERRNO_BADF),
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
        let filetype = stat.type_.into();
        *buf = Filestat {
            dev: stat.dev,
            ino: stat.ino,
            filetype,
            nlink: stat.nlink,
            size: stat.size,
            atim: datetime_to_timestamp(stat.atim),
            mtim: datetime_to_timestamp(stat.mtim),
            ctim: datetime_to_timestamp(stat.ctim),
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
            wasi_filesystem::NewTimestamp::Timestamp(wasi_filesystem::Datetime {
                seconds: atim / 1_000_000_000,
                nanoseconds: (atim % 1_000_000_000) as _,
            })
        } else {
            wasi_filesystem::NewTimestamp::NoChange
        };
    let mtim =
        if fst_flags & (FSTFLAGS_MTIM | FSTFLAGS_MTIM_NOW) == (FSTFLAGS_MTIM | FSTFLAGS_MTIM_NOW) {
            wasi_filesystem::NewTimestamp::Now
        } else if fst_flags & FSTFLAGS_MTIM == FSTFLAGS_MTIM {
            wasi_filesystem::NewTimestamp::Timestamp(wasi_filesystem::Datetime {
                seconds: mtim / 1_000_000_000,
                nanoseconds: (mtim % 1_000_000_000) as _,
            })
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
    drop(fs_rights_inheriting);

    let path = slice::from_raw_parts(path_ptr, path_len);
    let at_flags = at_flags_from_lookupflags(dirflags);
    let o_flags = o_flags_from_oflags(oflags);
    let flags = descriptor_flags_from_flags(fs_rights_base, fdflags);
    let mode = wasi_filesystem::Mode::READABLE | wasi_filesystem::Mode::WRITEABLE;
    let append = fdflags & wasi::FDFLAGS_APPEND == wasi::FDFLAGS_APPEND;

    State::with_mut(|state| {
        let file = state.get_dir(fd)?;
        let result = wasi_filesystem::open_at(file.fd, at_flags, path, o_flags, flags, mode)?;
        let desc = Descriptor::Streams(Streams {
            input: Cell::new(None),
            output: Cell::new(None),
            type_: StreamType::File(File {
                fd: result,
                position: Cell::new(0),
                append,
            }),
        });

        let fd = match state.closed {
            // No free fds; create a new one.
            None => state.push_desc(desc)?,
            // `recycle_fd` is a free fd.
            Some(recycle_fd) => {
                let recycle_desc = unwrap_result(state.get_mut(recycle_fd));
                let next_closed = match recycle_desc {
                    Descriptor::Closed(next) => *next,
                    _ => unreachable!(),
                };
                *recycle_desc = desc;
                state.closed = next_closed;
                recycle_fd
            }
        };

        *opened_fd = fd;
        Ok(())
    })
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
            let len = min(path.len(), buf_len);
            ptr::copy_nonoverlapping(path.as_ptr().cast(), buf, len);
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

struct Pollables {
    pointer: *mut Pollable,
    index: usize,
    length: usize,
}

impl Pollables {
    unsafe fn push(&mut self, pollable: Pollable) {
        assert!(self.index < self.length);
        *self.pointer.add(self.index) = pollable;
        self.index += 1;
    }
}

impl Drop for Pollables {
    fn drop(&mut self) {
        for i in 0..self.index {
            wasi_poll::drop_pollable(unsafe { *self.pointer.add(i) })
        }
    }
}

impl From<wasi_tcp::Errno> for Errno {
    fn from(error: wasi_tcp::Errno) -> Errno {
        use wasi_tcp::Errno::*;

        match error {
            // Use a black box to prevent the optimizer from generating a
            // lookup table, which would require a static initializer.
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

    // We're going to split the `nevents` buffer into two non-overlapping
    // buffers: one to store the pollable handles, and the other to store
    // the bool results.
    //
    // First, we assert that this is possible:
    assert!(align_of::<Event>() >= align_of::<Pollable>());
    assert!(align_of::<Pollable>() >= align_of::<u8>());
    assert!(
        unwrap(nsubscriptions.checked_mul(size_of::<Event>()))
            >= unwrap(
                unwrap(nsubscriptions.checked_mul(size_of::<Pollable>()))
                    .checked_add(unwrap(nsubscriptions.checked_mul(size_of::<u8>())))
            )
    );

    // Store the pollable handles at the beginning, and the bool results at the
    // end, so that we don't clobber the bool results when writting the events.
    let pollables = out as *mut c_void as *mut Pollable;
    let results = out.add(nsubscriptions).cast::<u8>().sub(nsubscriptions);

    // Indefinite sleeping is not supported in preview1.
    if nsubscriptions == 0 {
        return ERRNO_INVAL;
    }

    State::with(|state| {
        state.register_buffer(
            results,
            unwrap(nsubscriptions.checked_mul(size_of::<bool>())),
        );

        let mut pollables = Pollables {
            pointer: pollables,
            index: 0,
            length: nsubscriptions,
        };

        for subscription in subscriptions {
            const EVENTTYPE_CLOCK: u8 = wasi::EVENTTYPE_CLOCK.raw();
            const EVENTTYPE_FD_READ: u8 = wasi::EVENTTYPE_FD_READ.raw();
            const EVENTTYPE_FD_WRITE: u8 = wasi::EVENTTYPE_FD_WRITE.raw();
            pollables.push(match subscription.u.tag {
                EVENTTYPE_CLOCK => {
                    let clock = &subscription.u.u.clock;
                    let absolute = (clock.flags & SUBCLOCKFLAGS_SUBSCRIPTION_CLOCK_ABSTIME)
                        == SUBCLOCKFLAGS_SUBSCRIPTION_CLOCK_ABSTIME;
                    match clock.id {
                        CLOCKID_REALTIME => {
                            let timeout = if absolute {
                                // Convert `clock.timeout` to `Datetime`.
                                let mut datetime = wasi_clocks::Datetime {
                                    seconds: clock.timeout / 1_000_000_000,
                                    nanoseconds: (clock.timeout % 1_000_000_000) as _,
                                };

                                // Subtract `now`.
                                let now = wasi_clocks::wall_clock_now(state.default_wall_clock());
                                datetime.seconds -= now.seconds;
                                if datetime.nanoseconds < now.nanoseconds {
                                    datetime.seconds -= 1;
                                    datetime.nanoseconds += 1_000_000_000;
                                }
                                datetime.nanoseconds -= now.nanoseconds;

                                // Convert to nanoseconds.
                                let nanos = datetime
                                    .seconds
                                    .checked_mul(1_000_000_000)
                                    .ok_or(ERRNO_OVERFLOW)?;
                                nanos
                                    .checked_add(datetime.nanoseconds.into())
                                    .ok_or(ERRNO_OVERFLOW)?
                            } else {
                                clock.timeout
                            };

                            wasi_poll::subscribe_monotonic_clock(
                                state.default_monotonic_clock(),
                                timeout,
                                false,
                            )
                        }

                        CLOCKID_MONOTONIC => wasi_poll::subscribe_monotonic_clock(
                            state.default_monotonic_clock(),
                            clock.timeout,
                            absolute,
                        ),

                        _ => return Err(ERRNO_INVAL),
                    }
                }

                EVENTTYPE_FD_READ => {
                    match state.get_read_stream(subscription.u.u.fd_read.file_descriptor) {
                        Ok(stream) => wasi_poll::subscribe_read(stream),
                        // If the file descriptor isn't a stream, request a
                        // pollable which completes immediately so that it'll
                        // immediately fail.
                        Err(ERRNO_BADF) => wasi_poll::subscribe_monotonic_clock(
                            state.default_monotonic_clock(),
                            0,
                            false,
                        ),
                        Err(e) => return Err(e),
                    }
                }

                EVENTTYPE_FD_WRITE => {
                    match state.get_write_stream(subscription.u.u.fd_write.file_descriptor) {
                        Ok(stream) => wasi_poll::subscribe_write(stream),
                        // If the file descriptor isn't a stream, request a
                        // pollable which completes immediately so that it'll
                        // immediately fail.
                        Err(ERRNO_BADF) => wasi_poll::subscribe_monotonic_clock(
                            state.default_monotonic_clock(),
                            0,
                            false,
                        ),
                        Err(e) => return Err(e),
                    }
                }

                _ => return Err(ERRNO_INVAL),
            });
        }

        let vec =
            wasi_poll::poll_oneoff(slice::from_raw_parts(pollables.pointer, pollables.length));

        assert_eq!(vec.len(), nsubscriptions);
        assert_eq!(vec.as_ptr(), results);
        forget(vec);

        drop(pollables);

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
                    let desc = unwrap_result(state.get(subscription.u.u.fd_read.file_descriptor));
                    match desc {
                        Descriptor::Streams(streams) => match &streams.type_ {
                            StreamType::File(file) => match wasi_filesystem::stat(file.fd) {
                                Ok(stat) => {
                                    error = ERRNO_SUCCESS;
                                    nbytes = stat.size.saturating_sub(file.position.get());
                                    flags = if nbytes == 0 {
                                        EVENTRWFLAGS_FD_READWRITE_HANGUP
                                    } else {
                                        0
                                    };
                                }
                                Err(e) => {
                                    error = e.into();
                                    nbytes = 1;
                                    flags = 0;
                                }
                            },
                            StreamType::Socket(connection) => {
                                match wasi_tcp::bytes_readable(*connection) {
                                    Ok(result) => {
                                        error = ERRNO_SUCCESS;
                                        nbytes = result.0;
                                        flags = if result.1 {
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
                            StreamType::EmptyStdin => {
                                error = ERRNO_SUCCESS;
                                nbytes = 0;
                                flags = EVENTRWFLAGS_FD_READWRITE_HANGUP;
                            }
                            StreamType::Unknown => {
                                error = ERRNO_SUCCESS;
                                nbytes = 1;
                                flags = 0;
                            }
                        },
                        _ => unreachable!(),
                    }
                }
                2 => {
                    type_ = EVENTTYPE_FD_WRITE;
                    let desc = unwrap_result(state.get(subscription.u.u.fd_read.file_descriptor));
                    match desc {
                        Descriptor::Streams(streams) => match streams.type_ {
                            StreamType::File(_) | StreamType::Unknown => {
                                error = ERRNO_SUCCESS;
                                nbytes = 1;
                                flags = 0;
                            }
                            StreamType::Socket(connection) => {
                                match wasi_tcp::bytes_writable(connection) {
                                    Ok(result) => {
                                        error = ERRNO_SUCCESS;
                                        nbytes = result.0;
                                        flags = if result.1 {
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
                            StreamType::EmptyStdin => {
                                error = ERRNO_BADF;
                                nbytes = 0;
                                flags = 0;
                            }
                        },
                        _ => unreachable!(),
                    }
                }

                _ => unreachable!(),
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
    let status = if rval == 0 { Ok(()) } else { Err(()) };
    wasi_exit::exit(status); // does not return
    unreachable!("host exit implementation didn't exit!") // actually unreachable
}

/// Send a signal to the process of the calling thread.
/// Note: This is similar to `raise` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn proc_raise(sig: Signal) -> Errno {
    unreachable!()
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
        let result = wasi_random::get_random_bytes(buf_len as u32);
        assert_eq!(result.as_ptr(), buf);

        // The returned buffer's memory was allocated in `buf`, so don't separately
        // free it.
        forget(result);

        Ok(())
    })
}

/// Accept a new incoming connection.
/// Note: This is similar to `accept` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn sock_accept(fd: Fd, flags: Fdflags, connection: *mut Fd) -> Errno {
    unreachable!()
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
    unreachable!()
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
    unreachable!()
}

/// Shut down socket send and receive channels.
/// Note: This is similar to `shutdown` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn sock_shutdown(fd: Fd, how: Sdflags) -> Errno {
    unreachable!()
}

fn datetime_to_timestamp(datetime: wasi_filesystem::Datetime) -> Timestamp {
    u64::from(datetime.nanoseconds).saturating_add(datetime.seconds.saturating_mul(1_000_000_000))
}

fn at_flags_from_lookupflags(flags: Lookupflags) -> wasi_filesystem::AtFlags {
    if flags & LOOKUPFLAGS_SYMLINK_FOLLOW == LOOKUPFLAGS_SYMLINK_FOLLOW {
        wasi_filesystem::AtFlags::SYMLINK_FOLLOW
    } else {
        wasi_filesystem::AtFlags::empty()
    }
}

fn o_flags_from_oflags(flags: Oflags) -> wasi_filesystem::OFlags {
    let mut o_flags = wasi_filesystem::OFlags::empty();
    if flags & OFLAGS_CREAT == OFLAGS_CREAT {
        o_flags |= wasi_filesystem::OFlags::CREATE;
    }
    if flags & OFLAGS_DIRECTORY == OFLAGS_DIRECTORY {
        o_flags |= wasi_filesystem::OFlags::DIRECTORY;
    }
    if flags & OFLAGS_EXCL == OFLAGS_EXCL {
        o_flags |= wasi_filesystem::OFlags::EXCL;
    }
    if flags & OFLAGS_TRUNC == OFLAGS_TRUNC {
        o_flags |= wasi_filesystem::OFlags::TRUNC;
    }
    o_flags
}

fn descriptor_flags_from_flags(
    rights: Rights,
    fdflags: Fdflags,
) -> wasi_filesystem::DescriptorFlags {
    let mut flags = wasi_filesystem::DescriptorFlags::empty();
    if rights & wasi::RIGHTS_FD_READ == wasi::RIGHTS_FD_READ {
        flags |= wasi_filesystem::DescriptorFlags::READ;
    }
    if rights & wasi::RIGHTS_FD_WRITE == wasi::RIGHTS_FD_WRITE {
        flags |= wasi_filesystem::DescriptorFlags::WRITE;
    }
    if fdflags & wasi::FDFLAGS_SYNC == wasi::FDFLAGS_SYNC {
        flags |= wasi_filesystem::DescriptorFlags::SYNC;
    }
    if fdflags & wasi::FDFLAGS_DSYNC == wasi::FDFLAGS_DSYNC {
        flags |= wasi_filesystem::DescriptorFlags::DSYNC;
    }
    if fdflags & wasi::FDFLAGS_RSYNC == wasi::FDFLAGS_RSYNC {
        flags |= wasi_filesystem::DescriptorFlags::RSYNC;
    }
    if fdflags & wasi::FDFLAGS_NONBLOCK == wasi::FDFLAGS_NONBLOCK {
        flags |= wasi_filesystem::DescriptorFlags::NONBLOCK;
    }
    flags
}

impl From<wasi_filesystem::Errno> for Errno {
    #[inline(never)] // Disable inlining as this is bulky and relatively cold.
    fn from(err: wasi_filesystem::Errno) -> Errno {
        match err {
            // Use a black box to prevent the optimizer from generating a
            // lookup table, which would require a static initializer.
            wasi_filesystem::Errno::Access => black_box(ERRNO_ACCES),
            wasi_filesystem::Errno::Again => ERRNO_AGAIN,
            wasi_filesystem::Errno::Already => ERRNO_ALREADY,
            wasi_filesystem::Errno::Badf => ERRNO_BADF,
            wasi_filesystem::Errno::Busy => ERRNO_BUSY,
            wasi_filesystem::Errno::Deadlk => ERRNO_DEADLK,
            wasi_filesystem::Errno::Dquot => ERRNO_DQUOT,
            wasi_filesystem::Errno::Exist => ERRNO_EXIST,
            wasi_filesystem::Errno::Fbig => ERRNO_FBIG,
            wasi_filesystem::Errno::Ilseq => ERRNO_ILSEQ,
            wasi_filesystem::Errno::Inprogress => ERRNO_INPROGRESS,
            wasi_filesystem::Errno::Intr => ERRNO_INTR,
            wasi_filesystem::Errno::Inval => ERRNO_INVAL,
            wasi_filesystem::Errno::Io => ERRNO_IO,
            wasi_filesystem::Errno::Isdir => ERRNO_ISDIR,
            wasi_filesystem::Errno::Loop => ERRNO_LOOP,
            wasi_filesystem::Errno::Mlink => ERRNO_MLINK,
            wasi_filesystem::Errno::Msgsize => ERRNO_MSGSIZE,
            wasi_filesystem::Errno::Nametoolong => ERRNO_NAMETOOLONG,
            wasi_filesystem::Errno::Nodev => ERRNO_NODEV,
            wasi_filesystem::Errno::Noent => ERRNO_NOENT,
            wasi_filesystem::Errno::Nolck => ERRNO_NOLCK,
            wasi_filesystem::Errno::Nomem => ERRNO_NOMEM,
            wasi_filesystem::Errno::Nospc => ERRNO_NOSPC,
            wasi_filesystem::Errno::Nosys => ERRNO_NOSYS,
            wasi_filesystem::Errno::Notdir => ERRNO_NOTDIR,
            wasi_filesystem::Errno::Notempty => ERRNO_NOTEMPTY,
            wasi_filesystem::Errno::Notrecoverable => ERRNO_NOTRECOVERABLE,
            wasi_filesystem::Errno::Notsup => ERRNO_NOTSUP,
            wasi_filesystem::Errno::Notty => ERRNO_NOTTY,
            wasi_filesystem::Errno::Nxio => ERRNO_NXIO,
            wasi_filesystem::Errno::Overflow => ERRNO_OVERFLOW,
            wasi_filesystem::Errno::Perm => ERRNO_PERM,
            wasi_filesystem::Errno::Pipe => ERRNO_PIPE,
            wasi_filesystem::Errno::Rofs => ERRNO_ROFS,
            wasi_filesystem::Errno::Spipe => ERRNO_SPIPE,
            wasi_filesystem::Errno::Txtbsy => ERRNO_TXTBSY,
            wasi_filesystem::Errno::Xdev => ERRNO_XDEV,
        }
    }
}

impl From<wasi_filesystem::DescriptorType> for wasi::Filetype {
    fn from(ty: wasi_filesystem::DescriptorType) -> wasi::Filetype {
        match ty {
            wasi_filesystem::DescriptorType::RegularFile => FILETYPE_REGULAR_FILE,
            wasi_filesystem::DescriptorType::Directory => FILETYPE_DIRECTORY,
            wasi_filesystem::DescriptorType::BlockDevice => FILETYPE_BLOCK_DEVICE,
            wasi_filesystem::DescriptorType::CharacterDevice => FILETYPE_CHARACTER_DEVICE,
            // preview1 never had a FIFO code.
            wasi_filesystem::DescriptorType::Fifo => FILETYPE_UNKNOWN,
            // TODO: Add a way to disginguish between FILETYPE_SOCKET_STREAM and
            // FILETYPE_SOCKET_DGRAM.
            wasi_filesystem::DescriptorType::Socket => unreachable!(),
            wasi_filesystem::DescriptorType::SymbolicLink => FILETYPE_SYMBOLIC_LINK,
            wasi_filesystem::DescriptorType::Unknown => FILETYPE_UNKNOWN,
        }
    }
}

#[repr(C)]
enum Descriptor {
    /// A closed descriptor, holding a reference to the previous closed
    /// descriptor to support reusing them.
    Closed(Option<Fd>),

    /// Input and/or output wasi-streams, along with stream metadata.
    Streams(Streams),

    /// Writes to `fd_write` will go to the `wasi-stderr` API.
    Stderr,
}

/// Input and/or output wasi-streams, along with a stream type that
/// identifies what kind of stream they are and possibly supporting
/// type-specific operations like seeking.
struct Streams {
    /// The output stream, if present.
    input: Cell<Option<InputStream>>,

    /// The input stream, if present.
    output: Cell<Option<OutputStream>>,

    /// Information about the source of the stream.
    type_: StreamType,
}

impl Streams {
    /// Return the input stream, initializing it on the fly if needed.
    fn get_read_stream(&self) -> Result<InputStream, Errno> {
        match &self.input.get() {
            Some(wasi_stream) => Ok(*wasi_stream),
            None => match &self.type_ {
                // For files, we may have adjusted the position for seeking, so
                // create a new stream.
                StreamType::File(file) => {
                    let input = wasi_filesystem::read_via_stream(file.fd, file.position.get())?;
                    self.input.set(Some(input));
                    Ok(input)
                }
                _ => Err(ERRNO_BADF),
            },
        }
    }

    /// Return the output stream, initializing it on the fly if needed.
    fn get_write_stream(&self) -> Result<OutputStream, Errno> {
        match &self.output.get() {
            Some(wasi_stream) => Ok(*wasi_stream),
            None => match &self.type_ {
                // For files, we may have adjusted the position for seeking, so
                // create a new stream.
                StreamType::File(file) => {
                    let output = if file.append {
                        wasi_filesystem::append_via_stream(file.fd)?
                    } else {
                        wasi_filesystem::write_via_stream(file.fd, file.position.get())?
                    };
                    self.output.set(Some(output));
                    Ok(output)
                }
                _ => Err(ERRNO_BADF),
            },
        }
    }
}

#[allow(dead_code)] // until Socket is implemented
enum StreamType {
    /// It's a valid stream but we don't know where it comes from.
    Unknown,

    /// A stdin source containing no bytes.
    EmptyStdin,

    /// Streaming data with a file.
    File(File),

    /// Streaming data with a socket connection.
    Socket(wasi_tcp::Connection),
}

impl Drop for Descriptor {
    fn drop(&mut self) {
        match self {
            Descriptor::Streams(stream) => {
                if let Some(input) = stream.input.get() {
                    wasi_io::drop_input_stream(input);
                }
                if let Some(output) = stream.output.get() {
                    wasi_io::drop_output_stream(output);
                }
                match &stream.type_ {
                    StreamType::File(file) => wasi_filesystem::close(file.fd),
                    StreamType::Socket(_) => unreachable!(),
                    StreamType::EmptyStdin | StreamType::Unknown => {}
                }
            }
            Descriptor::Stderr => {}
            Descriptor::Closed(_) => {}
        }
    }
}

#[repr(C)]
struct File {
    /// The handle to the preview2 descriptor that this file is referencing.
    fd: wasi_filesystem::Descriptor,

    /// The current-position pointer.
    position: Cell<wasi_filesystem::Filesize>,

    /// In append mode, all writes append to the file.
    append: bool,
}

const PAGE_SIZE: usize = 65536;

/// The maximum path length. WASI doesn't explicitly guarantee this, but all
/// popular OS's have a `PATH_MAX` of at most 4096, so that's enough for this
/// polyfill.
const PATH_MAX: usize = 4096;

const MAX_DESCRIPTORS: usize = 128;

/// Maximum number of bytes to cache for a `wasi::Dirent` plus its path name.
const DIRENT_CACHE: usize = 256;

/// A canary value to detect memory corruption within `State`.
const MAGIC: u32 = u32::from_le_bytes(*b"ugh!");

#[repr(C)] // used for now to keep magic1 and magic2 at the start and end
struct State {
    /// A canary constant value located at the beginning of this structure to
    /// try to catch memory corruption coming from the bottom.
    magic1: u32,

    /// Used by `register_buffer` to coordinate allocations with
    /// `cabi_import_realloc`.
    buffer_ptr: Cell<*mut u8>,
    buffer_len: Cell<usize>,

    /// Storage of mapping from preview1 file descriptors to preview2 file
    /// descriptors.
    ndescriptors: u16,
    descriptors: MaybeUninit<[Descriptor; MAX_DESCRIPTORS]>,

    /// Points to the head of a free-list of closed file descriptors.
    closed: Option<Fd>,

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

    /// Environment variables passed to the `command` entrypoint
    env_vars: Option<&'static [StrTuple]>,

    /// Preopened directories passed to the `command` entrypoint
    preopens: Option<&'static [Preopen]>,

    /// Cache for the `fd_readdir` call for a final `wasi::Dirent` plus path
    /// name that didn't fit into the caller's buffer.
    dirent_cache: DirentCache,

    /// The clock handle for `CLOCKID_MONOTONIC`.
    default_monotonic_clock: Cell<Option<Fd>>,

    /// The clock handle for `CLOCKID_REALTIME`.
    default_wall_clock: Cell<Option<Fd>>,

    /// Another canary constant located at the end of the structure to catch
    /// memory corruption coming from the bottom.
    magic2: u32,
}

struct DirentCache {
    stream: Cell<Option<DirEntryStream>>,
    for_fd: Cell<wasi::Fd>,
    cookie: Cell<wasi::Dircookie>,
    cached_dirent: Cell<wasi::Dirent>,
    path_data: UnsafeCell<MaybeUninit<[u8; DIRENT_CACHE]>>,
}

struct DirEntryStream(wasi_filesystem::DirEntryStream);

impl Drop for DirEntryStream {
    fn drop(&mut self) {
        wasi_filesystem::close_dir_entry_stream(self.0);
    }
}

#[repr(C)]
pub struct WasmStr {
    ptr: *const u8,
    len: usize,
}

#[repr(C)]
pub struct StrTuple {
    key: WasmStr,
    value: WasmStr,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct StrTupleList {
    base: *const StrTuple,
    len: usize,
}

#[repr(C)]
pub struct Preopen {
    descriptor: u32,
    path: WasmStr,
}

#[repr(C)]
pub struct PreopenList {
    base: *const Preopen,
    len: usize,
}

const fn command_data_size() -> usize {
    // The total size of the struct should be a page, so start there
    let mut start = PAGE_SIZE;

    // Remove the big chunks of the struct, the `path_buf` and `descriptors`
    // fields.
    start -= PATH_MAX;
    start -= size_of::<Descriptor>() * MAX_DESCRIPTORS;
    start -= size_of::<DirentCache>();

    // Remove miscellaneous metadata also stored in state.
    start -= 21 * size_of::<usize>();

    // Everything else is the `command_data` allocation.
    start
}

// Statically assert that the `State` structure is the size of a wasm page. This
// mostly guarantees that it's not larger than one page which is relied upon
// below.
const _: () = {
    let _size_assert: [(); PAGE_SIZE] = [(); size_of::<RefCell<State>>()];
};

#[allow(improper_ctypes)]
extern "C" {
    fn get_global_ptr() -> *const RefCell<State>;
    fn set_global_ptr(a: *const RefCell<State>);
}

impl State {
    fn with(f: impl FnOnce(&State) -> Result<(), Errno>) -> Errno {
        let ptr = State::ptr();
        let ptr = ptr.try_borrow().unwrap_or_else(|_| unreachable!());
        assert_eq!(ptr.magic1, MAGIC);
        assert_eq!(ptr.magic2, MAGIC);
        let ret = f(&*ptr);
        match ret {
            Ok(()) => ERRNO_SUCCESS,
            Err(err) => err,
        }
    }

    fn with_mut(f: impl FnOnce(&mut State) -> Result<(), Errno>) -> Errno {
        let ptr = State::ptr();
        let mut ptr = ptr.try_borrow_mut().unwrap_or_else(|_| unreachable!());
        assert_eq!(ptr.magic1, MAGIC);
        assert_eq!(ptr.magic2, MAGIC);
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
        #[link(wasm_import_module = "__main_module__")]
        extern "C" {
            fn cabi_realloc(
                old_ptr: *mut u8,
                old_len: usize,
                align: usize,
                new_len: usize,
            ) -> *mut u8;
        }

        let ret = unsafe {
            cabi_realloc(
                ptr::null_mut(),
                0,
                mem::align_of::<RefCell<State>>(),
                mem::size_of::<RefCell<State>>(),
            ) as *mut RefCell<State>
        };

        let ret = unsafe {
            ret.write(RefCell::new(State {
                magic1: MAGIC,
                magic2: MAGIC,
                buffer_ptr: Cell::new(null_mut()),
                buffer_len: Cell::new(0),
                ndescriptors: 0,
                closed: None,
                descriptors: MaybeUninit::uninit(),
                path_buf: UnsafeCell::new(MaybeUninit::uninit()),
                command_data: MaybeUninit::uninit(),
                command_data_next: 0,
                args: None,
                env_vars: None,
                preopens: None,
                dirent_cache: DirentCache {
                    stream: Cell::new(None),
                    for_fd: Cell::new(0),
                    cookie: Cell::new(wasi::DIRCOOKIE_START),
                    cached_dirent: Cell::new(wasi::Dirent {
                        d_next: 0,
                        d_ino: 0,
                        d_type: FILETYPE_UNKNOWN,
                        d_namlen: 0,
                    }),
                    path_data: UnsafeCell::new(MaybeUninit::uninit()),
                },
                default_monotonic_clock: Cell::new(None),
                default_wall_clock: Cell::new(None),
            }));
            &*ret
        };
        ret.try_borrow_mut()
            .unwrap_or_else(|_| unreachable!())
            .init();
        ret
    }

    fn init(&mut self) {
        // Set up a default stdin. This will be overridden when `command`
        // is called.
        unwrap_result(self.push_desc(Descriptor::Streams(Streams {
            input: Cell::new(None),
            output: Cell::new(None),
            type_: StreamType::Unknown,
        })));
        // Set up a default stdout, writing to the stderr device. This will
        // be overridden when `command` is called.
        unwrap_result(self.push_desc(Descriptor::Stderr));
        // Set up a default stderr.
        unwrap_result(self.push_desc(Descriptor::Stderr));
    }

    fn push_desc(&mut self, desc: Descriptor) -> Result<Fd, Errno> {
        unsafe {
            let descriptors = self.descriptors.as_mut_ptr();
            let ndescriptors = unwrap_result(usize::try_from(self.ndescriptors));
            if ndescriptors >= (*descriptors).len() {
                return Err(ERRNO_NOMEM);
            }
            ptr::addr_of_mut!((*descriptors)[ndescriptors]).write(desc);
            self.ndescriptors += 1;
            Ok(Fd::from(self.ndescriptors - 1))
        }
    }

    fn descriptors(&self) -> &[Descriptor] {
        unsafe {
            slice::from_raw_parts(
                self.descriptors.as_ptr().cast(),
                unwrap_result(usize::try_from(self.ndescriptors)),
            )
        }
    }

    fn descriptors_mut(&mut self) -> &mut [Descriptor] {
        unsafe {
            slice::from_raw_parts_mut(
                self.descriptors.as_mut_ptr().cast(),
                unwrap_result(usize::try_from(self.ndescriptors)),
            )
        }
    }

    fn get(&self, fd: Fd) -> Result<&Descriptor, Errno> {
        self.descriptors()
            .get(unwrap_result(usize::try_from(fd)))
            .ok_or(ERRNO_BADF)
    }

    fn get_mut(&mut self, fd: Fd) -> Result<&mut Descriptor, Errno> {
        self.descriptors_mut()
            .get_mut(unwrap_result(usize::try_from(fd)))
            .ok_or(ERRNO_BADF)
    }

    fn get_stream_with_error(&self, fd: Fd, error: Errno) -> Result<&Streams, Errno> {
        match self.get(fd)? {
            Descriptor::Streams(streams) => Ok(streams),
            Descriptor::Closed(_) => Err(ERRNO_BADF),
            _ => Err(error),
        }
    }

    fn get_file_with_error(&self, fd: Fd, error: Errno) -> Result<&File, Errno> {
        match self.get(fd)? {
            Descriptor::Streams(Streams {
                type_: StreamType::File(file),
                ..
            }) => Ok(file),
            Descriptor::Closed(_) => Err(ERRNO_BADF),
            _ => Err(error),
        }
    }

    #[allow(dead_code)] // until Socket is implemented
    fn get_socket(&self, fd: Fd) -> Result<wasi_tcp::Connection, Errno> {
        match self.get(fd)? {
            Descriptor::Streams(Streams {
                type_: StreamType::Socket(socket),
                ..
            }) => Ok(*socket),
            Descriptor::Closed(_) => Err(ERRNO_BADF),
            _ => Err(ERRNO_INVAL),
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

    fn get_seekable_stream(&self, fd: Fd) -> Result<&Streams, Errno> {
        self.get_stream_with_error(fd, ERRNO_SPIPE)
    }

    fn get_read_stream(&self, fd: Fd) -> Result<InputStream, Errno> {
        match self.get(fd)? {
            Descriptor::Streams(streams) => streams.get_read_stream(),
            Descriptor::Closed(_) | Descriptor::Stderr => Err(ERRNO_BADF),
        }
    }

    fn get_write_stream(&self, fd: Fd) -> Result<OutputStream, Errno> {
        match self.get(fd)? {
            Descriptor::Streams(streams) => streams.get_write_stream(),
            Descriptor::Closed(_) | Descriptor::Stderr => Err(ERRNO_BADF),
        }
    }

    /// Register `buf` and `buf_len` to be used by `cabi_realloc` to satisfy
    /// the next request.
    fn register_buffer(&self, buf: *mut u8, buf_len: usize) {
        self.buffer_ptr.set(buf);
        self.buffer_len.set(buf_len);
    }

    /// Return a handle to the default wall clock, creating one if we
    /// don't already have one.
    fn default_wall_clock(&self) -> Fd {
        match self.default_wall_clock.get() {
            Some(fd) => fd,
            None => self.init_default_wall_clock(),
        }
    }

    fn init_default_wall_clock(&self) -> Fd {
        let clock = wasi_default_clocks::default_wall_clock();
        self.default_wall_clock.set(Some(clock));
        clock
    }

    /// Return a handle to the default monotonic clock, creating one if we
    /// don't already have one.
    fn default_monotonic_clock(&self) -> Fd {
        match self.default_monotonic_clock.get() {
            Some(fd) => fd,
            None => self.init_default_monotonic_clock(),
        }
    }

    fn init_default_monotonic_clock(&self) -> Fd {
        let clock = wasi_default_clocks::default_monotonic_clock();
        self.default_monotonic_clock.set(Some(clock));
        clock
    }
}
