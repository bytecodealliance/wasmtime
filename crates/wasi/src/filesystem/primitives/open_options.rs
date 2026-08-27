use crate::filesystem::primitives::{FollowSymlinks, ImplOpenOptionsExt};

/// Options and flags which can be used to configure how a file is opened.
///
/// This corresponds to [`std::fs::OpenOptions`].
///
/// This `OpenOptions` has no `open` method. To open a file with an
/// `OptionOptions`, first obtain a [`Dir`] containing the path, and then call
/// [`Dir::open_with`].
///
/// [`Dir`]: https://docs.rs/cap-std/latest/cap_std/fs/struct.Dir.html
/// [`Dir::open_with`]: https://docs.rs/cap-std/latest/cap_std/fs/struct.Dir.html#method.open_with
///
/// <details>
/// We need to define our own version because the libstd `OpenOptions` doesn't
/// have public accessors that we can use.
/// </details>
#[derive(Debug, Clone)]
pub struct OpenOptions {
    pub(crate) read: bool,
    pub(crate) write: bool,
    pub(crate) append: bool,
    pub(crate) truncate: bool,
    pub(crate) create: bool,
    pub(crate) create_new: bool,
    pub(crate) dir_required: bool,
    #[cfg(windows)]
    pub(crate) maybe_dir: bool,
    pub(crate) sync: bool,
    pub(crate) dsync: bool,
    #[cfg(any(target_os = "linux", target_os = "android"))]
    pub(crate) rsync: bool,
    #[cfg(not(windows))]
    pub(crate) nonblock: bool,
    pub(crate) readdir_required: bool,
    pub(crate) follow: FollowSymlinks,

    #[cfg(any(unix, windows, target_os = "vxworks"))]
    pub(crate) ext: ImplOpenOptionsExt,
}

impl OpenOptions {
    /// Creates a blank new set of options ready for configuration.
    ///
    /// This corresponds to [`std::fs::OpenOptions::new`].
    #[allow(clippy::new_without_default)]
    #[inline]
    pub const fn new() -> Self {
        Self {
            read: false,
            write: false,
            append: false,
            truncate: false,
            create: false,
            create_new: false,
            dir_required: false,
            #[cfg(windows)]
            maybe_dir: false,
            sync: false,
            dsync: false,
            #[cfg(any(target_os = "linux", target_os = "android"))]
            rsync: false,
            #[cfg(not(windows))]
            nonblock: false,
            readdir_required: false,
            follow: FollowSymlinks::Yes,

            #[cfg(any(unix, windows, target_os = "vxworks"))]
            ext: ImplOpenOptionsExt::new(),
        }
    }

    /// Sets the option for read access.
    ///
    /// This corresponds to [`std::fs::OpenOptions::read`].
    #[inline]
    pub fn read(&mut self, read: bool) -> &mut Self {
        self.read = read;
        self
    }

    /// Sets the option for write access.
    ///
    /// This corresponds to [`std::fs::OpenOptions::write`].
    #[inline]
    pub fn write(&mut self, write: bool) -> &mut Self {
        self.write = write;
        self
    }

    /// Sets the option for truncating a previous file.
    ///
    /// This corresponds to [`std::fs::OpenOptions::truncate`].
    #[inline]
    pub fn truncate(&mut self, truncate: bool) -> &mut Self {
        self.truncate = truncate;
        self
    }

    /// Sets the option to create a new file.
    ///
    /// This corresponds to [`std::fs::OpenOptions::create`].
    #[inline]
    pub fn create(&mut self, create: bool) -> &mut Self {
        self.create = create;
        self
    }

    /// Sets the option to always create a new file.
    ///
    /// This corresponds to [`std::fs::OpenOptions::create_new`].
    #[inline]
    pub fn create_new(&mut self, create_new: bool) -> &mut Self {
        self.create_new = create_new;
        self
    }

    /// Sets the option to enable or suppress following of symlinks.
    #[inline]
    pub(crate) fn follow(&mut self, follow: FollowSymlinks) -> &mut Self {
        self.follow = follow;
        self
    }

    /// Sets the option to enable an error if the opened object is not a
    /// directory.
    #[inline]
    pub(crate) fn dir_required(&mut self, dir_required: bool) -> &mut Self {
        self.dir_required = dir_required;
        self
    }

    /// Sets the option to request the ability to read directory entries.
    #[inline]
    pub(crate) fn readdir_required(&mut self, readdir_required: bool) -> &mut Self {
        self.readdir_required = readdir_required;
        self
    }

    /// Wrapper to allow `follow` to be exposed by the `cap-fs-ext` crate.
    ///
    /// This is hidden from the main API since this functionality isn't present
    /// in `std`. Use `cap_fs_ext::OpenOptionsFollowExt` instead of calling
    /// this directly.
    #[doc(hidden)]
    #[inline]
    pub fn _cap_fs_ext_follow(&mut self, follow: FollowSymlinks) -> &mut Self {
        self.follow(follow)
    }
}

/// Unix-specific extensions to [`fs::OpenOptions`].
#[cfg(any(target_os = "linux", target_os = "android"))]
pub trait OpenOptionsExt {
    /// Pass custom flags to the `flags` argument of `open`.
    fn custom_flags(&mut self, flags: i32) -> &mut Self;
}

/// WASI-specific extensions to [`fs::OpenOptions`].
#[cfg(target_os = "wasi")]
pub trait OpenOptionsExt {
    /// Pass custom `dirflags` argument to `path_open`.
    fn lookup_flags(&mut self, flags: u32) -> &mut Self;

    /// Indicates whether `OpenOptions` must open a directory or not.
    fn directory(&mut self, dir: bool) -> &mut Self;

    /// Indicates whether `__WASI_FDFLAG_DSYNC` is passed in the `fs_flags`
    /// field of `path_open`.
    fn dsync(&mut self, dsync: bool) -> &mut Self;

    /// Indicates whether `__WASI_FDFLAG_NONBLOCK` is passed in the `fs_flags`
    /// field of `path_open`.
    fn nonblock(&mut self, nonblock: bool) -> &mut Self;

    /// Indicates whether `__WASI_FDFLAG_RSYNC` is passed in the `fs_flags`
    /// field of `path_open`.
    fn rsync(&mut self, rsync: bool) -> &mut Self;

    /// Indicates whether `__WASI_FDFLAG_SYNC` is passed in the `fs_flags`
    /// field of `path_open`.
    fn sync(&mut self, sync: bool) -> &mut Self;

    /// Indicates the value that should be passed in for the `fs_rights_base`
    /// parameter of `path_open`.
    fn fs_rights_base(&mut self, rights: u64) -> &mut Self;

    /// Indicates the value that should be passed in for the
    /// `fs_rights_inheriting` parameter of `path_open`.
    fn fs_rights_inheriting(&mut self, rights: u64) -> &mut Self;

    /// Open a file or directory.
    fn open_at<P: AsRef<std::path::Path>>(
        &self,
        file: &std::fs::File,
        path: P,
    ) -> std::io::Result<std::fs::File>;
}

/// Windows-specific extensions to [`fs::OpenOptions`].
#[cfg(windows)]
pub trait OpenOptionsExt {
    /// Overrides the `dwDesiredAccess` argument to the call to [`CreateFile`]
    /// with the specified value.
    fn access_mode(&mut self, access: u32) -> &mut Self;

    /// Overrides the `dwShareMode` argument to the call to [`CreateFile`] with
    /// the specified value.
    fn share_mode(&mut self, val: u32) -> &mut Self;

    /// Sets extra flags for the `dwFileFlags` argument to the call to
    /// [`CreateFile2`] to the specified value (or combines it with
    /// `attributes` and `security_qos_flags` to set the `dwFlagsAndAttributes`
    /// for [`CreateFile`]).
    ///
    /// [`CreateFile`]: https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilea
    /// [`CreateFile2`]: https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfile2
    fn custom_flags(&mut self, flags: u32) -> &mut Self;
}

#[cfg(any(target_os = "linux", target_os = "android"))]
impl OpenOptionsExt for OpenOptions {
    #[inline]
    fn custom_flags(&mut self, flags: i32) -> &mut Self {
        self.ext.custom_flags(flags);
        self
    }
}

#[cfg(target_os = "wasi")]
impl OpenOptionsExt for OpenOptions {
    fn lookup_flags(&mut self, _: u32) -> &mut Self {
        todo!()
    }

    fn directory(&mut self, dir_required: bool) -> &mut Self {
        self.dir_required = dir_required;
        self
    }

    fn dsync(&mut self, _: bool) -> &mut Self {
        todo!()
    }

    fn nonblock(&mut self, _: bool) -> &mut Self {
        todo!()
    }

    fn rsync(&mut self, _: bool) -> &mut Self {
        todo!()
    }

    fn sync(&mut self, _: bool) -> &mut Self {
        todo!()
    }

    fn fs_rights_base(&mut self, _: u64) -> &mut Self {
        todo!()
    }

    fn fs_rights_inheriting(&mut self, _: u64) -> &mut Self {
        todo!()
    }

    fn open_at<P>(&self, dirfd: &std::fs::File, path: P) -> Result<std::fs::File, std::io::Error>
    where
        P: AsRef<std::path::Path>,
    {
        crate::fs::open(dirfd, path.as_ref(), self)
    }
}

#[cfg(target_os = "vxworks")]
impl OpenOptionsExt for OpenOptions {
    #[inline]
    fn mode(&mut self, mode: u32) -> &mut Self {
        self.ext.mode(mode);
        self
    }

    #[inline]
    fn custom_flags(&mut self, flags: i32) -> &mut Self {
        self.ext.custom_flags(flags);
        self
    }
}

#[cfg(windows)]
impl OpenOptionsExt for OpenOptions {
    #[inline]
    fn access_mode(&mut self, access: u32) -> &mut Self {
        self.ext.access_mode(access);
        self
    }

    /// To prevent race conditions on Windows, handles for directories must be
    /// opened without `FILE_SHARE_DELETE`.
    #[inline]
    fn share_mode(&mut self, val: u32) -> &mut Self {
        self.ext.share_mode(val);
        self
    }

    #[inline]
    fn custom_flags(&mut self, flags: u32) -> &mut Self {
        self.ext.custom_flags(flags);
        self
    }
}
