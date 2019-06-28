#![allow(non_camel_case_types)]
#![allow(unused_unsafe)]

use super::host_impl;
use crate::ctx::WasiCtx;
use crate::host;

use std::ffi::{OsStr, OsString};
use std::os::windows::prelude::RawHandle;
use std::path::{Component, Path, PathBuf};

/// Normalizes a path to ensure that the target path is located under the directory provided.
pub fn path_get<P: AsRef<OsStr>>(
    wasi_ctx: &WasiCtx,
    dirfd: host::__wasi_fd_t,
    _dirflags: host::__wasi_lookupflags_t,
    path: P,
    needed_base: host::__wasi_rights_t,
    needed_inheriting: host::__wasi_rights_t,
    needs_final_component: bool,
) -> Result<(RawHandle, OsString), host::__wasi_errno_t> {
    /// close all the intermediate handles, but make sure not to drop either the original
    /// dirfd or the one we return (which may be the same dirfd)
    fn ret_dir_success(dir_stack: &mut Vec<RawHandle>) -> RawHandle {
        let ret_dir = dir_stack.pop().expect("there is always a dirfd to return");
        if let Some(dirfds) = dir_stack.get(1..) {
            for dirfd in dirfds {
                winx::handle::close(*dirfd).unwrap_or_else(|e| {
                    dbg!(e);
                });
            }
        }
        ret_dir
    }

    /// close all file descriptors other than the base directory, and return the errno for
    /// convenience with `return`
    fn ret_error(
        dir_stack: &mut Vec<RawHandle>,
        errno: host::__wasi_errno_t,
    ) -> Result<(RawHandle, OsString), host::__wasi_errno_t> {
        if let Some(dirfds) = dir_stack.get(1..) {
            for dirfd in dirfds {
                winx::handle::close(*dirfd).unwrap_or_else(|e| {
                    dbg!(e);
                });
            }
        }
        Err(errno)
    }

    let dirfe = wasi_ctx.get_fd_entry(dirfd, needed_base, needed_inheriting)?;

    // Stack of directory handles. Index 0 always corresponds with the directory provided
    // to this function. Entering a directory causes a handle to be pushed, while handling
    // ".." entries causes an entry to be popped. Index 0 cannot be popped, as this would imply
    // escaping the base directory.
    let mut dir_stack = vec![dirfe.fd_object.raw_handle];

    // Stack of paths left to process. This is initially the `path` argument to this function, but
    // any symlinks we encounter are processed by pushing them on the stack.
    let mut path_stack = vec![PathBuf::from(path.as_ref())];

    loop {
        match path_stack.pop() {
            Some(cur_path) => {
                // dbg!(&cur_path);
                let mut components = cur_path.components();
                let head = match components.next() {
                    None => return ret_error(&mut dir_stack, host::__WASI_ENOENT),
                    Some(p) => p,
                };
                let tail = components.as_path();

                if tail.components().next().is_some() {
                    path_stack.push(PathBuf::from(tail));
                }

                match head {
                    Component::Prefix(_) | Component::RootDir => {
                        // path is absolute!
                        return ret_error(&mut dir_stack, host::__WASI_ENOTCAPABLE);
                    }
                    Component::CurDir => {
                        // "." so skip
                        continue;
                    }
                    Component::ParentDir => {
                        // ".." so pop a dir
                        let dirfd = dir_stack.pop().expect("dir_stack is never empty");

                        // we're not allowed to pop past the original directory
                        if dir_stack.is_empty() {
                            return ret_error(&mut dir_stack, host::__WASI_ENOTCAPABLE);
                        } else {
                            winx::handle::close(dirfd).unwrap_or_else(|e| {
                                dbg!(e);
                            });
                        }
                    }
                    Component::Normal(head) => {
                        // should the component be a directory? it should if there is more path left to process, or
                        // if it has a trailing slash and `needs_final_component` is not set
                        if !path_stack.is_empty()
                            || (Path::new(head).is_dir() && !needs_final_component)
                        {
                            match winx::file::openat(
                                *dir_stack.last().expect("dir_stack is never empty"),
                                head,
                                winx::file::AccessRight::FILE_GENERIC_READ,
                                winx::file::CreationDisposition::OPEN_EXISTING,
                                winx::file::FlagsAndAttributes::FILE_FLAG_BACKUP_SEMANTICS,
                            ) {
                                Ok(new_dir) => {
                                    dir_stack.push(new_dir);
                                    continue;
                                }
                                Err(e) => {
                                    return ret_error(&mut dir_stack, host_impl::errno_from_win(e));
                                }
                            }
                        } else {
                            // we're done
                            return Ok((ret_dir_success(&mut dir_stack), head.to_os_string()));
                        }
                    }
                }
            }
            None => {
                // no further components to process. means we've hit a case like "." or "a/..", or if the
                // input path has trailing slashes and `needs_final_component` is not set
                return Ok((
                    ret_dir_success(&mut dir_stack),
                    OsStr::new(".").to_os_string(),
                ));
            }
        }
    }
}
