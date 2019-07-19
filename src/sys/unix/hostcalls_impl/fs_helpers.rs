#![allow(non_camel_case_types)]
#![allow(unused_unsafe)]

use crate::ctx::WasiCtx;
use crate::fdentry::Descriptor;
use crate::host;
use crate::sys::errno_from_host;
use crate::sys::host_impl;
use nix::libc::{self, c_long};
use std::fs::File;
use std::path::{Component, Path};

/// Normalizes a path to ensure that the target path is located under the directory provided.
///
/// This is a workaround for not having Capsicum support in the OS.
pub(crate) fn path_get(
    wasi_ctx: &WasiCtx,
    dirfd: host::__wasi_fd_t,
    dirflags: host::__wasi_lookupflags_t,
    path: &str,
    needed_base: host::__wasi_rights_t,
    needed_inheriting: host::__wasi_rights_t,
    needs_final_component: bool,
) -> Result<(File, String), host::__wasi_errno_t> {
    const MAX_SYMLINK_EXPANSIONS: usize = 128;

    if path.contains("\0") {
        // if contains NUL, return EILSEQ
        return Err(host::__WASI_EILSEQ);
    }

    let dirfe = wasi_ctx.get_fd_entry(dirfd, needed_base, needed_inheriting)?;
    let dirfd = match &*dirfe.fd_object.descriptor {
        Descriptor::File(f) => f.try_clone().map_err(|err| {
            err.raw_os_error()
                .map_or(host::__WASI_EBADF, errno_from_host)
        })?,
        _ => return Err(host::__WASI_EBADF),
    };

    // Stack of directory file descriptors. Index 0 always corresponds with the directory provided
    // to this function. Entering a directory causes a file descriptor to be pushed, while handling
    // ".." entries causes an entry to be popped. Index 0 cannot be popped, as this would imply
    // escaping the base directory.
    let mut dir_stack = vec![dirfd];

    // Stack of paths left to process. This is initially the `path` argument to this function, but
    // any symlinks we encounter are processed by pushing them on the stack.
    let mut path_stack = vec![path.to_owned()];

    // Track the number of symlinks we've expanded, so we can return `ELOOP` after too many.
    let mut symlink_expansions = 0;

    // TODO: rewrite this using a custom posix path type, with a component iterator that respects
    // trailing slashes. This version does way too much allocation, and is way too fiddly.
    loop {
        match path_stack.pop() {
            Some(cur_path) => {
                // eprintln!("cur_path = {:?}", cur_path);

                let ends_with_slash = cur_path.ends_with("/");
                let mut components = Path::new(&cur_path).components();
                let head = match components.next() {
                    None => return Err(host::__WASI_ENOENT),
                    Some(p) => p,
                };
                let tail = components.as_path();

                if tail.components().next().is_some() {
                    let mut tail = host_impl::path_from_host(tail.as_os_str())?;
                    if ends_with_slash {
                        tail.push_str("/");
                    }
                    path_stack.push(tail);
                }

                match head {
                    Component::Prefix(_) | Component::RootDir => {
                        // path is absolute!
                        return Err(host::__WASI_ENOTCAPABLE);
                    }
                    Component::CurDir => {
                        // "." so skip
                        continue;
                    }
                    Component::ParentDir => {
                        // ".." so pop a dir
                        let _ = dir_stack.pop().ok_or(host::__WASI_ENOTCAPABLE)?;

                        // we're not allowed to pop past the original directory
                        if dir_stack.is_empty() {
                            return Err(host::__WASI_ENOTCAPABLE);
                        }
                    }
                    Component::Normal(head) => {
                        let mut head = host_impl::path_from_host(head)?;
                        if ends_with_slash {
                            // preserve trailing slash
                            head.push_str("/");
                        }

                        if !path_stack.is_empty() || (ends_with_slash && !needs_final_component) {
                            match openat(dir_stack.last().ok_or(host::__WASI_ENOTCAPABLE)?, &head) {
                                Ok(new_dir) => {
                                    dir_stack.push(new_dir);
                                    continue;
                                }
                                Err(e)
                                    if e == host::__WASI_ELOOP
                                        || e == host::__WASI_EMLINK
                                        || e == host::__WASI_ENOTDIR =>
                                // Check to see if it was a symlink. Linux indicates
                                // this with ENOTDIR because of the O_DIRECTORY flag.
                                {
                                    // attempt symlink expansion
                                    match readlinkat(
                                        dir_stack.last().ok_or(host::__WASI_ENOTCAPABLE)?,
                                        &head,
                                    ) {
                                        Ok(mut link_path) => {
                                            symlink_expansions += 1;
                                            if symlink_expansions > MAX_SYMLINK_EXPANSIONS {
                                                return Err(host::__WASI_ELOOP);
                                            }

                                            if head.ends_with("/") {
                                                link_path.push_str("/");
                                            }

                                            path_stack.push(link_path);
                                            continue;
                                        }
                                        Err(e) => {
                                            return Err(e);
                                        }
                                    }
                                }
                                Err(e) => {
                                    return Err(e);
                                }
                            }
                        } else if ends_with_slash
                            || (dirflags & host::__WASI_LOOKUP_SYMLINK_FOLLOW) != 0
                        {
                            // if there's a trailing slash, or if `LOOKUP_SYMLINK_FOLLOW` is set, attempt
                            // symlink expansion
                            match readlinkat(
                                dir_stack.last().ok_or(host::__WASI_ENOTCAPABLE)?,
                                &head,
                            ) {
                                Ok(mut link_path) => {
                                    symlink_expansions += 1;
                                    if symlink_expansions > MAX_SYMLINK_EXPANSIONS {
                                        return Err(host::__WASI_ELOOP);
                                    }

                                    if head.ends_with("/") {
                                        link_path.push_str("/");
                                    }

                                    path_stack.push(link_path);
                                    continue;
                                }
                                Err(e) => {
                                    if e != host::__WASI_EINVAL && e != host::__WASI_ENOENT {
                                        return Err(e);
                                    }
                                }
                            }
                        }

                        // not a symlink, so we're done;
                        return Ok((dir_stack.pop().ok_or(host::__WASI_ENOTCAPABLE)?, head));
                    }
                }
            }
            None => {
                // no further components to process. means we've hit a case like "." or "a/..", or if the
                // input path has trailing slashes and `needs_final_component` is not set
                return Ok((
                    dir_stack.pop().ok_or(host::__WASI_ENOTCAPABLE)?,
                    String::from("."),
                ));
            }
        }
    }
}

fn openat(dirfd: &File, path: &str) -> Result<File, host::__wasi_errno_t> {
    use nix::fcntl::{self, OFlag};
    use nix::sys::stat::Mode;
    use std::os::unix::prelude::{AsRawFd, FromRawFd};

    fcntl::openat(
        dirfd.as_raw_fd(),
        path,
        OFlag::O_RDONLY | OFlag::O_DIRECTORY | OFlag::O_NOFOLLOW,
        Mode::empty(),
    )
    .map(|new_fd| unsafe { File::from_raw_fd(new_fd) })
    .map_err(|e| host_impl::errno_from_nix(e.as_errno().unwrap()))
}

fn readlinkat(dirfd: &File, path: &str) -> Result<String, host::__wasi_errno_t> {
    use nix::fcntl;
    use std::os::unix::prelude::AsRawFd;

    let readlink_buf = &mut [0u8; libc::PATH_MAX as usize + 1];

    fcntl::readlinkat(dirfd.as_raw_fd(), path, readlink_buf)
        .map_err(|e| host_impl::errno_from_nix(e.as_errno().unwrap()))
        .and_then(host_impl::path_from_host)
}

#[cfg(not(target_os = "macos"))]
pub fn utime_now() -> c_long {
    libc::UTIME_NOW
}

#[cfg(target_os = "macos")]
pub fn utime_now() -> c_long {
    -1
}

#[cfg(not(target_os = "macos"))]
pub fn utime_omit() -> c_long {
    libc::UTIME_OMIT
}

#[cfg(target_os = "macos")]
pub fn utime_omit() -> c_long {
    -2
}
