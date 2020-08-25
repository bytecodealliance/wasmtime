use crate::entry::Entry;
use crate::handle::{Handle, HandleRights};
use crate::wasi::types;
use crate::{Error, Result};
use std::path::{Component, Path};
use std::str;
use wiggle::GuestPtr;

pub(crate) use crate::sys::path::{from_host, open_rights};

/// Normalizes a path to ensure that the target path is located under the directory provided.
///
/// This is a workaround for not having Capsicum support in the OS.
pub(crate) fn get(
    entry: &Entry,
    required_rights: &HandleRights,
    dirflags: types::Lookupflags,
    path_ptr: &GuestPtr<'_, str>,
    needs_final_component: bool,
) -> Result<(Box<dyn Handle>, String)> {
    const MAX_SYMLINK_EXPANSIONS: usize = 128;

    // Extract path as &str from guest's memory.
    let path = path_ptr.as_str()?;

    tracing::trace!(path = &*path);

    if path.contains('\0') {
        // if contains NUL, return Ilseq
        return Err(Error::Ilseq);
    }

    if entry.get_file_type() != types::Filetype::Directory {
        // if `dirfd` doesn't refer to a directory, return `Notdir`.
        return Err(Error::Notdir);
    }

    let handle = entry.as_handle(required_rights)?;
    let dirfd = handle.try_clone()?;

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
                tracing::debug!(cur_path = tracing::field::display(&cur_path), "path get");

                let ends_with_slash = cur_path.ends_with('/');
                let mut components = Path::new(&cur_path).components();
                let head = match components.next() {
                    None => return Err(Error::Noent),
                    Some(p) => p,
                };
                let tail = components.as_path();

                if tail.components().next().is_some() {
                    let mut tail = from_host(tail.as_os_str())?;
                    if ends_with_slash {
                        tail.push('/');
                    }
                    path_stack.push(tail);
                }

                tracing::debug!(path_stack = tracing::field::debug(&path_stack), "path_get");

                match head {
                    Component::Prefix(_) | Component::RootDir => {
                        // path is absolute!
                        return Err(Error::Notcapable);
                    }
                    Component::CurDir => {
                        // "." so skip
                    }
                    Component::ParentDir => {
                        // ".." so pop a dir
                        let _ = dir_stack.pop().ok_or(Error::Notcapable)?;

                        // we're not allowed to pop past the original directory
                        if dir_stack.is_empty() {
                            return Err(Error::Notcapable);
                        }
                    }
                    Component::Normal(head) => {
                        let mut head = from_host(head)?;
                        if ends_with_slash {
                            // preserve trailing slash
                            head.push('/');
                        }

                        if !path_stack.is_empty() || (ends_with_slash && !needs_final_component) {
                            let fd = dir_stack.last().ok_or(Error::Notcapable)?;
                            match fd.openat(
                                &head,
                                false,
                                false,
                                types::Oflags::DIRECTORY,
                                types::Fdflags::empty(),
                            ) {
                                Ok(new_dir) => {
                                    dir_stack.push(new_dir);
                                }
                                Err(e) => {
                                    match e {
                                        Error::Loop | Error::Mlink | Error::Notdir =>
                                        // Check to see if it was a symlink. Linux indicates
                                        // this with ENOTDIR because of the O_DIRECTORY flag.
                                        {
                                            // attempt symlink expansion
                                            let fd = dir_stack.last().ok_or(Error::Notcapable)?;
                                            let mut link_path = fd.readlinkat(&head)?;
                                            symlink_expansions += 1;
                                            if symlink_expansions > MAX_SYMLINK_EXPANSIONS {
                                                return Err(Error::Loop);
                                            }

                                            if head.ends_with('/') {
                                                link_path.push('/');
                                            }

                                            tracing::debug!(
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
                            || dirflags.contains(&types::Lookupflags::SYMLINK_FOLLOW)
                        {
                            // if there's a trailing slash, or if `LOOKUP_SYMLINK_FOLLOW` is set, attempt
                            // symlink expansion
                            let fd = dir_stack.last().ok_or(Error::Notcapable)?;
                            match fd.readlinkat(&head) {
                                Ok(mut link_path) => {
                                    symlink_expansions += 1;
                                    if symlink_expansions > MAX_SYMLINK_EXPANSIONS {
                                        return Err(Error::Loop);
                                    }

                                    if head.ends_with('/') {
                                        link_path.push('/');
                                    }

                                    tracing::debug!(
                                        "attempted symlink expansion link_path={:?}",
                                        link_path
                                    );

                                    path_stack.push(link_path);
                                    continue;
                                }
                                Err(Error::Inval) | Err(Error::Noent) | Err(Error::Notdir) => {
                                    // this handles the cases when trying to link to
                                    // a destination that already exists, and the target
                                    // path contains a slash
                                }
                                Err(e) => {
                                    return Err(e);
                                }
                            }
                        }

                        // not a symlink, so we're done;
                        return Ok((dir_stack.pop().ok_or(Error::Notcapable)?, head));
                    }
                }
            }
            None => {
                // no further components to process. means we've hit a case like "." or "a/..", or if the
                // input path has trailing slashes and `needs_final_component` is not set
                return Ok((dir_stack.pop().ok_or(Error::Notcapable)?, String::from(".")));
            }
        }
    }
}
