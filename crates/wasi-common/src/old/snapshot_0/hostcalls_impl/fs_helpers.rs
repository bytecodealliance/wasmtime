#![allow(non_camel_case_types)]
use crate::old::snapshot_0::sys::hostcalls_impl::fs_helpers::*;
use crate::old::snapshot_0::{error::WasiError, fdentry::FdEntry, wasi, Error, Result};
use crate::sys::{
    hoststring_ends_with_slash, hoststring_from_osstring, osstr_ends_with_slash, HostString,
};
use std::ffi::OsStr;
use std::fs::File;
use std::path::{Component, Path};

#[derive(Debug)]
pub(crate) struct PathGet {
    dirfd: File,
    path: HostString,
}

impl PathGet {
    pub(crate) fn dirfd(&self) -> &File {
        &self.dirfd
    }

    pub(crate) fn path(&self) -> &HostString {
        &self.path
    }
}

/// Normalizes a path to ensure that the target path is located under the directory provided.
///
/// This is a workaround for not having Capsicum support in the OS.
pub(crate) fn path_get(
    fe: &FdEntry,
    rights_base: wasi::__wasi_rights_t,
    rights_inheriting: wasi::__wasi_rights_t,
    dirflags: wasi::__wasi_lookupflags_t,
    path: &str,
    needs_final_component: bool,
) -> Result<PathGet> {
    const MAX_SYMLINK_EXPANSIONS: usize = 128;

    if path.contains('\0') {
        // if contains NUL, return EILSEQ
        return Err(Error::EILSEQ);
    }

    if fe.file_type != wasi::__WASI_FILETYPE_DIRECTORY {
        // if `dirfd` doesn't refer to a directory, return `ENOTDIR`.
        return Err(Error::ENOTDIR);
    }

    let dirfd = fe
        .as_descriptor(rights_base, rights_inheriting)?
        .as_file()?
        .try_clone()?;

    // Stack of directory file descriptors. Index 0 always corresponds with the directory provided
    // to this function. Entering a directory causes a file descriptor to be pushed, while handling
    // ".." entries causes an entry to be popped. Index 0 cannot be popped, as this would imply
    // escaping the base directory.
    let mut dir_stack = vec![dirfd];

    // Stack of paths left to process. This is initially the `path` argument to this function, but
    // any symlinks we encounter are processed by pushing them on the stack.
    let mut path_stack = vec![OsStr::new(path).to_os_string()];

    // Track the number of symlinks we've expanded, so we can return `ELOOP` after too many.
    let mut symlink_expansions = 0;

    // TODO: rewrite this using a custom posix path type, with a component iterator that respects
    // trailing slashes. This version does way too much allocation, and is way too fiddly.
    loop {
        match path_stack.pop() {
            Some(cur_path) => {
                log::debug!("path_get cur_path = {:?}", cur_path);

                let ends_with_slash = osstr_ends_with_slash(&cur_path);
                let mut components = Path::new(&cur_path).components();
                let head = match components.next() {
                    None => return Err(Error::ENOENT),
                    Some(p) => p,
                };
                let tail = components.as_path();

                if tail.components().next().is_some() {
                    let mut tail = tail.as_os_str().to_os_string();
                    if ends_with_slash {
                        tail.push("/");
                    }
                    path_stack.push(tail);
                }

                log::debug!("path_get path_stack = {:?}", path_stack);

                match head {
                    Component::Prefix(_) | Component::RootDir => {
                        // path is absolute!
                        return Err(Error::ENOTCAPABLE);
                    }
                    Component::CurDir => {
                        // "." so skip
                    }
                    Component::ParentDir => {
                        // ".." so pop a dir
                        let _ = dir_stack.pop().ok_or(Error::ENOTCAPABLE)?;

                        // we're not allowed to pop past the original directory
                        if dir_stack.is_empty() {
                            return Err(Error::ENOTCAPABLE);
                        }
                    }
                    Component::Normal(head) => {
                        let mut head = head.to_os_string();
                        if ends_with_slash {
                            // preserve trailing slash
                            head.push("/");
                        }
                        let head = hoststring_from_osstring(head);

                        if !path_stack.is_empty() || (ends_with_slash && !needs_final_component) {
                            match openat(dir_stack.last().ok_or(Error::ENOTCAPABLE)?, head.as_ref())
                            {
                                Ok(new_dir) => {
                                    dir_stack.push(new_dir);
                                }
                                Err(e) => {
                                    match e.as_wasi_error() {
                                        WasiError::ELOOP
                                        | WasiError::EMLINK
                                        | WasiError::ENOTDIR =>
                                        // Check to see if it was a symlink. Linux indicates
                                        // this with ENOTDIR because of the O_DIRECTORY flag.
                                        {
                                            // attempt symlink expansion
                                            let mut link_path = readlinkat(
                                                dir_stack.last().ok_or(Error::ENOTCAPABLE)?,
                                                head.as_ref(),
                                            )?;

                                            symlink_expansions += 1;
                                            if symlink_expansions > MAX_SYMLINK_EXPANSIONS {
                                                return Err(Error::ELOOP);
                                            }

                                            if hoststring_ends_with_slash(&head) {
                                                link_path.push("/");
                                            }

                                            log::debug!(
                                                "attempted symlink expansion link_path={:?}",
                                                link_path
                                            );

                                            path_stack.push(link_path);
                                        }
                                        _ => {
                                            return Err(e);
                                        }
                                    }
                                }
                            }

                            continue;
                        } else if ends_with_slash
                            || (dirflags & wasi::__WASI_LOOKUPFLAGS_SYMLINK_FOLLOW) != 0
                        {
                            // if there's a trailing slash, or if `LOOKUP_SYMLINK_FOLLOW` is set, attempt
                            // symlink expansion
                            match readlinkat(
                                dir_stack.last().ok_or(Error::ENOTCAPABLE)?,
                                head.as_ref(),
                            ) {
                                Ok(mut link_path) => {
                                    symlink_expansions += 1;
                                    if symlink_expansions > MAX_SYMLINK_EXPANSIONS {
                                        return Err(Error::ELOOP);
                                    }

                                    if hoststring_ends_with_slash(&head) {
                                        link_path.push("/");
                                    }

                                    log::debug!(
                                        "attempted symlink expansion link_path={:?}",
                                        link_path
                                    );

                                    path_stack.push(link_path);
                                    continue;
                                }
                                Err(e) => {
                                    if e.as_wasi_error() != WasiError::EINVAL
                                        && e.as_wasi_error() != WasiError::ENOENT
                                        // this handles the cases when trying to link to
                                        // a destination that already exists, and the target
                                        // path contains a slash
                                        && e.as_wasi_error() != WasiError::ENOTDIR
                                    {
                                        return Err(e);
                                    }
                                }
                            }
                        }

                        // not a symlink, so we're done;
                        return Ok(PathGet {
                            dirfd: dir_stack.pop().ok_or(Error::ENOTCAPABLE)?,
                            path: head,
                        });
                    }
                }
            }
            None => {
                // no further components to process. means we've hit a case like "." or "a/..", or if the
                // input path has trailing slashes and `needs_final_component` is not set
                return Ok(PathGet {
                    dirfd: dir_stack.pop().ok_or(Error::ENOTCAPABLE)?,
                    path: hoststring_from_osstring(OsStr::new(".").to_os_string()),
                });
            }
        }
    }
}
