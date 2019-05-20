#![allow(non_camel_case_types)]
#![allow(unused_unsafe)]

use super::host_impl;
use crate::ctx::WasiCtx;
use crate::host;

use nix::libc::{self, c_long};
use std::ffi::{OsStr, OsString};
use std::os::unix::prelude::{OsStrExt, OsStringExt, RawFd};

/// Normalizes a path to ensure that the target path is located under the directory provided.
///
/// This is a workaround for not having Capsicum support in the OS.
pub fn path_get<P: AsRef<OsStr>>(
    wasi_ctx: &WasiCtx,
    dirfd: host::__wasi_fd_t,
    dirflags: host::__wasi_lookupflags_t,
    path: P,
    needed_base: host::__wasi_rights_t,
    needed_inheriting: host::__wasi_rights_t,
    needs_final_component: bool,
) -> Result<(RawFd, OsString), host::__wasi_errno_t> {
    use nix::errno::Errno;
    use nix::fcntl::{openat, readlinkat, OFlag};
    use nix::sys::stat::Mode;

    const MAX_SYMLINK_EXPANSIONS: usize = 128;

    /// close all the intermediate file descriptors, but make sure not to drop either the original
    /// dirfd or the one we return (which may be the same dirfd)
    fn ret_dir_success(dir_stack: &mut Vec<RawFd>) -> RawFd {
        let ret_dir = dir_stack.pop().expect("there is always a dirfd to return");
        if let Some(dirfds) = dir_stack.get(1..) {
            for dirfd in dirfds {
                nix::unistd::close(*dirfd).unwrap_or_else(|e| {
                    dbg!(e);
                });
            }
        }
        ret_dir
    }

    /// close all file descriptors other than the base directory, and return the errno for
    /// convenience with `return`
    fn ret_error(
        dir_stack: &mut Vec<RawFd>,
        errno: host::__wasi_errno_t,
    ) -> Result<(RawFd, OsString), host::__wasi_errno_t> {
        if let Some(dirfds) = dir_stack.get(1..) {
            for dirfd in dirfds {
                nix::unistd::close(*dirfd).unwrap_or_else(|e| {
                    dbg!(e);
                });
            }
        }
        Err(errno)
    }

    let dirfe = wasi_ctx.get_fd_entry(dirfd, needed_base, needed_inheriting)?;

    // Stack of directory file descriptors. Index 0 always corresponds with the directory provided
    // to this function. Entering a directory causes a file descriptor to be pushed, while handling
    // ".." entries causes an entry to be popped. Index 0 cannot be popped, as this would imply
    // escaping the base directory.
    let mut dir_stack = vec![dirfe.fd_object.rawfd];

    // Stack of paths left to process. This is initially the `path` argument to this function, but
    // any symlinks we encounter are processed by pushing them on the stack.
    let mut path_stack = vec![path.as_ref().to_owned().into_vec()];

    // Track the number of symlinks we've expanded, so we can return `ELOOP` after too many.
    let mut symlink_expansions = 0;

    // Buffer to read links into; defined outside of the loop so we don't reallocate it constantly.
    let mut readlink_buf = vec![0u8; libc::PATH_MAX as usize + 1];

    // TODO: rewrite this using a custom posix path type, with a component iterator that respects
    // trailing slashes. This version does way too much allocation, and is way too fiddly.
    loop {
        let component = if let Some(cur_path) = path_stack.pop() {
            // eprintln!(
            //     "cur_path = {:?}",
            //     std::str::from_utf8(cur_path.as_slice()).unwrap()
            // );
            let mut split = cur_path.splitn(2, |&c| c == '/' as u8);
            let head = split.next();
            let tail = split.next();
            match (head, tail) {
                (None, _) => {
                    // split always returns at least a singleton iterator with an empty slice
                    panic!("unreachable");
                }
                // path is empty
                (Some([]), None) => {
                    return ret_error(&mut dir_stack, host::__WASI_ENOENT);
                }
                // path starts with `/`, is absolute
                (Some([]), Some(_)) => {
                    return ret_error(&mut dir_stack, host::__WASI_ENOTCAPABLE);
                }
                // the final component of the path with no trailing slash
                (Some(component), None) => component.to_vec(),
                (Some(component), Some(rest)) => {
                    if rest.iter().all(|&c| c == '/' as u8) {
                        // the final component of the path with trailing slashes; put one trailing
                        // slash back on
                        let mut component = component.to_vec();
                        component.push('/' as u8);
                        component
                    } else {
                        // non-final component; push the rest back on the stack
                        path_stack.push(rest.to_vec());
                        component.to_vec()
                    }
                }
            }
        } else {
            // if the path stack is ever empty, we return rather than going through the loop again
            panic!("unreachable");
        };

        // eprintln!(
        //     "component = {:?}",
        //     std::str::from_utf8(component.as_slice()).unwrap()
        // );

        match component.as_slice() {
            b"." => {
                // skip component
            }
            b".." => {
                // pop a directory
                let dirfd = dir_stack.pop().expect("dir_stack is never empty");

                // we're not allowed to pop past the original directory
                if dir_stack.is_empty() {
                    return ret_error(&mut dir_stack, host::__WASI_ENOTCAPABLE);
                } else {
                    nix::unistd::close(dirfd).unwrap_or_else(|e| {
                        dbg!(e);
                    });
                }
            }
            // should the component be a directory? it should if there is more path left to process, or
            // if it has a trailing slash and `needs_final_component` is not set
            component
                if !path_stack.is_empty()
                    || (component.ends_with(b"/") && !needs_final_component) =>
            {
                match openat(
                    *dir_stack.first().expect("dir_stack is never empty"),
                    component,
                    OFlag::O_RDONLY | OFlag::O_DIRECTORY | OFlag::O_NOFOLLOW,
                    Mode::empty(),
                ) {
                    Ok(new_dir) => {
                        dir_stack.push(new_dir);
                        continue;
                    }
                    Err(e)
                        // Check to see if it was a symlink. Linux indicates
                        // this with ENOTDIR because of the O_DIRECTORY flag.
                        if e.as_errno() == Some(Errno::ELOOP)
                            || e.as_errno() == Some(Errno::EMLINK)
                            || e.as_errno() == Some(Errno::ENOTDIR) =>
                    {
                        // attempt symlink expansion
                        match readlinkat(
                            *dir_stack.last().expect("dir_stack is never empty"),
                            component,
                            readlink_buf.as_mut_slice(),
                        ) {
                            Ok(link_path) => {
                                symlink_expansions += 1;
                                if symlink_expansions > MAX_SYMLINK_EXPANSIONS {
                                    return ret_error(&mut dir_stack, host::__WASI_ELOOP);
                                }

                                let mut link_path = link_path.as_bytes().to_vec();

                                // append a trailing slash if the component leading to it has one, so
                                // that we preserve any ENOTDIR that might come from trying to open a
                                // non-directory
                                if component.ends_with(b"/") {
                                    link_path.push('/' as u8);
                                }

                                path_stack.push(link_path);
                                continue;
                            }
                            Err(e) => {
                                return ret_error(
                                    &mut dir_stack,
                                    host_impl::errno_from_nix(e.as_errno().unwrap()),
                                );
                            }
                        }
                    }
                    Err(e) => {
                        return ret_error(
                            &mut dir_stack,
                            host_impl::errno_from_nix(e.as_errno().unwrap()),
                        );
                    }
                }
            }
            // the final component
            component => {
                // if there's a trailing slash, or if `LOOKUP_SYMLINK_FOLLOW` is set, attempt
                // symlink expansion
                if component.ends_with(b"/") || (dirflags & host::__WASI_LOOKUP_SYMLINK_FOLLOW) != 0
                {
                    match readlinkat(
                        *dir_stack.last().expect("dir_stack is never empty"),
                        component,
                        readlink_buf.as_mut_slice(),
                    ) {
                        Ok(link_path) => {
                            symlink_expansions += 1;
                            if symlink_expansions > MAX_SYMLINK_EXPANSIONS {
                                return ret_error(&mut dir_stack, host::__WASI_ELOOP);
                            }

                            let mut link_path = link_path.as_bytes().to_vec();

                            // append a trailing slash if the component leading to it has one, so
                            // that we preserve any ENOTDIR that might come from trying to open a
                            // non-directory
                            if component.ends_with(b"/") {
                                link_path.push('/' as u8);
                            }

                            path_stack.push(link_path);
                            continue;
                        }
                        Err(e) => {
                            let errno = e.as_errno().unwrap();
                            if errno != Errno::EINVAL && errno != Errno::ENOENT {
                                // only return an error if this path is not actually a symlink
                                return ret_error(&mut dir_stack, host_impl::errno_from_nix(errno));
                            }
                        }
                    }
                }

                // not a symlink, so we're done;
                return Ok((
                    ret_dir_success(&mut dir_stack),
                    OsStr::from_bytes(component).to_os_string(),
                ));
            }
        }

        if path_stack.is_empty() {
            // no further components to process. means we've hit a case like "." or "a/..", or if the
            // input path has trailing slashes and `needs_final_component` is not set
            return Ok((
                ret_dir_success(&mut dir_stack),
                OsStr::new(".").to_os_string(),
            ));
        } else {
            continue;
        }
    }
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
