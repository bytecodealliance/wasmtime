use crate::sys;
use crate::sys::entry::OsHandle;
use crate::wasi::{types, Errno, Result};
use crate::{entry::Descriptor, entry::Entry};
use std::path::{Component, Path};
use std::str;
use wiggle::{GuestBorrows, GuestPtr};

pub(crate) use sys::path::*;

#[derive(Debug)]
pub(crate) struct PathGet {
    dirfd: Descriptor,
    path: String,
}

impl PathGet {
    pub(crate) fn dirfd(&self) -> &Descriptor {
        &self.dirfd
    }

    pub(crate) fn path(&self) -> &str {
        &self.path
    }

    pub(crate) fn create_directory(self) -> Result<()> {
        match &self.dirfd {
            Descriptor::OsHandle(file) => create_directory(&file, &self.path),
            Descriptor::VirtualFile(virt) => virt.create_directory(&Path::new(&self.path)),
            other => {
                panic!("invalid descriptor to create directory: {:?}", other);
            }
        }
    }

    pub(crate) fn open_with(
        self,
        read: bool,
        write: bool,
        oflags: types::Oflags,
        fs_flags: types::Fdflags,
    ) -> Result<Descriptor> {
        match &self.dirfd {
            Descriptor::OsHandle(_) => {
                open(self, read, write, oflags, fs_flags).map_err(Into::into)
            }
            Descriptor::VirtualFile(virt) => virt
                .openat(Path::new(&self.path), read, write, oflags, fs_flags)
                .map(|file| Descriptor::VirtualFile(file)),
            other => {
                panic!("invalid descriptor to path_open: {:?}", other);
            }
        }
    }
}

struct PathRef<'a, 'b> {
    dirfd: &'a Descriptor,
    path: &'b str,
}

impl<'a, 'b> PathRef<'a, 'b> {
    fn new(dirfd: &'a Descriptor, path: &'b str) -> Self {
        PathRef { dirfd, path }
    }

    fn open(&self) -> Result<Descriptor> {
        match self.dirfd {
            Descriptor::OsHandle(file) => Ok(Descriptor::OsHandle(OsHandle::from(openat(
                &file, &self.path,
            )?))),
            Descriptor::VirtualFile(virt) => virt
                .openat(
                    Path::new(&self.path),
                    false,
                    false,
                    types::Oflags::DIRECTORY,
                    types::Fdflags::empty(),
                )
                .map(|file| Descriptor::VirtualFile(file)),
            other => {
                panic!("invalid descriptor for open: {:?}", other);
            }
        }
    }

    fn readlink(&self) -> Result<String> {
        match self.dirfd {
            Descriptor::OsHandle(file) => readlinkat(file, self.path),
            Descriptor::VirtualFile(virt) => {
                virt.readlinkat(Path::new(self.path)).map_err(Into::into)
            }
            other => {
                panic!("invalid descriptor for readlink: {:?}", other);
            }
        }
    }
}

/// Normalizes a path to ensure that the target path is located under the directory provided.
///
/// This is a workaround for not having Capsicum support in the OS.
pub(crate) fn get(
    fe: &Entry,
    rights_base: types::Rights,
    rights_inheriting: types::Rights,
    dirflags: types::Lookupflags,
    path: &GuestPtr<'_, str>,
    needs_final_component: bool,
) -> Result<PathGet> {
    const MAX_SYMLINK_EXPANSIONS: usize = 128;

    // Extract path as &str from guest's memory.
    let path = unsafe {
        let mut bc = GuestBorrows::new();
        let raw = path.as_raw(&mut bc)?;
        &*raw
    };

    log::trace!("     | (path_ptr,path_len)='{}'", path);

    if path.contains('\0') {
        // if contains NUL, return Ilseq
        return Err(Errno::Ilseq);
    }

    if fe.file_type != types::Filetype::Directory {
        // if `dirfd` doesn't refer to a directory, return `Notdir`.
        return Err(Errno::Notdir);
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
    let mut path_stack = vec![path.to_owned()];

    // Track the number of symlinks we've expanded, so we can return `ELOOP` after too many.
    let mut symlink_expansions = 0;

    // TODO: rewrite this using a custom posix path type, with a component iterator that respects
    // trailing slashes. This version does way too much allocation, and is way too fiddly.
    loop {
        match path_stack.pop() {
            Some(cur_path) => {
                log::debug!("path_get cur_path = {:?}", cur_path);

                let ends_with_slash = cur_path.ends_with('/');
                let mut components = Path::new(&cur_path).components();
                let head = match components.next() {
                    None => return Err(Errno::Noent),
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

                log::debug!("path_get path_stack = {:?}", path_stack);

                match head {
                    Component::Prefix(_) | Component::RootDir => {
                        // path is absolute!
                        return Err(Errno::Notcapable);
                    }
                    Component::CurDir => {
                        // "." so skip
                    }
                    Component::ParentDir => {
                        // ".." so pop a dir
                        let _ = dir_stack.pop().ok_or(Errno::Notcapable)?;

                        // we're not allowed to pop past the original directory
                        if dir_stack.is_empty() {
                            return Err(Errno::Notcapable);
                        }
                    }
                    Component::Normal(head) => {
                        let mut head = from_host(head)?;
                        if ends_with_slash {
                            // preserve trailing slash
                            head.push('/');
                        }

                        if !path_stack.is_empty() || (ends_with_slash && !needs_final_component) {
                            match PathRef::new(dir_stack.last().ok_or(Errno::Notcapable)?, &head)
                                .open()
                            {
                                Ok(new_dir) => {
                                    dir_stack.push(new_dir);
                                }
                                Err(e) => {
                                    match e {
                                        Errno::Loop | Errno::Mlink | Errno::Notdir =>
                                        // Check to see if it was a symlink. Linux indicates
                                        // this with ENOTDIR because of the O_DIRECTORY flag.
                                        {
                                            // attempt symlink expansion
                                            let mut link_path = PathRef::new(
                                                dir_stack.last().ok_or(Errno::Notcapable)?,
                                                &head,
                                            )
                                            .readlink()?;

                                            symlink_expansions += 1;
                                            if symlink_expansions > MAX_SYMLINK_EXPANSIONS {
                                                return Err(Errno::Loop);
                                            }

                                            if head.ends_with('/') {
                                                link_path.push('/');
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
                            || dirflags.contains(&types::Lookupflags::SYMLINK_FOLLOW)
                        {
                            // if there's a trailing slash, or if `LOOKUP_SYMLINK_FOLLOW` is set, attempt
                            // symlink expansion
                            match PathRef::new(dir_stack.last().ok_or(Errno::Notcapable)?, &head)
                                .readlink()
                            {
                                Ok(mut link_path) => {
                                    symlink_expansions += 1;
                                    if symlink_expansions > MAX_SYMLINK_EXPANSIONS {
                                        return Err(Errno::Loop);
                                    }

                                    if head.ends_with('/') {
                                        link_path.push('/');
                                    }

                                    log::debug!(
                                        "attempted symlink expansion link_path={:?}",
                                        link_path
                                    );

                                    path_stack.push(link_path);
                                    continue;
                                }
                                Err(e) => {
                                    if e != Errno::Inval
                                        && e != Errno::Noent
                                        // this handles the cases when trying to link to
                                        // a destination that already exists, and the target
                                        // path contains a slash
                                        && e != Errno::Notdir
                                    {
                                        return Err(e);
                                    }
                                }
                            }
                        }

                        // not a symlink, so we're done;
                        return Ok(PathGet {
                            dirfd: dir_stack.pop().ok_or(Errno::Notcapable)?,
                            path: head,
                        });
                    }
                }
            }
            None => {
                // no further components to process. means we've hit a case like "." or "a/..", or if the
                // input path has trailing slashes and `needs_final_component` is not set
                return Ok(PathGet {
                    dirfd: dir_stack.pop().ok_or(Errno::Notcapable)?,
                    path: String::from("."),
                });
            }
        }
    }
}
