#![allow(non_camel_case_types)]
#![allow(unused_unsafe)]

use crate::sys::errno_from_host;
use crate::sys::host_impl;
use crate::{host, Result};
use std::fs::File;
use std::os::windows::prelude::{AsRawHandle, FromRawHandle};
use std::path::{Component, Path};

/// Normalizes a path to ensure that the target path is located under the directory provided.
pub(crate) fn path_get(
    dirfd: &File,
    _dirflags: host::__wasi_lookupflags_t,
    path: &str,
    needs_final_component: bool,
) -> Result<(File, String)> {
    if path.contains("\0") {
        // if contains NUL, return EILSEQ
        return Err(host::__WASI_EILSEQ);
    }

    let dirfd = dirfd.try_clone().map_err(|err| {
        err.raw_os_error()
            .map_or(host::__WASI_EBADF, errno_from_host)
    })?;

    // Stack of directory handles. Index 0 always corresponds with the directory provided
    // to this function. Entering a directory causes a handle to be pushed, while handling
    // ".." entries causes an entry to be popped. Index 0 cannot be popped, as this would imply
    // escaping the base directory.
    let mut dir_stack = vec![dirfd];

    // Stack of paths left to process. This is initially the `path` argument to this function, but
    // any symlinks we encounter are processed by pushing them on the stack.
    let mut path_stack = vec![path.to_owned()];

    loop {
        match path_stack.pop() {
            Some(cur_path) => {
                // dbg!(&cur_path);
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
                        // should the component be a directory? it should if there is more path left to process, or
                        // if it has a trailing slash and `needs_final_component` is not set
                        if !path_stack.is_empty() || (ends_with_slash && !needs_final_component) {
                            match winx::file::openat(
                                dir_stack
                                    .last()
                                    .ok_or(host::__WASI_ENOTCAPABLE)?
                                    .as_raw_handle(),
                                head.as_str(),
                                winx::file::AccessRight::FILE_GENERIC_READ,
                                winx::file::CreationDisposition::OPEN_EXISTING,
                                winx::file::FlagsAndAttributes::FILE_FLAG_BACKUP_SEMANTICS,
                            ) {
                                Ok(new_dir) => {
                                    dir_stack.push(unsafe { File::from_raw_handle(new_dir) });
                                    continue;
                                }
                                Err(e) => {
                                    return Err(host_impl::errno_from_win(e));
                                }
                            }
                        } else {
                            // we're done
                            return Ok((dir_stack.pop().ok_or(host::__WASI_ENOTCAPABLE)?, head));
                        }
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
