use super::{Entry, EntryHandle};
use crate::handle::{
    Fdflags, Filestat, Filetype, Fstflags, Handle, HandleRights, Lookupflags, Oflags, Rights, Size,
};
use crate::sched::Timestamp;
use crate::sys::path::{from_host, open_rights};
use crate::{Error, Result};
use std::convert::TryInto;
use std::ops::{Deref, DerefMut};
use std::path::{Component, Path};
use tracing::trace;
use wiggle::GuestPtr;

impl Entry {
    /// Normalizes a path to ensure that the target path is located under the directory provided.
    ///
    /// This is a workaround for not having Capsicum support in the OS.
    pub fn path_get(
        &self,
        required_rights: &HandleRights,
        dirflags: Lookupflags,
        path: &str,
        needs_final_component: bool,
    ) -> Result<(Box<dyn Handle>, String)> {
        const MAX_SYMLINK_EXPANSIONS: usize = 128;

        tracing::trace!(path = path);

        if path.contains('\0') {
            // if contains NUL, return Ilseq
            return Err(Error::Ilseq);
        }

        if self.get_file_type() != Filetype::Directory {
            // if `dirfd` doesn't refer to a directory, return `Notdir`.
            return Err(Error::Notdir);
        }

        let handle = self.as_handle(required_rights)?;
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

                            if !path_stack.is_empty() || (ends_with_slash && !needs_final_component)
                            {
                                let fd = dir_stack.last().ok_or(Error::Notcapable)?;
                                match fd.openat(
                                    &head,
                                    false,
                                    false,
                                    Oflags::DIRECTORY,
                                    Fdflags::empty(),
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
                                                let fd =
                                                    dir_stack.last().ok_or(Error::Notcapable)?;
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
                                || dirflags.contains(&Lookupflags::SYMLINK_FOLLOW)
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

    pub fn path_create_directory(&self, path: &str) -> Result<()> {
        let required_rights =
            HandleRights::from_base(Rights::PATH_OPEN | Rights::PATH_CREATE_DIRECTORY);
        let (dirfd, path) = self.path_get(&required_rights, Lookupflags::empty(), path, false)?;
        dirfd.create_directory(&path)
    }

    pub fn path_filestat_get(&self, flags: Lookupflags, path: &str) -> Result<Filestat> {
        let required_rights = HandleRights::from_base(Rights::PATH_FILESTAT_GET);
        let (dirfd, path) = self.path_get(&required_rights, flags, path, false)?;
        let host_filestat =
            dirfd.filestat_get_at(&path, flags.contains(&Lookupflags::SYMLINK_FOLLOW))?;
        Ok(host_filestat)
    }

    pub fn path_filestat_set_times(
        &self,
        flags: Lookupflags,
        path: &str,
        atim: Timestamp,
        mtim: Timestamp,
        fst_flags: Fstflags,
    ) -> Result<()> {
        let required_rights = HandleRights::from_base(Rights::PATH_FILESTAT_SET_TIMES);
        let (dirfd, path) = self.path_get(&required_rights, flags, path, false)?;
        dirfd.filestat_set_times_at(
            &path,
            atim,
            mtim,
            fst_flags,
            flags.contains(&Lookupflags::SYMLINK_FOLLOW),
        )?;
        Ok(())
    }

    pub fn path_link(
        &self,
        old_flags: Lookupflags,
        old_path: &GuestPtr<str>,
        new_entry: &Entry,
        new_path: &GuestPtr<str>,
    ) -> Result<()> {
        let required_rights = HandleRights::from_base(Rights::PATH_LINK_SOURCE);
        let (old_dirfd, old_path) = {
            // Each argument should only be borrowed for the scope its used, so
            // that they can overlap
            let old_path = old_path.as_str()?;
            self.path_get(
                &required_rights,
                Lookupflags::empty(),
                old_path.deref(),
                false,
            )?
        };
        let required_rights = HandleRights::from_base(Rights::PATH_LINK_TARGET);
        let (new_dirfd, new_path) = {
            let new_path = new_path.as_str()?;
            new_entry.path_get(
                &required_rights,
                Lookupflags::empty(),
                new_path.deref(),
                false,
            )?
        };
        old_dirfd.link(
            &old_path,
            new_dirfd,
            &new_path,
            old_flags.contains(&Lookupflags::SYMLINK_FOLLOW),
        )
    }

    pub fn path_open(
        &self,
        dirflags: Lookupflags,
        path: &str,
        oflags: Oflags,
        fs_rights_base: Rights,
        fs_rights_inheriting: Rights,
        fdflags: Fdflags,
    ) -> Result<Entry> {
        let needed_rights = open_rights(
            &HandleRights::new(fs_rights_base, fs_rights_inheriting),
            oflags,
            fdflags,
        );
        trace!(needed_rights = tracing::field::debug(&needed_rights));
        let (dirfd, path) = self.path_get(
            &needed_rights,
            dirflags,
            path,
            oflags & Oflags::CREAT != Oflags::empty(),
        )?;
        let read = fs_rights_base & (Rights::FD_READ | Rights::FD_READDIR) != Rights::empty();
        let write = fs_rights_base
            & (Rights::FD_DATASYNC
                | Rights::FD_WRITE
                | Rights::FD_ALLOCATE
                | Rights::FD_FILESTAT_SET_SIZE)
            != Rights::empty();
        trace!(read = read, write = write, "dirfd.openat");
        let fd = dirfd.openat(&path, read, write, oflags, fdflags)?;
        let entry = Entry::new(EntryHandle::from(fd));

        // We need to manually deny the rights which are not explicitly requested
        // because Entry::from will assign maximal consistent rights.
        let mut rights = entry.get_rights();
        rights.base &= fs_rights_base;
        rights.inheriting &= fs_rights_inheriting;
        entry.set_rights(rights);
        Ok(entry)
    }

    pub fn path_readlink(&self, path: &GuestPtr<str>, buf: &GuestPtr<[u8]>) -> Result<Size> {
        let required_rights = HandleRights::from_base(Rights::PATH_READLINK);
        let (dirfd, path) = {
            // Each argument should only be borrowed for the scope its used, so
            // that they can overlap
            let path = path.as_str()?;
            self.path_get(&required_rights, Lookupflags::empty(), path.deref(), false)?
        };
        let mut buf = buf.as_slice()?;
        let host_bufused = dirfd.readlink(&path, buf.deref_mut())?.try_into()?;
        Ok(host_bufused)
    }

    pub fn path_remove_directory(&self, path: &str) -> Result<()> {
        let required_rights = HandleRights::from_base(Rights::PATH_REMOVE_DIRECTORY);
        let (dirfd, path) = self.path_get(&required_rights, Lookupflags::empty(), path, true)?;
        dirfd.remove_directory(&path)
    }

    pub fn path_rename(
        &self,
        old_path: &GuestPtr<str>,
        new_entry: &Entry,
        new_path: &GuestPtr<str>,
    ) -> Result<()> {
        let required_rights = HandleRights::from_base(Rights::PATH_RENAME_SOURCE);
        let (old_dirfd, old_path) = {
            // Each path argument should only be borrowed for the scope its used, so
            // that they can overlap
            let old_path = old_path.as_str()?;
            self.path_get(
                &required_rights,
                Lookupflags::empty(),
                old_path.deref(),
                true,
            )?
        };
        let (new_dirfd, new_path) = {
            let new_path = new_path.as_str()?;
            new_entry.path_get(
                &required_rights,
                Lookupflags::empty(),
                new_path.deref(),
                true,
            )?
        };
        old_dirfd.rename(&old_path, new_dirfd, &new_path)
    }

    pub fn path_symlink(&self, old_path: &GuestPtr<str>, new_path: &GuestPtr<str>) -> Result<()> {
        let required_rights = HandleRights::from_base(Rights::PATH_SYMLINK);
        let (new_fd, new_path) = {
            let new_path = new_path.as_str()?;
            self.path_get(
                &required_rights,
                Lookupflags::empty(),
                new_path.deref(),
                true,
            )?
        };
        let old_path = old_path.as_str()?;
        new_fd.symlink(old_path.deref(), &new_path)
    }

    pub fn path_unlink_file(&self, path: &str) -> Result<()> {
        let required_rights = HandleRights::from_base(Rights::PATH_UNLINK_FILE);
        let (dirfd, path) = self.path_get(&required_rights, Lookupflags::empty(), path, false)?;
        dirfd.unlink_file(&path)
    }
}
