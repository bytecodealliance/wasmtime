use crate::clocks::Datetime;
use crate::runtime::{AbortOnDropJoinHandle, spawn_blocking};
use cap_primitives::fs::{DirOptions, FollowSymlinks, Metadata, OpenOptions, SystemTimeSpec};
use std::collections::hash_map;
use std::sync::Arc;
use std::time::SystemTime;
use tracing::debug;
use wasmtime::component::{HasData, Resource, ResourceTable};
use wasmtime::error::Context as _;

#[cfg(unix)]
pub(crate) mod unix;
#[cfg(unix)]
pub(crate) use unix as sys;
#[cfg(windows)]
pub(crate) mod windows;
#[cfg(windows)]
pub(crate) use windows as sys;

/// A helper struct which implements [`HasData`] for the `wasi:filesystem` APIs.
///
/// This can be useful when directly calling `add_to_linker` functions directly,
/// such as [`wasmtime_wasi::p2::bindings::filesystem::types::add_to_linker`] as
/// the `D` type parameter. See [`HasData`] for more information about the type
/// parameter's purpose.
///
/// When using this type you can skip the [`WasiFilesystemView`] trait, for
/// example.
///
/// [`wasmtime_wasi::p2::bindings::filesystem::types::add_to_linker`]: crate::p2::bindings::filesystem::types::add_to_linker
///
/// # Examples
///
/// ```
/// use wasmtime::component::{Linker, ResourceTable};
/// use wasmtime::{Engine, Result};
/// use wasmtime_wasi::filesystem::*;
///
/// struct MyStoreState {
///     table: ResourceTable,
///     filesystem: WasiFilesystemCtx,
/// }
///
/// fn main() -> Result<()> {
///     let engine = Engine::default();
///     let mut linker = Linker::new(&engine);
///
///     wasmtime_wasi::p2::bindings::filesystem::types::add_to_linker::<MyStoreState, WasiFilesystem>(
///         &mut linker,
///         |state| WasiFilesystemCtxView {
///             table: &mut state.table,
///             ctx: &mut state.filesystem,
///         },
///     )?;
///     Ok(())
/// }
/// ```
pub struct WasiFilesystem;

impl HasData for WasiFilesystem {
    type Data<'a> = WasiFilesystemCtxView<'a>;
}

#[derive(Clone, Default)]
pub struct WasiFilesystemCtx {
    pub(crate) allow_blocking_current_thread: bool,
    pub(crate) preopens: Vec<(Dir, String)>,
}

pub struct WasiFilesystemCtxView<'a> {
    pub ctx: &'a mut WasiFilesystemCtx,
    pub table: &'a mut ResourceTable,
}

pub trait WasiFilesystemView: Send {
    fn filesystem(&mut self) -> WasiFilesystemCtxView<'_>;
}

bitflags::bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct FilePerms: usize {
        const READ = 0b1;
        const WRITE = 0b10;
    }
}

bitflags::bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct OpenMode: usize {
        const READ = 0b1;
        const WRITE = 0b10;
    }
}

bitflags::bitflags! {
    /// Permission bits for operating on a directory.
    ///
    /// Directories can be limited to being readonly. This will restrict what
    /// can be done with them, for example preventing creation of new files.
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct DirPerms: usize {
        /// This directory can be read, for example its entries can be iterated
        /// over and files can be opened.
        const READ = 0b1;

        /// This directory can be mutated, for example by creating new files
        /// within it.
        const MUTATE = 0b10;
    }
}

bitflags::bitflags! {
    /// Flags determining the method of how paths are resolved.
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub(crate) struct PathFlags: usize {
        /// This directory can be read, for example its entries can be iterated
        /// over and files can be opened.
        const SYMLINK_FOLLOW = 0b1;
    }
}

bitflags::bitflags! {
    /// Open flags used by `open-at`.
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub(crate) struct OpenFlags: usize {
        /// Create file if it does not exist, similar to `O_CREAT` in POSIX.
        const CREATE = 0b1;
        /// Fail if not a directory, similar to `O_DIRECTORY` in POSIX.
        const DIRECTORY = 0b10;
        /// Fail if file already exists, similar to `O_EXCL` in POSIX.
        const EXCLUSIVE = 0b100;
        /// Truncate file to size 0, similar to `O_TRUNC` in POSIX.
        const TRUNCATE = 0b1000;
    }
}

bitflags::bitflags! {
    /// Descriptor flags.
    ///
    /// Note: This was called `fdflags` in earlier versions of WASI.
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub(crate) struct DescriptorFlags: usize {
        /// Read mode: Data can be read.
        const READ = 0b1;
        /// Write mode: Data can be written to.
        const WRITE = 0b10;
        /// Request that writes be performed according to synchronized I/O file
        /// integrity completion. The data stored in the file and the file's
        /// metadata are synchronized. This is similar to `O_SYNC` in POSIX.
        ///
        /// The precise semantics of this operation have not yet been defined for
        /// WASI. At this time, it should be interpreted as a request, and not a
        /// requirement.
        const FILE_INTEGRITY_SYNC = 0b100;
        /// Request that writes be performed according to synchronized I/O data
        /// integrity completion. Only the data stored in the file is
        /// synchronized. This is similar to `O_DSYNC` in POSIX.
        ///
        /// The precise semantics of this operation have not yet been defined for
        /// WASI. At this time, it should be interpreted as a request, and not a
        /// requirement.
        const DATA_INTEGRITY_SYNC = 0b1000;
        /// Requests that reads be performed at the same level of integrity
        /// requested for writes. This is similar to `O_RSYNC` in POSIX.
        ///
        /// The precise semantics of this operation have not yet been defined for
        /// WASI. At this time, it should be interpreted as a request, and not a
        /// requirement.
        const REQUESTED_WRITE_SYNC = 0b10000;
        /// Mutating directories mode: Directory contents may be mutated.
        ///
        /// When this flag is unset on a descriptor, operations using the
        /// descriptor which would create, rename, delete, modify the data or
        /// metadata of filesystem objects, or obtain another handle which
        /// would permit any of those, shall fail with `error-code::read-only` if
        /// they would otherwise succeed.
        ///
        /// This may only be set on directories.
        const MUTATE_DIRECTORY = 0b100000;
    }
}

/// Error codes returned by functions, similar to `errno` in POSIX.
/// Not all of these error codes are returned by the functions provided by this
/// API; some are used in higher-level library layers, and others are provided
/// merely for alignment with POSIX.
#[cfg_attr(
    windows,
    expect(dead_code, reason = "on Windows, some of these are not used")
)]
pub(crate) enum ErrorCode {
    /// Permission denied, similar to `EACCES` in POSIX.
    Access,
    /// Connection already in progress, similar to `EALREADY` in POSIX.
    Already,
    /// Bad descriptor, similar to `EBADF` in POSIX.
    BadDescriptor,
    /// Device or resource busy, similar to `EBUSY` in POSIX.
    Busy,
    /// File exists, similar to `EEXIST` in POSIX.
    Exist,
    /// File too large, similar to `EFBIG` in POSIX.
    FileTooLarge,
    /// Illegal byte sequence, similar to `EILSEQ` in POSIX.
    IllegalByteSequence,
    /// Operation in progress, similar to `EINPROGRESS` in POSIX.
    InProgress,
    /// Interrupted function, similar to `EINTR` in POSIX.
    Interrupted,
    /// Invalid argument, similar to `EINVAL` in POSIX.
    Invalid,
    /// I/O error, similar to `EIO` in POSIX.
    Io,
    /// Is a directory, similar to `EISDIR` in POSIX.
    IsDirectory,
    /// Too many levels of symbolic links, similar to `ELOOP` in POSIX.
    Loop,
    /// Too many links, similar to `EMLINK` in POSIX.
    TooManyLinks,
    /// Filename too long, similar to `ENAMETOOLONG` in POSIX.
    NameTooLong,
    /// No such file or directory, similar to `ENOENT` in POSIX.
    NoEntry,
    /// Not enough space, similar to `ENOMEM` in POSIX.
    InsufficientMemory,
    /// No space left on device, similar to `ENOSPC` in POSIX.
    InsufficientSpace,
    /// Not a directory or a symbolic link to a directory, similar to `ENOTDIR` in POSIX.
    NotDirectory,
    /// Directory not empty, similar to `ENOTEMPTY` in POSIX.
    NotEmpty,
    /// Not supported, similar to `ENOTSUP` and `ENOSYS` in POSIX.
    Unsupported,
    /// Value too large to be stored in data type, similar to `EOVERFLOW` in POSIX.
    Overflow,
    /// Operation not permitted, similar to `EPERM` in POSIX.
    NotPermitted,
    /// Broken pipe, similar to `EPIPE` in POSIX.
    Pipe,
    /// Invalid seek, similar to `ESPIPE` in POSIX.
    InvalidSeek,
}

/// The type of a filesystem object referenced by a descriptor.
///
/// Note: This was called `filetype` in earlier versions of WASI.
pub(crate) enum DescriptorType {
    /// The type of the descriptor or file is unknown or is different from
    /// any of the other types specified.
    Unknown,
    /// The descriptor refers to a block device inode.
    #[cfg_attr(
        windows,
        expect(dead_code, reason = "windows has no notion of block devices")
    )]
    BlockDevice,
    /// The descriptor refers to a character device inode.
    CharacterDevice,
    /// The descriptor refers to a directory inode.
    Directory,
    /// The file refers to a symbolic link inode.
    SymbolicLink,
    /// The descriptor refers to a regular file inode.
    RegularFile,
}

impl From<cap_primitives::fs::FileType> for DescriptorType {
    fn from(ft: cap_primitives::fs::FileType) -> Self {
        if ft.is_dir() {
            DescriptorType::Directory
        } else if ft.is_symlink() {
            DescriptorType::SymbolicLink
        } else if ft.is_file() {
            DescriptorType::RegularFile
        } else {
            sys::descriptor_type(ft)
        }
    }
}

/// File attributes.
///
/// Note: This was called `filestat` in earlier versions of WASI.
pub(crate) struct DescriptorStat {
    /// File type.
    pub type_: DescriptorType,
    /// Number of hard links to the file.
    pub link_count: u64,
    /// For regular files, the file size in bytes. For symbolic links, the
    /// length in bytes of the pathname contained in the symbolic link.
    pub size: u64,
    /// Last data access timestamp.
    ///
    /// If the `option` is none, the platform doesn't maintain an access
    /// timestamp for this file.
    pub data_access_timestamp: Option<Datetime>,
    /// Last data modification timestamp.
    ///
    /// If the `option` is none, the platform doesn't maintain a
    /// modification timestamp for this file.
    pub data_modification_timestamp: Option<Datetime>,
    /// Last file status-change timestamp.
    ///
    /// If the `option` is none, the platform doesn't maintain a
    /// status-change timestamp for this file.
    pub status_change_timestamp: Option<Datetime>,
}

impl DescriptorStat {
    /// Creates a `DescriptorStat` from a `Metadata` plus the hard link
    /// count.
    fn new(meta: &Metadata, link_count: u64) -> Self {
        Self {
            type_: meta.file_type().into(),
            link_count,
            size: meta.len(),
            data_access_timestamp: meta
                .accessed()
                .ok()
                .and_then(|t| Datetime::try_from(t.into_std()).ok()),
            data_modification_timestamp: meta
                .modified()
                .ok()
                .and_then(|t| Datetime::try_from(t.into_std()).ok()),
            status_change_timestamp: meta
                .created()
                .ok()
                .and_then(|t| Datetime::try_from(t.into_std()).ok()),
        }
    }
}

/// A 128-bit hash value, split into parts because wasm doesn't have a
/// 128-bit integer type.
pub(crate) struct MetadataHashValue {
    /// 64 bits of a 128-bit hash value.
    pub lower: u64,
    /// Another 64 bits of a 128-bit hash value.
    pub upper: u64,
}

impl MetadataHashValue {
    /// Creates a hash value from a file's unique identity, e.g. a
    /// device/inode number pair.
    fn new(identity: impl std::hash::Hash) -> Self {
        // Without incurring any deps, std provides us with a 64 bit hash
        // function:
        use std::hash::Hasher as _;
        // Note that this means that the metadata hash (which becomes a preview1 ino) may
        // change when a different rustc release is used to build this host implementation:
        let mut hasher = hash_map::DefaultHasher::new();
        identity.hash(&mut hasher);
        let lower = hasher.finish();
        // MetadataHashValue has a pair of 64-bit members for representing a
        // single 128-bit number. However, we only have 64 bits of entropy. To
        // synthesize the upper 64 bits, lets xor the lower half with an arbitrary
        // constant, in this case the 64 bit integer corresponding to the IEEE
        // double representation of (a number as close as possible to) pi.
        // This seems better than just repeating the same bits in the upper and
        // lower parts outright, which could make folks wonder if the struct was
        // mangled in the ABI, or worse yet, lead to consumers of this interface
        // expecting them to be equal.
        let upper = lower ^ 4614256656552045848u64;
        Self { lower, upper }
    }
}

#[derive(Copy, Clone, Debug)]
pub(crate) enum Advice {
    Normal,
    Sequential,
    Random,
    WillNeed,
    DontNeed,
    NoReuse,
}

#[cfg(unix)]
fn from_raw_os_error(err: Option<i32>) -> Option<ErrorCode> {
    use rustix::io::Errno as RustixErrno;
    if err.is_none() {
        return None;
    }
    Some(match RustixErrno::from_raw_os_error(err.unwrap()) {
        RustixErrno::PIPE => ErrorCode::Pipe,
        RustixErrno::PERM => ErrorCode::NotPermitted,
        RustixErrno::NOENT => ErrorCode::NoEntry,
        RustixErrno::NOMEM => ErrorCode::InsufficientMemory,
        RustixErrno::IO => ErrorCode::Io,
        RustixErrno::BADF => ErrorCode::BadDescriptor,
        RustixErrno::BUSY => ErrorCode::Busy,
        RustixErrno::ACCESS => ErrorCode::Access,
        RustixErrno::NOTDIR => ErrorCode::NotDirectory,
        RustixErrno::ISDIR => ErrorCode::IsDirectory,
        RustixErrno::INVAL => ErrorCode::Invalid,
        RustixErrno::EXIST => ErrorCode::Exist,
        RustixErrno::FBIG => ErrorCode::FileTooLarge,
        RustixErrno::NOSPC => ErrorCode::InsufficientSpace,
        RustixErrno::SPIPE => ErrorCode::InvalidSeek,
        RustixErrno::MLINK => ErrorCode::TooManyLinks,
        RustixErrno::NAMETOOLONG => ErrorCode::NameTooLong,
        RustixErrno::NOTEMPTY => ErrorCode::NotEmpty,
        RustixErrno::LOOP => ErrorCode::Loop,
        RustixErrno::OVERFLOW => ErrorCode::Overflow,
        RustixErrno::ILSEQ => ErrorCode::IllegalByteSequence,
        RustixErrno::NOTSUP => ErrorCode::Unsupported,
        RustixErrno::ALREADY => ErrorCode::Already,
        RustixErrno::INPROGRESS => ErrorCode::InProgress,
        RustixErrno::INTR => ErrorCode::Interrupted,

        // On some platforms, these have the same value as other errno values.
        #[allow(unreachable_patterns, reason = "see comment")]
        RustixErrno::OPNOTSUPP => ErrorCode::Unsupported,

        _ => return None,
    })
}

#[cfg(windows)]
fn from_raw_os_error(raw_os_error: Option<i32>) -> Option<ErrorCode> {
    use windows_sys::Win32::Foundation;
    Some(match raw_os_error.map(|code| code as u32) {
        Some(Foundation::ERROR_FILE_NOT_FOUND) => ErrorCode::NoEntry,
        Some(Foundation::ERROR_PATH_NOT_FOUND) => ErrorCode::NoEntry,
        Some(Foundation::ERROR_ACCESS_DENIED) => ErrorCode::Access,
        Some(Foundation::ERROR_SHARING_VIOLATION) => ErrorCode::Access,
        Some(Foundation::ERROR_PRIVILEGE_NOT_HELD) => ErrorCode::NotPermitted,
        Some(Foundation::ERROR_INVALID_HANDLE) => ErrorCode::BadDescriptor,
        Some(Foundation::ERROR_INVALID_NAME) => ErrorCode::NoEntry,
        Some(Foundation::ERROR_NOT_ENOUGH_MEMORY) => ErrorCode::InsufficientMemory,
        Some(Foundation::ERROR_OUTOFMEMORY) => ErrorCode::InsufficientMemory,
        Some(Foundation::ERROR_DIR_NOT_EMPTY) => ErrorCode::NotEmpty,
        Some(Foundation::ERROR_NOT_READY) => ErrorCode::Busy,
        Some(Foundation::ERROR_BUSY) => ErrorCode::Busy,
        Some(Foundation::ERROR_NOT_SUPPORTED) => ErrorCode::Unsupported,
        Some(Foundation::ERROR_FILE_EXISTS) => ErrorCode::Exist,
        Some(Foundation::ERROR_BROKEN_PIPE) => ErrorCode::Pipe,
        Some(Foundation::ERROR_BUFFER_OVERFLOW) => ErrorCode::NameTooLong,
        Some(Foundation::ERROR_NOT_A_REPARSE_POINT) => ErrorCode::Invalid,
        Some(Foundation::ERROR_NEGATIVE_SEEK) => ErrorCode::Invalid,
        Some(Foundation::ERROR_DIRECTORY) => ErrorCode::NotDirectory,
        Some(Foundation::ERROR_ALREADY_EXISTS) => ErrorCode::Exist,
        Some(Foundation::ERROR_STOPPED_ON_SYMLINK) => ErrorCode::Loop,
        Some(Foundation::ERROR_DIRECTORY_NOT_SUPPORTED) => ErrorCode::IsDirectory,
        _ => return None,
    })
}

impl<'a> From<&'a std::io::Error> for ErrorCode {
    fn from(err: &'a std::io::Error) -> ErrorCode {
        match from_raw_os_error(err.raw_os_error()) {
            Some(errno) => errno,
            None => {
                debug!("unknown raw os error: {err}");
                match err.kind() {
                    std::io::ErrorKind::NotFound => ErrorCode::NoEntry,
                    std::io::ErrorKind::PermissionDenied => ErrorCode::NotPermitted,
                    std::io::ErrorKind::AlreadyExists => ErrorCode::Exist,
                    std::io::ErrorKind::InvalidInput => ErrorCode::Invalid,
                    _ => ErrorCode::Io,
                }
            }
        }
    }
}

impl From<std::io::Error> for ErrorCode {
    fn from(err: std::io::Error) -> ErrorCode {
        ErrorCode::from(&err)
    }
}

#[derive(Clone)]
pub enum Descriptor {
    File(File),
    Dir(Dir),
}

impl Descriptor {
    pub(crate) fn file(&self) -> Result<&File, ErrorCode> {
        match self {
            Descriptor::File(f) => Ok(f),
            Descriptor::Dir(_) => Err(ErrorCode::BadDescriptor),
        }
    }

    pub(crate) fn dir(&self) -> Result<&Dir, ErrorCode> {
        match self {
            Descriptor::Dir(d) => Ok(d),
            Descriptor::File(_) => Err(ErrorCode::NotDirectory),
        }
    }

    pub(crate) async fn sync_data(&self) -> Result<(), ErrorCode> {
        match self {
            Self::File(f) => {
                match f.run_blocking(|f| f.sync_data()).await {
                    Ok(()) => Ok(()),
                    // On windows, `sync_data` uses `FileFlushBuffers` which fails with
                    // `ERROR_ACCESS_DENIED` if the file is not upen for writing. Ignore
                    // this error, for POSIX compatibility.
                    #[cfg(windows)]
                    Err(err)
                        if err.raw_os_error()
                            == Some(windows_sys::Win32::Foundation::ERROR_ACCESS_DENIED as _) =>
                    {
                        Ok(())
                    }
                    Err(err) => Err(err.into()),
                }
            }
            Self::Dir(d) => {
                d.run_blocking(|d| {
                    let d = cap_primitives::fs::open(
                        d,
                        std::path::Component::CurDir.as_ref(),
                        OpenOptions::new().read(true),
                    )?;
                    d.sync_data()?;
                    Ok(())
                })
                .await
            }
        }
    }

    pub(crate) async fn get_flags(&self) -> Result<DescriptorFlags, ErrorCode> {
        match self {
            Self::File(f) => {
                let mut flags = f.run_blocking(|f| sys::get_flags(f)).await?;
                if f.open_mode.contains(OpenMode::READ) {
                    flags |= DescriptorFlags::READ;
                }
                if f.open_mode.contains(OpenMode::WRITE) {
                    flags |= DescriptorFlags::WRITE;
                }
                Ok(flags)
            }
            Self::Dir(d) => {
                let mut flags = d.run_blocking(|d| sys::get_flags(d)).await?;
                if d.open_mode.contains(OpenMode::READ) {
                    flags |= DescriptorFlags::READ;
                }
                if d.open_mode.contains(OpenMode::WRITE) {
                    flags |= DescriptorFlags::MUTATE_DIRECTORY;
                }
                Ok(flags)
            }
        }
    }

    pub(crate) async fn get_type(&self) -> Result<DescriptorType, ErrorCode> {
        match self {
            Self::File(f) => {
                let meta = f.run_blocking(|f| Metadata::from_file(f)).await?;
                Ok(meta.file_type().into())
            }
            Self::Dir(_) => Ok(DescriptorType::Directory),
        }
    }

    pub(crate) async fn set_times(
        &self,
        atim: Option<SystemTime>,
        mtim: Option<SystemTime>,
    ) -> Result<(), ErrorCode> {
        let mut times = std::fs::FileTimes::new();
        if let Some(atim) = atim {
            times = times.set_accessed(atim);
        }
        if let Some(mtim) = mtim {
            times = times.set_modified(mtim);
        }
        match self {
            Self::File(f) => {
                if !f.perms.contains(FilePerms::WRITE) {
                    return Err(ErrorCode::NotPermitted);
                }
                f.run_blocking(move |f| f.set_times(times)).await?;
                Ok(())
            }
            Self::Dir(d) => {
                if !d.perms.contains(DirPerms::MUTATE) {
                    return Err(ErrorCode::NotPermitted);
                }
                d.run_blocking(move |d| d.set_times(times)).await?;
                Ok(())
            }
        }
    }

    pub(crate) async fn sync(&self) -> Result<(), ErrorCode> {
        match self {
            Self::File(f) => {
                match f.run_blocking(|f| f.sync_all()).await {
                    Ok(()) => Ok(()),
                    // On windows, `sync_data` uses `FileFlushBuffers` which fails with
                    // `ERROR_ACCESS_DENIED` if the file is not upen for writing. Ignore
                    // this error, for POSIX compatibility.
                    #[cfg(windows)]
                    Err(err)
                        if err.raw_os_error()
                            == Some(windows_sys::Win32::Foundation::ERROR_ACCESS_DENIED as _) =>
                    {
                        Ok(())
                    }
                    Err(err) => Err(err.into()),
                }
            }
            Self::Dir(d) => {
                d.run_blocking(|d| {
                    let d = cap_primitives::fs::open(
                        d,
                        std::path::Component::CurDir.as_ref(),
                        OpenOptions::new().read(true),
                    )?;
                    d.sync_all()?;
                    Ok(())
                })
                .await
            }
        }
    }

    pub(crate) async fn stat(&self) -> Result<DescriptorStat, ErrorCode> {
        match self {
            Self::File(f) => Ok(f.run_blocking(|f| sys::stat(f)).await?),
            Self::Dir(d) => Ok(d.run_blocking(|f| sys::stat(f)).await?),
        }
    }

    pub(crate) async fn is_same_object(&self, other: &Self) -> wasmtime::Result<bool> {
        // No permissions check on metadata: if opened, allowed to stat it
        let other = match other {
            Self::File(f) => Arc::clone(&f.file),
            Self::Dir(d) => Arc::clone(&d.dir),
        };
        Ok(match self {
            Self::File(f) => {
                f.run_blocking(move |f| sys::is_same_file(f, &other))
                    .await?
            }
            Self::Dir(d) => {
                d.run_blocking(move |d| sys::is_same_file(d, &other))
                    .await?
            }
        })
    }

    pub(crate) async fn metadata_hash(&self) -> Result<MetadataHashValue, ErrorCode> {
        match self {
            Self::File(f) => Ok(f.run_blocking(|f| sys::metadata_hash(f)).await?),
            Self::Dir(d) => Ok(d.run_blocking(|d| sys::metadata_hash(d)).await?),
        }
    }
}

#[derive(Clone)]
pub struct File {
    /// The operating system File this struct is mediating access to.
    ///
    /// Wrapped in an Arc because the same underlying file is used for
    /// implementing the stream types. A copy is also needed for
    /// `spawn_blocking`.
    pub file: Arc<std::fs::File>,
    /// Permissions to enforce on access to the file. These permissions are
    /// specified by a user of the `crate::WasiCtxBuilder`, and are
    /// enforced prior to any enforced by the underlying operating system.
    pub perms: FilePerms,
    /// The mode the file was opened under: bits for reading, and writing.
    /// Required to correctly report the DescriptorFlags, because
    /// cap-primitives doesn't presently provide a cross-platform equivalent
    /// of reading the oflags back out using fcntl.
    pub open_mode: OpenMode,

    allow_blocking_current_thread: bool,
}

impl File {
    pub fn new(
        file: std::fs::File,
        perms: FilePerms,
        open_mode: OpenMode,
        allow_blocking_current_thread: bool,
    ) -> Self {
        Self {
            file: Arc::new(file),
            perms,
            open_mode,
            allow_blocking_current_thread,
        }
    }

    /// Execute the blocking `body` function.
    ///
    /// Depending on how the WasiCtx was configured, the body may either be:
    /// - Executed directly on the current thread. In this case the `async`
    ///   signature of this method is effectively a lie and the returned
    ///   Future will always be immediately Ready. Or:
    /// - Spawned on a background thread using [`tokio::task::spawn_blocking`]
    ///   and immediately awaited.
    ///
    /// Intentionally blocking the executor thread might seem unorthodox, but is
    /// not actually a problem for specific workloads. See:
    /// - [`crate::WasiCtxBuilder::allow_blocking_current_thread`]
    /// - [Poor performance of wasmtime file I/O maybe because tokio](https://github.com/bytecodealliance/wasmtime/issues/7973)
    /// - [Implement opt-in for enabling WASI to block the current thread](https://github.com/bytecodealliance/wasmtime/pull/8190)
    pub(crate) async fn run_blocking<F, R>(&self, body: F) -> R
    where
        F: FnOnce(&std::fs::File) -> R + Send + 'static,
        R: Send + 'static,
    {
        match self.as_blocking_file() {
            Some(file) => body(file),
            None => self.spawn_blocking(body).await,
        }
    }

    pub(crate) fn spawn_blocking<F, R>(&self, body: F) -> AbortOnDropJoinHandle<R>
    where
        F: FnOnce(&std::fs::File) -> R + Send + 'static,
        R: Send + 'static,
    {
        let f = self.file.clone();
        spawn_blocking(move || body(&f))
    }

    /// Returns `Some` when the current thread is allowed to block in filesystem
    /// operations, and otherwise returns `None` to indicate that
    /// `spawn_blocking` must be used.
    pub(crate) fn as_blocking_file(&self) -> Option<&std::fs::File> {
        if self.allow_blocking_current_thread {
            Some(&self.file)
        } else {
            None
        }
    }

    /// Returns reference to the underlying [`std::fs::File`]
    #[cfg(feature = "p3")]
    pub(crate) fn as_file(&self) -> &Arc<std::fs::File> {
        &self.file
    }

    pub(crate) async fn advise(
        &self,
        offset: u64,
        len: u64,
        advice: Advice,
    ) -> Result<(), ErrorCode> {
        self.run_blocking(move |f| sys::advise(f, offset, len, advice))
            .await?;
        Ok(())
    }

    pub(crate) async fn set_size(&self, size: u64) -> Result<(), ErrorCode> {
        if !self.perms.contains(FilePerms::WRITE) {
            return Err(ErrorCode::NotPermitted);
        }
        self.run_blocking(move |f| f.set_len(size)).await?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct Dir {
    /// The operating system file descriptor this struct is mediating access
    /// to.
    ///
    /// This is a handle to a directory, and all paths accessed through this
    /// struct are sandboxed to be within this directory via `cap-primitives`.
    ///
    /// Wrapped in an Arc because a copy is needed for `run_blocking`.
    pub dir: Arc<std::fs::File>,
    /// Permissions to enforce on access to this directory. These permissions
    /// are specified by a user of the `crate::WasiCtxBuilder`, and
    /// are enforced prior to any enforced by the underlying operating system.
    ///
    /// These permissions are also enforced on any directories opened under
    /// this directory.
    pub perms: DirPerms,
    /// Permissions to enforce on any files opened under this directory.
    pub file_perms: FilePerms,
    /// The mode the directory was opened under: bits for reading, and writing.
    /// Required to correctly report the DescriptorFlags, because
    /// cap-primitives doesn't presently provide a cross-platform equivalent
    /// of reading the oflags back out using fcntl.
    pub open_mode: OpenMode,

    pub(crate) allow_blocking_current_thread: bool,
}

impl Dir {
    pub fn new(
        dir: std::fs::File,
        perms: DirPerms,
        file_perms: FilePerms,
        open_mode: OpenMode,
        allow_blocking_current_thread: bool,
    ) -> Self {
        Dir {
            dir: Arc::new(dir),
            perms,
            file_perms,
            open_mode,
            allow_blocking_current_thread,
        }
    }

    /// Execute the blocking `body` function.
    ///
    /// Depending on how the WasiCtx was configured, the body may either be:
    /// - Executed directly on the current thread. In this case the `async`
    ///   signature of this method is effectively a lie and the returned
    ///   Future will always be immediately Ready. Or:
    /// - Spawned on a background thread using [`tokio::task::spawn_blocking`]
    ///   and immediately awaited.
    ///
    /// Intentionally blocking the executor thread might seem unorthodox, but is
    /// not actually a problem for specific workloads. See:
    /// - [`crate::WasiCtxBuilder::allow_blocking_current_thread`]
    /// - [Poor performance of wasmtime file I/O maybe because tokio](https://github.com/bytecodealliance/wasmtime/issues/7973)
    /// - [Implement opt-in for enabling WASI to block the current thread](https://github.com/bytecodealliance/wasmtime/pull/8190)
    pub(crate) async fn run_blocking<F, R>(&self, body: F) -> R
    where
        F: FnOnce(&std::fs::File) -> R + Send + 'static,
        R: Send + 'static,
    {
        if self.allow_blocking_current_thread {
            body(&self.dir)
        } else {
            let d = self.dir.clone();
            spawn_blocking(move || body(&d)).await
        }
    }

    /// Returns reference to the underlying directory handle.
    #[cfg(feature = "p3")]
    pub(crate) fn as_dir(&self) -> &Arc<std::fs::File> {
        &self.dir
    }

    pub(crate) async fn create_directory_at(&self, path: String) -> Result<(), ErrorCode> {
        if !self.perms.contains(DirPerms::MUTATE) {
            return Err(ErrorCode::NotPermitted);
        }
        self.run_blocking(move |d| {
            cap_primitives::fs::create_dir(d, path.as_ref(), &DirOptions::new())
        })
        .await?;
        Ok(())
    }

    pub(crate) async fn stat_at(
        &self,
        path_flags: PathFlags,
        path: String,
    ) -> Result<DescriptorStat, ErrorCode> {
        if !self.perms.contains(DirPerms::READ) {
            return Err(ErrorCode::NotPermitted);
        }

        let follow = if path_flags.contains(PathFlags::SYMLINK_FOLLOW) {
            FollowSymlinks::Yes
        } else {
            FollowSymlinks::No
        };
        let ret = self
            .run_blocking(move |d| sys::stat_at(d, path.as_ref(), follow))
            .await?;
        Ok(ret)
    }

    pub(crate) async fn set_times_at(
        &self,
        path_flags: PathFlags,
        path: String,
        atim: Option<SystemTime>,
        mtim: Option<SystemTime>,
    ) -> Result<(), ErrorCode> {
        if !self.perms.contains(DirPerms::MUTATE) {
            return Err(ErrorCode::NotPermitted);
        }
        let atim =
            atim.map(|t| SystemTimeSpec::Absolute(cap_primitives::time::SystemTime::from_std(t)));
        let mtim =
            mtim.map(|t| SystemTimeSpec::Absolute(cap_primitives::time::SystemTime::from_std(t)));
        if path_flags.contains(PathFlags::SYMLINK_FOLLOW) {
            self.run_blocking(move |d| cap_primitives::fs::set_times(d, path.as_ref(), atim, mtim))
                .await?;
        } else {
            self.run_blocking(move |d| {
                cap_primitives::fs::set_times_nofollow(d, path.as_ref(), atim, mtim)
            })
            .await?;
        }
        Ok(())
    }

    pub(crate) async fn link_at(
        &self,
        old_path_flags: PathFlags,
        old_path: String,
        new_dir: &Self,
        new_path: String,
    ) -> Result<(), ErrorCode> {
        if !self.perms.contains(DirPerms::MUTATE) {
            return Err(ErrorCode::NotPermitted);
        }
        if !new_dir.perms.contains(DirPerms::MUTATE) {
            return Err(ErrorCode::NotPermitted);
        }
        if old_path_flags.contains(PathFlags::SYMLINK_FOLLOW) {
            return Err(ErrorCode::Invalid);
        }
        if self.perms != new_dir.perms || self.file_perms != new_dir.file_perms {
            return Err(ErrorCode::NotPermitted);
        }
        let new_dir_handle = Arc::clone(&new_dir.dir);
        self.run_blocking(move |d| {
            cap_primitives::fs::hard_link(d, old_path.as_ref(), &new_dir_handle, new_path.as_ref())
        })
        .await?;
        Ok(())
    }

    pub(crate) async fn open_at(
        &self,
        path_flags: PathFlags,
        path: String,
        oflags: OpenFlags,
        flags: DescriptorFlags,
        allow_blocking_current_thread: bool,
    ) -> Result<Descriptor, ErrorCode> {
        if !self.perms.contains(DirPerms::READ) {
            return Err(ErrorCode::NotPermitted);
        }

        if !self.perms.contains(DirPerms::MUTATE) {
            if oflags.contains(OpenFlags::CREATE) || oflags.contains(OpenFlags::TRUNCATE) {
                return Err(ErrorCode::NotPermitted);
            }
            if flags.contains(DescriptorFlags::WRITE) {
                return Err(ErrorCode::NotPermitted);
            }
        }

        // Track whether we are creating file, for permission check:
        let mut create = false;
        // Track open mode, for permission check and recording in created descriptor:
        let mut open_mode = OpenMode::empty();
        // Construct the OpenOptions to give the OS:
        let mut opts = OpenOptions::new();
        sys::maybe_dir(&mut opts);

        if oflags.contains(OpenFlags::CREATE) {
            if oflags.contains(OpenFlags::EXCLUSIVE) {
                opts.create_new(true);
            } else {
                opts.create(true);
            }
            create = true;
            opts.write(true);
            open_mode |= OpenMode::WRITE;
        }

        if oflags.contains(OpenFlags::TRUNCATE) {
            opts.truncate(true).write(true);
            open_mode |= OpenMode::WRITE;
        }
        if flags.contains(DescriptorFlags::READ) {
            opts.read(true);
            open_mode |= OpenMode::READ;
        }
        if flags.contains(DescriptorFlags::WRITE) {
            opts.write(true);
            open_mode |= OpenMode::WRITE;
        } else {
            // If not opened write, open read. This way the OS lets us open
            // the file, but we can use perms to reject use of the file later.
            opts.read(true);
            open_mode |= OpenMode::READ;
        }

        // Note that this is intentionally scoped to a separate block to
        // minimize the surface area that is depended on by cap-fs-ext. Ideally
        // the underlying functionality in `cap-primitives` would get exposed,
        // but that'll require an upstream PR.
        {
            use cap_fs_ext_avoid_using_this::OpenOptionsFollowExt;
            if path_flags.contains(PathFlags::SYMLINK_FOLLOW) {
                opts.follow(FollowSymlinks::Yes);
            } else {
                opts.follow(FollowSymlinks::No);
            }
        }

        // These flags are not yet supported in cap-primitives:
        if flags.contains(DescriptorFlags::FILE_INTEGRITY_SYNC)
            || flags.contains(DescriptorFlags::DATA_INTEGRITY_SYNC)
            || flags.contains(DescriptorFlags::REQUESTED_WRITE_SYNC)
        {
            return Err(ErrorCode::Unsupported);
        }

        if oflags.contains(OpenFlags::DIRECTORY) {
            if oflags.contains(OpenFlags::CREATE)
                || oflags.contains(OpenFlags::EXCLUSIVE)
                || oflags.contains(OpenFlags::TRUNCATE)
            {
                return Err(ErrorCode::Invalid);
            }
        }

        // Now enforce this WasiCtx's permissions before letting the OS have
        // its shot:
        if !self.perms.contains(DirPerms::MUTATE) && create {
            return Err(ErrorCode::NotPermitted);
        }
        if !self.file_perms.contains(FilePerms::WRITE) && open_mode.contains(OpenMode::WRITE) {
            return Err(ErrorCode::NotPermitted);
        }

        // Represents each possible outcome from the spawn_blocking operation.
        // This makes sure we don't have to give spawn_blocking any way to
        // manipulate the table.
        enum OpenResult {
            Dir(std::fs::File),
            File(std::fs::File),
            NotDir,
        }

        let opened = self
            .run_blocking::<_, std::io::Result<OpenResult>>(move |d| {
                let opened = cap_primitives::fs::open(d, path.as_ref(), &opts)?;
                if Metadata::from_file(&opened)?.is_dir() {
                    Ok(OpenResult::Dir(opened))
                } else if oflags.contains(OpenFlags::DIRECTORY) {
                    Ok(OpenResult::NotDir)
                } else {
                    Ok(OpenResult::File(opened))
                }
            })
            .await?;

        match opened {
            // Paper over a divergence between Windows and POSIX, where
            // POSIX returns EISDIR if you open a directory with the
            // WRITE flag: https://pubs.opengroup.org/onlinepubs/9699919799/functions/open.html#:~:text=EISDIR
            #[cfg(windows)]
            OpenResult::Dir(_) if flags.contains(DescriptorFlags::WRITE) => {
                Err(ErrorCode::IsDirectory)
            }

            OpenResult::Dir(dir) => Ok(Descriptor::Dir(Dir::new(
                dir,
                self.perms,
                self.file_perms,
                open_mode,
                allow_blocking_current_thread,
            ))),

            OpenResult::File(file) => Ok(Descriptor::File(File::new(
                file,
                self.file_perms,
                open_mode,
                allow_blocking_current_thread,
            ))),

            OpenResult::NotDir => Err(ErrorCode::NotDirectory),
        }
    }

    pub(crate) async fn readlink_at(&self, path: String) -> Result<String, ErrorCode> {
        if !self.perms.contains(DirPerms::READ) {
            return Err(ErrorCode::NotPermitted);
        }
        let link = self
            .run_blocking(move |d| cap_primitives::fs::read_link(d, path.as_ref()))
            .await?;
        link.into_os_string()
            .into_string()
            .or(Err(ErrorCode::IllegalByteSequence))
    }

    pub(crate) async fn remove_directory_at(&self, path: String) -> Result<(), ErrorCode> {
        if !self.perms.contains(DirPerms::MUTATE) {
            return Err(ErrorCode::NotPermitted);
        }
        self.run_blocking(move |d| cap_primitives::fs::remove_dir(d, path.as_ref()))
            .await?;
        Ok(())
    }

    pub(crate) async fn rename_at(
        &self,
        old_path: String,
        new_dir: &Self,
        new_path: String,
    ) -> Result<(), ErrorCode> {
        if !self.perms.contains(DirPerms::MUTATE) {
            return Err(ErrorCode::NotPermitted);
        }
        if !new_dir.perms.contains(DirPerms::MUTATE) {
            return Err(ErrorCode::NotPermitted);
        }
        if self.perms != new_dir.perms || self.file_perms != new_dir.file_perms {
            return Err(ErrorCode::NotPermitted);
        }
        let new_dir_handle = Arc::clone(&new_dir.dir);
        self.run_blocking(move |d| {
            cap_primitives::fs::rename(d, old_path.as_ref(), &new_dir_handle, new_path.as_ref())
        })
        .await?;
        Ok(())
    }

    pub(crate) async fn symlink_at(
        &self,
        src_path: String,
        dest_path: String,
    ) -> Result<(), ErrorCode> {
        if !self.perms.contains(DirPerms::MUTATE) {
            return Err(ErrorCode::NotPermitted);
        }
        self.run_blocking(move |d| sys::symlink(src_path.as_ref(), d, dest_path.as_ref()))
            .await?;
        Ok(())
    }

    pub(crate) async fn unlink_file_at(&self, path: String) -> Result<(), ErrorCode> {
        if !self.perms.contains(DirPerms::MUTATE) {
            return Err(ErrorCode::NotPermitted);
        }
        self.run_blocking(move |d| sys::remove_file_or_symlink(d, path.as_ref()))
            .await?;
        Ok(())
    }

    pub(crate) async fn metadata_hash_at(
        &self,
        path_flags: PathFlags,
        path: String,
    ) -> Result<MetadataHashValue, ErrorCode> {
        // No permissions check on metadata: if dir opened, allowed to stat it
        let follow = if path_flags.contains(PathFlags::SYMLINK_FOLLOW) {
            FollowSymlinks::Yes
        } else {
            FollowSymlinks::No
        };
        let hash = self
            .run_blocking(move |d| sys::metadata_hash_at(d, path.as_ref(), follow))
            .await?;
        Ok(hash)
    }
}

impl WasiFilesystemCtxView<'_> {
    pub(crate) fn get_directories(
        &mut self,
    ) -> wasmtime::Result<Vec<(Resource<Descriptor>, String)>> {
        let preopens = self.ctx.preopens.clone();
        let mut results = Vec::with_capacity(preopens.len());
        for (dir, name) in preopens {
            let fd = self
                .table
                .push(Descriptor::Dir(dir))
                .with_context(|| format!("failed to push preopen {name}"))?;
            results.push((fd, name));
        }
        Ok(results)
    }
}
