use crate::preview2::filesystem::TableFsExt;
use crate::preview2::preview2::filesystem::TableReaddirExt;
use crate::preview2::{wasi, TableError, WasiView};

use core::borrow::Borrow;
use core::cell::Cell;
use core::mem::{size_of, size_of_val};
use core::ops::{Deref, DerefMut};
use core::slice;
use core::sync::atomic::{AtomicU64, Ordering};

use std::collections::BTreeMap;
use std::sync::Arc;

use anyhow::{anyhow, bail, Context};
use wiggle::tracing::instrument;
use wiggle::{GuestPtr, GuestSliceMut, GuestStrCow, GuestType};

#[derive(Clone, Debug)]
struct File {
    /// The handle to the preview2 descriptor that this file is referencing.
    fd: wasi::filesystem::Descriptor,

    /// The current-position pointer.
    position: Arc<AtomicU64>,

    /// In append mode, all writes append to the file.
    append: bool,

    /// In blocking mode, read and write calls dispatch to blocking_read and
    /// blocking_write on the underlying streams. When false, read and write
    /// dispatch to stream's plain read and write.
    blocking: bool,
}

#[derive(Clone, Debug)]
enum Descriptor {
    Stdin(wasi::preopens::InputStream),
    Stdout(wasi::preopens::OutputStream),
    Stderr(wasi::preopens::OutputStream),
    PreopenDirectory((wasi::filesystem::Descriptor, String)),
    File(File),
}

#[derive(Debug, Default)]
pub struct WasiPreview1Adapter {
    descriptors: Option<Descriptors>,
}

#[derive(Debug, Default)]
struct Descriptors {
    used: BTreeMap<u32, Descriptor>,
    free: Vec<u32>,
}

impl Deref for Descriptors {
    type Target = BTreeMap<u32, Descriptor>;

    fn deref(&self) -> &Self::Target {
        &self.used
    }
}

impl DerefMut for Descriptors {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.used
    }
}

impl Descriptors {
    /// Initializes [Self] using `preopens`
    async fn new(
        preopens: &mut (impl wasi::preopens::Host
                  + wasi::stdin::Host
                  + wasi::stdout::Host
                  + wasi::stderr::Host
                  + ?Sized),
    ) -> Result<Self, types::Error> {
        let stdin = preopens
            .get_stdin()
            .await
            .context("failed to call `get-stdin`")
            .map_err(types::Error::trap)?;
        let stdout = preopens
            .get_stdout()
            .await
            .context("failed to call `get-stdout`")
            .map_err(types::Error::trap)?;
        let stderr = preopens
            .get_stderr()
            .await
            .context("failed to call `get-stderr`")
            .map_err(types::Error::trap)?;
        let directories = preopens
            .get_directories()
            .await
            .context("failed to call `get-directories`")
            .map_err(types::Error::trap)?;

        let mut descriptors = Self::default();
        descriptors.push(Descriptor::Stdin(stdin))?;
        descriptors.push(Descriptor::Stdout(stdout))?;
        descriptors.push(Descriptor::Stderr(stderr))?;
        for dir in directories {
            descriptors.push(Descriptor::PreopenDirectory(dir))?;
        }
        Ok(descriptors)
    }

    /// Returns next descriptor number, which was never assigned
    fn unused(&self) -> ErrnoResult<u32> {
        match self.last_key_value() {
            Some((fd, _)) => {
                if let Some(fd) = fd.checked_add(1) {
                    return Ok(fd);
                }
                if self.len() == u32::MAX as usize {
                    return Err(types::Errno::Loop);
                }
                // TODO: Optimize
                Ok((0..u32::MAX)
                    .rev()
                    .find(|fd| !self.contains_key(fd))
                    .expect("failed to find an unused file descriptor"))
            }
            None => Ok(0),
        }
    }

    /// Removes the [Descriptor] corresponding to `fd`
    fn remove(&mut self, fd: types::Fd) -> Option<Descriptor> {
        let fd = fd.into();
        let desc = self.used.remove(&fd)?;
        self.free.push(fd);
        Some(desc)
    }

    /// Pushes the [Descriptor] returning corresponding number.
    /// This operation will try to reuse numbers previously removed via [`Self::remove`]
    /// and rely on [`Self::unused`] if no free numbers are recorded
    fn push(&mut self, desc: Descriptor) -> ErrnoResult<u32> {
        let fd = if let Some(fd) = self.free.pop() {
            fd
        } else {
            self.unused()?
        };
        assert!(self.insert(fd, desc).is_none());
        Ok(fd)
    }

    /// Like [Self::push], but for [`File`]
    fn push_file(&mut self, file: File) -> ErrnoResult<u32> {
        self.push(Descriptor::File(file))
    }
}

impl WasiPreview1Adapter {
    pub fn new() -> Self {
        Self::default()
    }
}

// Any context that needs to support preview 1 will impl this trait. They can
// construct the needed member with WasiPreview1Adapter::new().
pub trait WasiPreview1View: Send + Sync + WasiView {
    fn adapter(&self) -> &WasiPreview1Adapter;
    fn adapter_mut(&mut self) -> &mut WasiPreview1Adapter;
}

/// A mutably-borrowed [`WasiPreview1View`] implementation, which provides access to the stored
/// state. It can be thought of as an in-flight [`WasiPreview1Adapter`] transaction, all
/// changes will be recorded in the underlying [`WasiPreview1Adapter`] returned by
/// [`WasiPreview1View::adapter_mut`] on [`Drop`] of this struct.
// NOTE: This exists for the most part just due to the fact that `bindgen` generates methods with
// `&mut self` receivers and so this struct lets us extend the lifetime of the `&mut self` borrow
// of the [`WasiPreview1View`] to provide means to return mutably and immutably borrowed [`Descriptors`]
// without having to rely on something like `Arc<Mutex<Descriptors>>`, while also being able to
// call methods like [`TableFsExt::is_file`] and hiding complexity from preview1 method implementations.
struct Transaction<'a, T: WasiPreview1View + ?Sized> {
    view: &'a mut T,
    descriptors: Cell<Descriptors>,
}

impl<T: WasiPreview1View + ?Sized> Drop for Transaction<'_, T> {
    /// Record changes in the [`WasiPreview1Adapter`] returned by [`WasiPreview1View::adapter_mut`]
    fn drop(&mut self) {
        let descriptors = self.descriptors.take();
        self.view.adapter_mut().descriptors = Some(descriptors);
    }
}

impl<T: WasiPreview1View + ?Sized> Transaction<'_, T> {
    /// Borrows [`Descriptor`] corresponding to `fd`.
    ///
    /// # Errors
    ///
    /// Returns [`types::Errno::Badf`] if no [`Descriptor`] is found
    fn get_descriptor(&mut self, fd: types::Fd) -> ErrnoResult<&Descriptor> {
        let fd = fd.into();
        self.descriptors
            .get_mut()
            .get(&fd)
            .ok_or(types::Errno::Badf)
    }

    /// Borrows [`File`] corresponding to `fd`
    /// if it describes a [`Descriptor::File`] of [`crate::preview2::filesystem::File`] type
    fn get_file(&mut self, fd: types::Fd) -> ErrnoResult<&File> {
        let fd = fd.into();
        match self.descriptors.get_mut().get(&fd) {
            Some(Descriptor::File(file @ File { fd, .. })) if self.view.table().is_file(*fd) => {
                Ok(file)
            }
            _ => Err(types::Errno::Badf),
        }
    }

    /// Mutably borrows [`File`] corresponding to `fd`
    /// if it describes a [`Descriptor::File`] of [`crate::preview2::filesystem::File`] type
    fn get_file_mut(&mut self, fd: types::Fd) -> ErrnoResult<&mut File> {
        let fd = fd.into();
        match self.descriptors.get_mut().get_mut(&fd) {
            Some(Descriptor::File(file)) if self.view.table().is_file(file.fd) => Ok(file),
            _ => Err(types::Errno::Badf),
        }
    }

    /// Borrows [`File`] corresponding to `fd`
    /// if it describes a [`Descriptor::File`] of [`crate::preview2::filesystem::File`] type.
    ///
    /// # Errors
    ///
    /// Returns [`types::Errno::Spipe`] if the descriptor corresponds to stdio
    fn get_seekable(&mut self, fd: types::Fd) -> ErrnoResult<&File> {
        let fd = fd.into();
        match self.descriptors.get_mut().get(&fd) {
            Some(Descriptor::File(file @ File { fd, .. })) if self.view.table().is_file(*fd) => {
                Ok(file)
            }
            Some(Descriptor::Stdin(..) | Descriptor::Stdout(..) | Descriptor::Stderr(..)) => {
                // NOTE: legacy implementation returns SPIPE here
                Err(types::Errno::Spipe)
            }
            _ => Err(types::Errno::Badf),
        }
    }

    /// Returns [`wasi::filesystem::Descriptor`] corresponding to `fd`
    fn get_fd(&mut self, fd: types::Fd) -> ErrnoResult<wasi::filesystem::Descriptor> {
        match self.get_descriptor(fd)? {
            Descriptor::File(File { fd, .. }) => Ok(*fd),
            Descriptor::PreopenDirectory((fd, _)) => Ok(*fd),
            Descriptor::Stdin(stream) => Ok(*stream),
            Descriptor::Stdout(stream) | Descriptor::Stderr(stream) => Ok(*stream),
        }
    }

    /// Returns [`wasi::filesystem::Descriptor`] corresponding to `fd`
    /// if it describes a [`Descriptor::File`] of [`crate::preview2::filesystem::File`] type
    fn get_file_fd(&mut self, fd: types::Fd) -> ErrnoResult<wasi::filesystem::Descriptor> {
        self.get_file(fd).map(|File { fd, .. }| *fd)
    }

    /// Returns [`wasi::filesystem::Descriptor`] corresponding to `fd`
    /// if it describes a [`Descriptor::File`] or [`Descriptor::PreopenDirectory`]
    /// of [`crate::preview2::filesystem::Dir`] type
    fn get_dir_fd(&mut self, fd: types::Fd) -> ErrnoResult<wasi::filesystem::Descriptor> {
        let fd = fd.into();
        match self.descriptors.get_mut().get(&fd) {
            Some(Descriptor::File(File { fd, .. })) if self.view.table().is_dir(*fd) => Ok(*fd),
            Some(Descriptor::PreopenDirectory((fd, _))) => Ok(*fd),
            _ => Err(types::Errno::Badf),
        }
    }
}

#[wiggle::async_trait]
trait WasiPreview1ViewExt:
    WasiPreview1View
    + wasi::preopens::Host
    + wasi::stdin::Host
    + wasi::stdout::Host
    + wasi::stderr::Host
{
    /// Lazily initializes [`WasiPreview1Adapter`] returned by [`WasiPreview1View::adapter_mut`]
    /// and returns [`Transaction`] on success
    async fn transact(&mut self) -> Result<Transaction<'_, Self>, types::Error> {
        let descriptors = if let Some(descriptors) = self.adapter_mut().descriptors.take() {
            descriptors
        } else {
            Descriptors::new(self).await?
        }
        .into();
        Ok(Transaction {
            view: self,
            descriptors,
        })
    }

    /// Lazily initializes [`WasiPreview1Adapter`] returned by [`WasiPreview1View::adapter_mut`]
    /// and returns [`wasi::filesystem::Descriptor`] corresponding to `fd`
    async fn get_fd(
        &mut self,
        fd: types::Fd,
    ) -> Result<wasi::filesystem::Descriptor, types::Error> {
        let mut st = self.transact().await?;
        let fd = st.get_fd(fd)?;
        Ok(fd)
    }

    /// Lazily initializes [`WasiPreview1Adapter`] returned by [`WasiPreview1View::adapter_mut`]
    /// and returns [`wasi::filesystem::Descriptor`] corresponding to `fd`
    /// if it describes a [`Descriptor::File`] of [`crate::preview2::filesystem::File`] type
    async fn get_file_fd(
        &mut self,
        fd: types::Fd,
    ) -> Result<wasi::filesystem::Descriptor, types::Error> {
        let mut st = self.transact().await?;
        let fd = st.get_file_fd(fd)?;
        Ok(fd)
    }

    /// Lazily initializes [`WasiPreview1Adapter`] returned by [`WasiPreview1View::adapter_mut`]
    /// and returns [`wasi::filesystem::Descriptor`] corresponding to `fd`
    /// if it describes a [`Descriptor::File`] or [`Descriptor::PreopenDirectory`]
    /// of [`crate::preview2::filesystem::Dir`] type
    async fn get_dir_fd(
        &mut self,
        fd: types::Fd,
    ) -> Result<wasi::filesystem::Descriptor, types::Error> {
        let mut st = self.transact().await?;
        let fd = st.get_dir_fd(fd)?;
        Ok(fd)
    }
}

impl<T: WasiPreview1View + wasi::preopens::Host> WasiPreview1ViewExt for T {}

pub fn add_to_linker<
    T: WasiPreview1View
        + wasi::environment::Host
        + wasi::exit::Host
        + wasi::filesystem::Host
        + wasi::monotonic_clock::Host
        + wasi::poll::Host
        + wasi::preopens::Host
        + wasi::random::Host
        + wasi::streams::Host
        + wasi::wall_clock::Host,
>(
    linker: &mut wasmtime::Linker<T>,
) -> anyhow::Result<()> {
    wasi_snapshot_preview1::add_to_linker(linker, |t| t)
}

// Generate the wasi_snapshot_preview1::WasiSnapshotPreview1 trait,
// and the module types.
// None of the generated modules, traits, or types should be used externally
// to this module.
wiggle::from_witx!({
    witx: ["$CARGO_MANIFEST_DIR/witx/wasi_snapshot_preview1.witx"],
    errors: { errno => trappable Error },
    async: *,
});

impl wiggle::GuestErrorType for types::Errno {
    fn success() -> Self {
        Self::Success
    }
}

fn systimespec(
    set: bool,
    ts: types::Timestamp,
    now: bool,
) -> ErrnoResult<wasi::filesystem::NewTimestamp> {
    if set && now {
        Err(types::Errno::Inval)
    } else if set {
        Ok(wasi::filesystem::NewTimestamp::Timestamp(
            wasi::filesystem::Datetime {
                seconds: ts / 1_000_000_000,
                nanoseconds: (ts % 1_000_000_000) as _,
            },
        ))
    } else if now {
        Ok(wasi::filesystem::NewTimestamp::Now)
    } else {
        Ok(wasi::filesystem::NewTimestamp::NoChange)
    }
}

impl TryFrom<wasi::wall_clock::Datetime> for types::Timestamp {
    type Error = types::Errno;

    fn try_from(
        wasi::wall_clock::Datetime {
            seconds,
            nanoseconds,
        }: wasi::wall_clock::Datetime,
    ) -> Result<Self, Self::Error> {
        types::Timestamp::from(seconds)
            .checked_mul(1_000_000_000)
            .and_then(|ns| ns.checked_add(nanoseconds.into()))
            .ok_or(types::Errno::Overflow)
    }
}

impl From<types::Lookupflags> for wasi::filesystem::PathFlags {
    fn from(flags: types::Lookupflags) -> Self {
        if flags.contains(types::Lookupflags::SYMLINK_FOLLOW) {
            wasi::filesystem::PathFlags::SYMLINK_FOLLOW
        } else {
            wasi::filesystem::PathFlags::empty()
        }
    }
}

impl From<types::Oflags> for wasi::filesystem::OpenFlags {
    fn from(flags: types::Oflags) -> Self {
        let mut out = wasi::filesystem::OpenFlags::empty();
        if flags.contains(types::Oflags::CREAT) {
            out |= wasi::filesystem::OpenFlags::CREATE;
        }
        if flags.contains(types::Oflags::DIRECTORY) {
            out |= wasi::filesystem::OpenFlags::DIRECTORY;
        }
        if flags.contains(types::Oflags::EXCL) {
            out |= wasi::filesystem::OpenFlags::EXCLUSIVE;
        }
        if flags.contains(types::Oflags::TRUNC) {
            out |= wasi::filesystem::OpenFlags::TRUNCATE;
        }
        out
    }
}

impl From<types::Advice> for wasi::filesystem::Advice {
    fn from(advice: types::Advice) -> Self {
        match advice {
            types::Advice::Normal => wasi::filesystem::Advice::Normal,
            types::Advice::Sequential => wasi::filesystem::Advice::Sequential,
            types::Advice::Random => wasi::filesystem::Advice::Random,
            types::Advice::Willneed => wasi::filesystem::Advice::WillNeed,
            types::Advice::Dontneed => wasi::filesystem::Advice::DontNeed,
            types::Advice::Noreuse => wasi::filesystem::Advice::NoReuse,
        }
    }
}

impl TryFrom<wasi::filesystem::DescriptorType> for types::Filetype {
    type Error = anyhow::Error;

    fn try_from(ty: wasi::filesystem::DescriptorType) -> Result<Self, Self::Error> {
        match ty {
            wasi::filesystem::DescriptorType::RegularFile => Ok(types::Filetype::RegularFile),
            wasi::filesystem::DescriptorType::Directory => Ok(types::Filetype::Directory),
            wasi::filesystem::DescriptorType::BlockDevice => Ok(types::Filetype::BlockDevice),
            wasi::filesystem::DescriptorType::CharacterDevice => {
                Ok(types::Filetype::CharacterDevice)
            }
            // preview1 never had a FIFO code.
            wasi::filesystem::DescriptorType::Fifo => Ok(types::Filetype::Unknown),
            // TODO: Add a way to disginguish between FILETYPE_SOCKET_STREAM and
            // FILETYPE_SOCKET_DGRAM.
            wasi::filesystem::DescriptorType::Socket => {
                bail!("sockets are not currently supported")
            }
            wasi::filesystem::DescriptorType::SymbolicLink => Ok(types::Filetype::SymbolicLink),
            wasi::filesystem::DescriptorType::Unknown => Ok(types::Filetype::Unknown),
        }
    }
}

impl From<wasi::filesystem::ErrorCode> for types::Errno {
    fn from(code: wasi::filesystem::ErrorCode) -> Self {
        match code {
            wasi::filesystem::ErrorCode::Access => types::Errno::Acces,
            wasi::filesystem::ErrorCode::WouldBlock => types::Errno::Again,
            wasi::filesystem::ErrorCode::Already => types::Errno::Already,
            wasi::filesystem::ErrorCode::BadDescriptor => types::Errno::Badf,
            wasi::filesystem::ErrorCode::Busy => types::Errno::Busy,
            wasi::filesystem::ErrorCode::Deadlock => types::Errno::Deadlk,
            wasi::filesystem::ErrorCode::Quota => types::Errno::Dquot,
            wasi::filesystem::ErrorCode::Exist => types::Errno::Exist,
            wasi::filesystem::ErrorCode::FileTooLarge => types::Errno::Fbig,
            wasi::filesystem::ErrorCode::IllegalByteSequence => types::Errno::Ilseq,
            wasi::filesystem::ErrorCode::InProgress => types::Errno::Inprogress,
            wasi::filesystem::ErrorCode::Interrupted => types::Errno::Intr,
            wasi::filesystem::ErrorCode::Invalid => types::Errno::Inval,
            wasi::filesystem::ErrorCode::Io => types::Errno::Io,
            wasi::filesystem::ErrorCode::IsDirectory => types::Errno::Isdir,
            wasi::filesystem::ErrorCode::Loop => types::Errno::Loop,
            wasi::filesystem::ErrorCode::TooManyLinks => types::Errno::Mlink,
            wasi::filesystem::ErrorCode::MessageSize => types::Errno::Msgsize,
            wasi::filesystem::ErrorCode::NameTooLong => types::Errno::Nametoolong,
            wasi::filesystem::ErrorCode::NoDevice => types::Errno::Nodev,
            wasi::filesystem::ErrorCode::NoEntry => types::Errno::Noent,
            wasi::filesystem::ErrorCode::NoLock => types::Errno::Nolck,
            wasi::filesystem::ErrorCode::InsufficientMemory => types::Errno::Nomem,
            wasi::filesystem::ErrorCode::InsufficientSpace => types::Errno::Nospc,
            wasi::filesystem::ErrorCode::Unsupported => types::Errno::Notsup,
            wasi::filesystem::ErrorCode::NotDirectory => types::Errno::Notdir,
            wasi::filesystem::ErrorCode::NotEmpty => types::Errno::Notempty,
            wasi::filesystem::ErrorCode::NotRecoverable => types::Errno::Notrecoverable,
            wasi::filesystem::ErrorCode::NoTty => types::Errno::Notty,
            wasi::filesystem::ErrorCode::NoSuchDevice => types::Errno::Nxio,
            wasi::filesystem::ErrorCode::Overflow => types::Errno::Overflow,
            wasi::filesystem::ErrorCode::NotPermitted => types::Errno::Perm,
            wasi::filesystem::ErrorCode::Pipe => types::Errno::Pipe,
            wasi::filesystem::ErrorCode::ReadOnly => types::Errno::Rofs,
            wasi::filesystem::ErrorCode::InvalidSeek => types::Errno::Spipe,
            wasi::filesystem::ErrorCode::TextFileBusy => types::Errno::Txtbsy,
            wasi::filesystem::ErrorCode::CrossDevice => types::Errno::Xdev,
        }
    }
}

impl From<wasi::filesystem::ErrorCode> for types::Error {
    fn from(code: wasi::filesystem::ErrorCode) -> Self {
        types::Errno::from(code).into()
    }
}

impl TryFrom<wasi::filesystem::Error> for types::Errno {
    type Error = anyhow::Error;

    fn try_from(err: wasi::filesystem::Error) -> Result<Self, Self::Error> {
        match err.downcast() {
            Ok(code) => Ok(code.into()),
            Err(e) => Err(e),
        }
    }
}

impl TryFrom<wasi::filesystem::Error> for types::Error {
    type Error = anyhow::Error;

    fn try_from(err: wasi::filesystem::Error) -> Result<Self, Self::Error> {
        match err.downcast() {
            Ok(code) => Ok(code.into()),
            Err(e) => Err(e),
        }
    }
}

impl From<TableError> for types::Errno {
    fn from(err: TableError) -> Self {
        match err {
            TableError::Full => types::Errno::Nomem,
            TableError::NotPresent | TableError::WrongType => types::Errno::Badf,
        }
    }
}

impl From<TableError> for types::Error {
    fn from(err: TableError) -> Self {
        types::Errno::from(err).into()
    }
}

type ErrnoResult<T> = Result<T, types::Errno>;

fn write_bytes<'a>(
    ptr: impl Borrow<GuestPtr<'a, u8>>,
    buf: impl AsRef<[u8]>,
) -> ErrnoResult<GuestPtr<'a, u8>> {
    // NOTE: legacy implementation always returns Inval errno

    let buf = buf.as_ref();
    let len = buf.len().try_into().or(Err(types::Errno::Inval))?;

    let ptr = ptr.borrow();
    ptr.as_array(len)
        .copy_from_slice(buf)
        .or(Err(types::Errno::Inval))?;
    ptr.add(len).or(Err(types::Errno::Inval))
}

fn write_byte<'a>(ptr: impl Borrow<GuestPtr<'a, u8>>, byte: u8) -> ErrnoResult<GuestPtr<'a, u8>> {
    let ptr = ptr.borrow();
    ptr.write(byte).or(Err(types::Errno::Inval))?;
    ptr.add(1).or(Err(types::Errno::Inval))
}

fn read_str<'a>(ptr: impl Borrow<GuestPtr<'a, str>>) -> ErrnoResult<GuestStrCow<'a>> {
    // NOTE: legacy implementation always returns Inval errno
    ptr.borrow().as_cow().or(Err(types::Errno::Inval))
}

fn read_string<'a>(ptr: impl Borrow<GuestPtr<'a, str>>) -> ErrnoResult<String> {
    read_str(ptr).map(|s| s.to_string())
}

// Find first non-empty buffer.
fn first_non_empty_ciovec(ciovs: &types::CiovecArray<'_>) -> ErrnoResult<Option<Vec<u8>>> {
    ciovs
        .iter()
        .map(|iov| {
            let iov = iov
                .or(Err(types::Errno::Inval))?
                .read()
                .or(Err(types::Errno::Inval))?;
            if iov.buf_len == 0 {
                return Ok(None);
            }
            iov.buf
                .as_array(iov.buf_len)
                .to_vec()
                .or(Err(types::Errno::Inval))
                .map(Some)
        })
        .find_map(Result::transpose)
        .transpose()
}

// Find first non-empty buffer.
fn first_non_empty_iovec<'a>(
    iovs: &types::IovecArray<'a>,
) -> ErrnoResult<Option<GuestSliceMut<'a, u8>>> {
    iovs.iter()
        .map(|iov| {
            let iov = iov
                .or(Err(types::Errno::Inval))?
                .read()
                .or(Err(types::Errno::Inval))?;
            if iov.buf_len == 0 {
                return Ok(None);
            }
            iov.buf
                .as_array(iov.buf_len)
                .as_slice_mut()
                .map_err(|_| types::Errno::Inval)
        })
        .find_map(Result::transpose)
        .transpose()
}

// Implement the WasiSnapshotPreview1 trait using only the traits that are
// required for T, i.e., in terms of the preview 2 wit interface, and state
// stored in the WasiPreview1Adapter struct.
#[wiggle::async_trait]
impl<
        T: WasiPreview1View
            + wasi::environment::Host
            + wasi::exit::Host
            + wasi::filesystem::Host
            + wasi::monotonic_clock::Host
            + wasi::poll::Host
            + wasi::preopens::Host
            + wasi::random::Host
            + wasi::streams::Host
            + wasi::wall_clock::Host,
    > wasi_snapshot_preview1::WasiSnapshotPreview1 for T
{
    #[instrument(skip(self))]
    async fn args_get<'b>(
        &mut self,
        argv: &GuestPtr<'b, GuestPtr<'b, u8>>,
        argv_buf: &GuestPtr<'b, u8>,
    ) -> Result<(), types::Error> {
        self.get_arguments()
            .await
            .context("failed to call `get-arguments`")
            .map_err(types::Error::trap)?
            .into_iter()
            .try_fold(
                (*argv, *argv_buf),
                |(argv, argv_buf), arg| -> ErrnoResult<_> {
                    // NOTE: legacy implementation always returns Inval errno

                    argv.write(argv_buf).map_err(|_| types::Errno::Inval)?;
                    let argv = argv.add(1).map_err(|_| types::Errno::Inval)?;

                    let argv_buf = write_bytes(argv_buf, arg)?;
                    let argv_buf = write_byte(argv_buf, 0)?;

                    Ok((argv, argv_buf))
                },
            )?;
        Ok(())
    }

    #[instrument(skip(self))]
    async fn args_sizes_get(&mut self) -> Result<(types::Size, types::Size), types::Error> {
        let args = self
            .get_arguments()
            .await
            .context("failed to call `get-arguments`")
            .map_err(types::Error::trap)?;
        let num = args.len().try_into().map_err(|_| types::Errno::Overflow)?;
        let len = args
            .iter()
            .map(|buf| buf.len() + 1) // Each argument is expected to be `\0` terminated.
            .sum::<usize>()
            .try_into()
            .map_err(|_| types::Errno::Overflow)?;
        Ok((num, len))
    }

    #[instrument(skip(self))]
    async fn environ_get<'b>(
        &mut self,
        environ: &GuestPtr<'b, GuestPtr<'b, u8>>,
        environ_buf: &GuestPtr<'b, u8>,
    ) -> Result<(), types::Error> {
        self.get_environment()
            .await
            .context("failed to call `get-environment`")
            .map_err(types::Error::trap)?
            .into_iter()
            .try_fold(
                (*environ, *environ_buf),
                |(environ, environ_buf), (k, v)| -> ErrnoResult<_> {
                    // NOTE: legacy implementation always returns Inval errno

                    environ
                        .write(environ_buf)
                        .map_err(|_| types::Errno::Inval)?;
                    let environ = environ.add(1).map_err(|_| types::Errno::Inval)?;

                    let environ_buf = write_bytes(environ_buf, k)?;
                    let environ_buf = write_byte(environ_buf, b'=')?;
                    let environ_buf = write_bytes(environ_buf, v)?;
                    let environ_buf = write_byte(environ_buf, 0)?;

                    Ok((environ, environ_buf))
                },
            )?;
        Ok(())
    }

    #[instrument(skip(self))]
    async fn environ_sizes_get(&mut self) -> Result<(types::Size, types::Size), types::Error> {
        let environ = self
            .get_environment()
            .await
            .context("failed to call `get-environment`")
            .map_err(types::Error::trap)?;
        let num = environ
            .len()
            .try_into()
            .map_err(|_| types::Errno::Overflow)?;
        let len = environ
            .iter()
            .map(|(k, v)| k.len() + 1 + v.len() + 1) // Key/value pairs are expected to be joined with `=`s, and terminated with `\0`s.
            .sum::<usize>()
            .try_into()
            .map_err(|_| types::Errno::Overflow)?;
        Ok((num, len))
    }

    #[instrument(skip(self))]
    async fn clock_res_get(
        &mut self,
        id: types::Clockid,
    ) -> Result<types::Timestamp, types::Error> {
        let res = match id {
            types::Clockid::Realtime => wasi::wall_clock::Host::resolution(self)
                .await
                .context("failed to call `wall_clock::resolution`")
                .map_err(types::Error::trap)?
                .try_into()?,
            types::Clockid::Monotonic => wasi::monotonic_clock::Host::resolution(self)
                .await
                .context("failed to call `monotonic_clock::resolution`")
                .map_err(types::Error::trap)?,
            types::Clockid::ProcessCputimeId | types::Clockid::ThreadCputimeId => {
                return Err(types::Errno::Badf.into())
            }
        };
        Ok(res)
    }

    #[instrument(skip(self))]
    async fn clock_time_get(
        &mut self,
        id: types::Clockid,
        _precision: types::Timestamp,
    ) -> Result<types::Timestamp, types::Error> {
        let now = match id {
            types::Clockid::Realtime => wasi::wall_clock::Host::now(self)
                .await
                .context("failed to call `wall_clock::now`")
                .map_err(types::Error::trap)?
                .try_into()?,
            types::Clockid::Monotonic => wasi::monotonic_clock::Host::now(self)
                .await
                .context("failed to call `monotonic_clock::now`")
                .map_err(types::Error::trap)?,
            types::Clockid::ProcessCputimeId | types::Clockid::ThreadCputimeId => {
                return Err(types::Errno::Badf.into())
            }
        };
        Ok(now)
    }

    #[instrument(skip(self))]
    async fn fd_advise(
        &mut self,
        fd: types::Fd,
        offset: types::Filesize,
        len: types::Filesize,
        advice: types::Advice,
    ) -> Result<(), types::Error> {
        let fd = self.get_file_fd(fd).await?;
        self.advise(fd, offset, len, advice.into())
            .await
            .map_err(|e| {
                e.try_into()
                    .context("failed to call `advise`")
                    .unwrap_or_else(types::Error::trap)
            })
    }

    /// Force the allocation of space in a file.
    /// NOTE: This is similar to `posix_fallocate` in POSIX.
    #[instrument(skip(self))]
    async fn fd_allocate(
        &mut self,
        fd: types::Fd,
        _offset: types::Filesize,
        _len: types::Filesize,
    ) -> Result<(), types::Error> {
        self.get_file_fd(fd).await?;
        Err(types::Errno::Notsup.into())
    }

    /// Close a file descriptor.
    /// NOTE: This is similar to `close` in POSIX.
    #[instrument(skip(self))]
    async fn fd_close(&mut self, fd: types::Fd) -> Result<(), types::Error> {
        let desc = self
            .transact()
            .await?
            .descriptors
            .get_mut()
            .remove(fd)
            .ok_or(types::Errno::Badf)?
            .clone();
        match desc {
            Descriptor::Stdin(stream) => self
                .drop_input_stream(stream)
                .await
                .context("failed to call `drop-input-stream`"),
            Descriptor::Stdout(stream) | Descriptor::Stderr(stream) => self
                .drop_output_stream(stream)
                .await
                .context("failed to call `drop-output-stream`"),
            Descriptor::File(File { fd, .. }) | Descriptor::PreopenDirectory((fd, _)) => self
                .drop_descriptor(fd)
                .await
                .context("failed to call `drop-descriptor`"),
        }
        .map_err(types::Error::trap)
    }

    /// Synchronize the data of a file to disk.
    /// NOTE: This is similar to `fdatasync` in POSIX.
    #[instrument(skip(self))]
    async fn fd_datasync(&mut self, fd: types::Fd) -> Result<(), types::Error> {
        let fd = self.get_file_fd(fd).await?;
        self.sync_data(fd).await.map_err(|e| {
            e.try_into()
                .context("failed to call `sync-data`")
                .unwrap_or_else(types::Error::trap)
        })
    }

    /// Get the attributes of a file descriptor.
    /// NOTE: This returns similar flags to `fsync(fd, F_GETFL)` in POSIX, as well as additional fields.
    #[instrument(skip(self))]
    async fn fd_fdstat_get(&mut self, fd: types::Fd) -> Result<types::Fdstat, types::Error> {
        let (fd, blocking, append) = match self.transact().await?.get_descriptor(fd)? {
            Descriptor::Stdin(..) => {
                let fs_rights_base = types::Rights::FD_READ;
                return Ok(types::Fdstat {
                    fs_filetype: types::Filetype::CharacterDevice,
                    fs_flags: types::Fdflags::empty(),
                    fs_rights_base,
                    fs_rights_inheriting: fs_rights_base,
                });
            }
            Descriptor::Stdout(..) | Descriptor::Stderr(..) => {
                let fs_rights_base = types::Rights::FD_WRITE;
                return Ok(types::Fdstat {
                    fs_filetype: types::Filetype::CharacterDevice,
                    fs_flags: types::Fdflags::empty(),
                    fs_rights_base,
                    fs_rights_inheriting: fs_rights_base,
                });
            }
            Descriptor::PreopenDirectory((fd, _)) => (*fd, false, false),
            Descriptor::File(File {
                fd,
                blocking,
                append,
                ..
            }) => (*fd, *blocking, *append),
        };

        // TODO: use `try_join!` to poll both futures async, unfortunately that is not currently
        // possible, because `bindgen` generates methods with `&mut self` receivers.
        let flags = self.get_flags(fd).await.map_err(|e| {
            e.try_into()
                .context("failed to call `get-flags`")
                .unwrap_or_else(types::Error::trap)
        })?;
        let fs_filetype = self
            .get_type(fd)
            .await
            .map_err(|e| {
                e.try_into()
                    .context("failed to call `get-type`")
                    .unwrap_or_else(types::Error::trap)
            })?
            .try_into()
            .map_err(types::Error::trap)?;
        let mut fs_flags = types::Fdflags::empty();
        let mut fs_rights_base = types::Rights::all();
        if !flags.contains(wasi::filesystem::DescriptorFlags::READ) {
            fs_rights_base &= !types::Rights::FD_READ;
        }
        if !flags.contains(wasi::filesystem::DescriptorFlags::WRITE) {
            fs_rights_base &= !types::Rights::FD_WRITE;
        }
        if flags.contains(wasi::filesystem::DescriptorFlags::DATA_INTEGRITY_SYNC) {
            fs_flags |= types::Fdflags::DSYNC;
        }
        if flags.contains(wasi::filesystem::DescriptorFlags::REQUESTED_WRITE_SYNC) {
            fs_flags |= types::Fdflags::RSYNC;
        }
        if flags.contains(wasi::filesystem::DescriptorFlags::FILE_INTEGRITY_SYNC) {
            fs_flags |= types::Fdflags::SYNC;
        }
        if append {
            fs_flags |= types::Fdflags::APPEND;
        }
        if !blocking {
            fs_flags |= types::Fdflags::NONBLOCK;
        }
        Ok(types::Fdstat {
            fs_filetype,
            fs_flags,
            fs_rights_base,
            fs_rights_inheriting: fs_rights_base,
        })
    }

    /// Adjust the flags associated with a file descriptor.
    /// NOTE: This is similar to `fcntl(fd, F_SETFL, flags)` in POSIX.
    #[instrument(skip(self))]
    async fn fd_fdstat_set_flags(
        &mut self,
        fd: types::Fd,
        flags: types::Fdflags,
    ) -> Result<(), types::Error> {
        let mut st = self.transact().await?;
        let File {
            append, blocking, ..
        } = st.get_file_mut(fd)?;

        // Only support changing the NONBLOCK or APPEND flags.
        if flags.contains(types::Fdflags::DSYNC)
            || flags.contains(types::Fdflags::SYNC)
            || flags.contains(types::Fdflags::RSYNC)
        {
            return Err(types::Errno::Inval.into());
        }
        *append = flags.contains(types::Fdflags::APPEND);
        *blocking = !flags.contains(types::Fdflags::NONBLOCK);
        Ok(())
    }

    /// Does not do anything if `fd` corresponds to a valid descriptor and returns `[types::Errno::Badf]` error otherwise.
    #[instrument(skip(self))]
    async fn fd_fdstat_set_rights(
        &mut self,
        fd: types::Fd,
        _fs_rights_base: types::Rights,
        _fs_rights_inheriting: types::Rights,
    ) -> Result<(), types::Error> {
        self.get_fd(fd).await?;
        Ok(())
    }

    /// Return the attributes of an open file.
    #[instrument(skip(self))]
    async fn fd_filestat_get(&mut self, fd: types::Fd) -> Result<types::Filestat, types::Error> {
        let desc = self.transact().await?.get_descriptor(fd)?.clone();
        match desc {
            Descriptor::Stdin(..) | Descriptor::Stdout(..) | Descriptor::Stderr(..) => {
                Ok(types::Filestat {
                    dev: 0,
                    ino: 0,
                    filetype: types::Filetype::CharacterDevice,
                    nlink: 0,
                    size: 0,
                    atim: 0,
                    mtim: 0,
                    ctim: 0,
                })
            }
            Descriptor::PreopenDirectory((fd, _)) | Descriptor::File(File { fd, .. }) => {
                let wasi::filesystem::DescriptorStat {
                    device: dev,
                    inode: ino,
                    type_,
                    link_count: nlink,
                    size,
                    data_access_timestamp,
                    data_modification_timestamp,
                    status_change_timestamp,
                } = self.stat(fd).await.map_err(|e| {
                    e.try_into()
                        .context("failed to call `stat`")
                        .unwrap_or_else(types::Error::trap)
                })?;
                let filetype = type_.try_into().map_err(types::Error::trap)?;
                let atim = data_access_timestamp.try_into()?;
                let mtim = data_modification_timestamp.try_into()?;
                let ctim = status_change_timestamp.try_into()?;
                Ok(types::Filestat {
                    dev,
                    ino,
                    filetype,
                    nlink,
                    size,
                    atim,
                    mtim,
                    ctim,
                })
            }
        }
    }

    /// Adjust the size of an open file. If this increases the file's size, the extra bytes are filled with zeros.
    /// NOTE: This is similar to `ftruncate` in POSIX.
    #[instrument(skip(self))]
    async fn fd_filestat_set_size(
        &mut self,
        fd: types::Fd,
        size: types::Filesize,
    ) -> Result<(), types::Error> {
        let fd = self.get_file_fd(fd).await?;
        self.set_size(fd, size).await.map_err(|e| {
            e.try_into()
                .context("failed to call `set-size`")
                .unwrap_or_else(types::Error::trap)
        })
    }

    /// Adjust the timestamps of an open file or directory.
    /// NOTE: This is similar to `futimens` in POSIX.
    #[instrument(skip(self))]
    async fn fd_filestat_set_times(
        &mut self,
        fd: types::Fd,
        atim: types::Timestamp,
        mtim: types::Timestamp,
        fst_flags: types::Fstflags,
    ) -> Result<(), types::Error> {
        let atim = systimespec(
            fst_flags.contains(types::Fstflags::ATIM),
            atim,
            fst_flags.contains(types::Fstflags::ATIM_NOW),
        )?;
        let mtim = systimespec(
            fst_flags.contains(types::Fstflags::MTIM),
            mtim,
            fst_flags.contains(types::Fstflags::MTIM_NOW),
        )?;

        let fd = self.get_fd(fd).await?;
        self.set_times(fd, atim, mtim).await.map_err(|e| {
            e.try_into()
                .context("failed to call `set-times`")
                .unwrap_or_else(types::Error::trap)
        })
    }

    /// Read from a file descriptor.
    /// NOTE: This is similar to `readv` in POSIX.
    #[instrument(skip(self))]
    async fn fd_read<'a>(
        &mut self,
        fd: types::Fd,
        iovs: &types::IovecArray<'a>,
    ) -> Result<types::Size, types::Error> {
        let desc = self.transact().await?.get_descriptor(fd)?.clone();
        let (mut buf, read, end) = match desc {
            Descriptor::File(File {
                fd,
                blocking,
                position,
                ..
            }) if self.table().is_file(fd) => {
                let Some(buf) = first_non_empty_iovec(iovs)? else {
                    return Ok(0)
                };

                let pos = position.load(Ordering::Relaxed);
                let stream = self
                    .read_via_stream(fd, pos)
                    .await
                    .context("failed to call `read-via-stream`")
                    .map_err(types::Error::trap)?;
                let max = buf.len().try_into().unwrap_or(u64::MAX);
                let (read, end) = if blocking {
                    self.blocking_read(stream, max)
                } else {
                    wasi::streams::Host::read(self, stream, max)
                }
                .await
                .map_err(|_| types::Errno::Io)?;

                let n = read.len().try_into().or(Err(types::Errno::Overflow))?;
                let pos = pos.checked_add(n).ok_or(types::Errno::Overflow)?;
                position.store(pos, Ordering::Relaxed);

                (buf, read, end)
            }
            Descriptor::Stdin(stream) => {
                let Some(buf) = first_non_empty_iovec(iovs)? else {
                    return Ok(0)
                };
                let (read, end) = wasi::streams::Host::read(
                    self,
                    stream,
                    buf.len().try_into().unwrap_or(u64::MAX),
                )
                .await
                .map_err(|_| types::Errno::Io)?;
                (buf, read, end)
            }
            _ => return Err(types::Errno::Badf.into()),
        };
        if read.len() > buf.len() {
            return Err(types::Errno::Range.into());
        }
        if !end && read.len() == 0 {
            return Err(types::Errno::Intr.into());
        }
        let (buf, _) = buf.split_at_mut(read.len());
        buf.copy_from_slice(&read);
        let n = read.len().try_into().or(Err(types::Errno::Overflow))?;
        Ok(n)
    }

    /// Read from a file descriptor, without using and updating the file descriptor's offset.
    /// NOTE: This is similar to `preadv` in POSIX.
    #[instrument(skip(self))]
    async fn fd_pread<'a>(
        &mut self,
        fd: types::Fd,
        iovs: &types::IovecArray<'a>,
        offset: types::Filesize,
    ) -> Result<types::Size, types::Error> {
        let desc = self.transact().await?.get_descriptor(fd)?.clone();
        let (mut buf, read, end) = match desc {
            Descriptor::File(File { fd, blocking, .. }) if self.table().is_file(fd) => {
                let Some(buf) = first_non_empty_iovec(iovs)? else {
                    return Ok(0)
                };

                let stream = self
                    .read_via_stream(fd, offset)
                    .await
                    .context("failed to call `read-via-stream`")
                    .map_err(types::Error::trap)?;
                let max = buf.len().try_into().unwrap_or(u64::MAX);
                let (read, end) = if blocking {
                    self.blocking_read(stream, max)
                } else {
                    wasi::streams::Host::read(self, stream, max)
                }
                .await
                .map_err(|_| types::Errno::Io)?;

                (buf, read, end)
            }
            Descriptor::Stdin(..) => {
                // NOTE: legacy implementation returns SPIPE here
                return Err(types::Errno::Spipe.into());
            }
            _ => return Err(types::Errno::Badf.into()),
        };
        if read.len() > buf.len() {
            return Err(types::Errno::Range.into());
        }
        if !end && read.len() == 0 {
            return Err(types::Errno::Intr.into());
        }
        let (buf, _) = buf.split_at_mut(read.len());
        buf.copy_from_slice(&read);
        let n = read.len().try_into().or(Err(types::Errno::Overflow))?;
        Ok(n)
    }

    /// Write to a file descriptor.
    /// NOTE: This is similar to `writev` in POSIX.
    #[instrument(skip(self))]
    async fn fd_write<'a>(
        &mut self,
        fd: types::Fd,
        ciovs: &types::CiovecArray<'a>,
    ) -> Result<types::Size, types::Error> {
        let desc = self.transact().await?.get_descriptor(fd)?.clone();
        let n = match desc {
            Descriptor::File(File {
                fd,
                blocking,
                append,
                position,
            }) if self.table().is_file(fd) => {
                let Some(buf) = first_non_empty_ciovec(ciovs)? else {
                    return Ok(0)
                };
                let (stream, pos) = if append {
                    let stream = self
                        .append_via_stream(fd)
                        .await
                        .context("failed to call `append-via-stream`")
                        .map_err(types::Error::trap)?;
                    (stream, 0)
                } else {
                    let position = position.load(Ordering::Relaxed);
                    let stream = self
                        .write_via_stream(fd, position)
                        .await
                        .context("failed to call `write-via-stream`")
                        .map_err(types::Error::trap)?;
                    (stream, position)
                };
                let n = if blocking {
                    self.blocking_write(stream, buf)
                } else {
                    wasi::streams::Host::write(self, stream, buf)
                }
                .await
                .map_err(|_| types::Errno::Io)?;
                if !append {
                    let pos = pos.checked_add(n).ok_or(types::Errno::Overflow)?;
                    position.store(pos, Ordering::Relaxed);
                }
                n
            }
            Descriptor::Stdout(stream) | Descriptor::Stderr(stream) => {
                let Some(buf) = first_non_empty_ciovec(ciovs)? else {
                    return Ok(0)
                };
                wasi::streams::Host::write(self, stream, buf)
                    .await
                    .map_err(|_| types::Errno::Io)?
            }
            _ => return Err(types::Errno::Badf.into()),
        }
        .try_into()
        .or(Err(types::Errno::Overflow))?;
        Ok(n)
    }

    /// Write to a file descriptor, without using and updating the file descriptor's offset.
    /// NOTE: This is similar to `pwritev` in POSIX.
    #[instrument(skip(self))]
    async fn fd_pwrite<'a>(
        &mut self,
        fd: types::Fd,
        ciovs: &types::CiovecArray<'a>,
        offset: types::Filesize,
    ) -> Result<types::Size, types::Error> {
        let desc = self.transact().await?.get_descriptor(fd)?.clone();
        let n = match desc {
            Descriptor::File(File { fd, blocking, .. }) if self.table().is_file(fd) => {
                let Some(buf) = first_non_empty_ciovec(ciovs)? else {
                    return Ok(0)
                };
                let stream = self
                    .write_via_stream(fd, offset)
                    .await
                    .context("failed to call `write-via-stream`")
                    .map_err(types::Error::trap)?;
                if blocking {
                    self.blocking_write(stream, buf)
                } else {
                    wasi::streams::Host::write(self, stream, buf)
                }
                .await
                .map_err(|_| types::Errno::Io)?
            }
            Descriptor::Stdout(..) | Descriptor::Stderr(..) => {
                // NOTE: legacy implementation returns SPIPE here
                return Err(types::Errno::Spipe.into());
            }
            _ => return Err(types::Errno::Badf.into()),
        }
        .try_into()
        .or(Err(types::Errno::Overflow))?;
        Ok(n)
    }

    /// Return a description of the given preopened file descriptor.
    #[instrument(skip(self))]
    async fn fd_prestat_get(&mut self, fd: types::Fd) -> Result<types::Prestat, types::Error> {
        if let Descriptor::PreopenDirectory((_, p)) = self.transact().await?.get_descriptor(fd)? {
            let pr_name_len = p.len().try_into().or(Err(types::Errno::Overflow))?;
            return Ok(types::Prestat::Dir(types::PrestatDir { pr_name_len }));
        }
        Err(types::Errno::Badf.into()) // NOTE: legacy implementation returns BADF here
    }

    /// Return a description of the given preopened file descriptor.
    #[instrument(skip(self))]
    async fn fd_prestat_dir_name<'a>(
        &mut self,
        fd: types::Fd,
        path: &GuestPtr<'a, u8>,
        path_max_len: types::Size,
    ) -> Result<(), types::Error> {
        let path_max_len = path_max_len.try_into().or(Err(types::Errno::Overflow))?;
        if let Descriptor::PreopenDirectory((_, p)) = self.transact().await?.get_descriptor(fd)? {
            if p.len() > path_max_len {
                return Err(types::Errno::Nametoolong.into());
            }
            write_bytes(path, p)?;
            return Ok(());
        }
        Err(types::Errno::Notdir.into()) // NOTE: legacy implementation returns NOTDIR here
    }

    /// Atomically replace a file descriptor by renumbering another file descriptor.
    #[instrument(skip(self))]
    async fn fd_renumber(&mut self, from: types::Fd, to: types::Fd) -> Result<(), types::Error> {
        let mut st = self.transact().await?;
        let descriptors = st.descriptors.get_mut();
        let desc = descriptors.remove(from).ok_or(types::Errno::Badf)?;
        descriptors.insert(to.into(), desc);
        Ok(())
    }

    /// Move the offset of a file descriptor.
    /// NOTE: This is similar to `lseek` in POSIX.
    #[instrument(skip(self))]
    async fn fd_seek(
        &mut self,
        fd: types::Fd,
        offset: types::Filedelta,
        whence: types::Whence,
    ) -> Result<types::Filesize, types::Error> {
        let (fd, position) = {
            let mut st = self.transact().await?;
            let File { fd, position, .. } = st.get_seekable(fd)?;
            (*fd, Arc::clone(&position))
        };
        let pos = match whence {
            types::Whence::Set if offset >= 0 => offset as _,
            types::Whence::Cur => position
                .load(Ordering::Relaxed)
                .checked_add_signed(offset)
                .ok_or(types::Errno::Inval)?,
            types::Whence::End => {
                let wasi::filesystem::DescriptorStat { size, .. } =
                    self.stat(fd).await.map_err(|e| {
                        e.try_into()
                            .context("failed to call `stat`")
                            .unwrap_or_else(types::Error::trap)
                    })?;
                size.checked_add_signed(offset).ok_or(types::Errno::Inval)?
            }
            _ => return Err(types::Errno::Inval.into()),
        };
        position.store(pos, Ordering::Relaxed);
        Ok(pos)
    }

    /// Synchronize the data and metadata of a file to disk.
    /// NOTE: This is similar to `fsync` in POSIX.
    #[instrument(skip(self))]
    async fn fd_sync(&mut self, fd: types::Fd) -> Result<(), types::Error> {
        let fd = self.get_file_fd(fd).await?;
        self.sync(fd).await.map_err(|e| {
            e.try_into()
                .context("failed to call `sync`")
                .unwrap_or_else(types::Error::trap)
        })
    }

    /// Return the current offset of a file descriptor.
    /// NOTE: This is similar to `lseek(fd, 0, SEEK_CUR)` in POSIX.
    #[instrument(skip(self))]
    async fn fd_tell(&mut self, fd: types::Fd) -> Result<types::Filesize, types::Error> {
        let pos = self
            .transact()
            .await?
            .get_seekable(fd)
            .map(|File { position, .. }| position.load(Ordering::Relaxed))?;
        Ok(pos)
    }

    #[instrument(skip(self))]
    async fn fd_readdir<'a>(
        &mut self,
        fd: types::Fd,
        buf: &GuestPtr<'a, u8>,
        buf_len: types::Size,
        cookie: types::Dircookie,
    ) -> Result<types::Size, types::Error> {
        let fd = self.get_dir_fd(fd).await?;
        let stream = self.read_directory(fd).await.map_err(|e| {
            e.try_into()
                .context("failed to call `read-directory`")
                .unwrap_or_else(types::Error::trap)
        })?;
        let wasi::filesystem::DescriptorStat {
            inode: fd_inode, ..
        } = self.stat(fd).await.map_err(|e| {
            e.try_into()
                .context("failed to call `stat`")
                .unwrap_or_else(types::Error::trap)
        })?;
        let cookie = cookie.try_into().map_err(|_| types::Errno::Overflow)?;

        let head = [
            (
                types::Dirent {
                    d_next: 1u64.to_le(),
                    d_ino: fd_inode.to_le(),
                    d_type: types::Filetype::Directory,
                    d_namlen: 1u32.to_le(),
                },
                ".".into(),
            ),
            (
                types::Dirent {
                    d_next: 2u64.to_le(),
                    d_ino: fd_inode.to_le(), // NOTE: incorrect, but legacy implementation returns `fd` inode here
                    d_type: types::Filetype::Directory,
                    d_namlen: 2u32.to_le(),
                },
                "..".into(),
            ),
        ]
        .into_iter()
        .map(Ok::<_, types::Error>);

        let dir = self
            .table_mut()
            // remove iterator from table and use it directly:
            .delete_readdir(stream)?
            .into_iter()
            .zip(3u64..)
            .map(|(entry, d_next)| {
                let wasi::filesystem::DirectoryEntry { inode, type_, name } =
                    entry.map_err(|e| {
                        e.try_into()
                            .context("failed to inspect `read-directory` entry")
                            .unwrap_or_else(types::Error::trap)
                    })?;
                let d_type = type_.try_into().map_err(types::Error::trap)?;
                let d_namlen: u32 = name.len().try_into().map_err(|_| types::Errno::Overflow)?;
                Ok((
                    types::Dirent {
                        d_next: d_next.to_le(),
                        d_ino: inode.unwrap_or_default().to_le(),
                        d_type, // endian-invariant
                        d_namlen: d_namlen.to_le(),
                    },
                    name,
                ))
            });

        // assume that `types::Dirent` size always fits in `u32`
        const DIRENT_SIZE: u32 = size_of::<types::Dirent>() as _;
        assert_eq!(
            types::Dirent::guest_size(),
            DIRENT_SIZE,
            "Dirent guest repr and host repr should match"
        );
        let mut buf = *buf;
        let mut cap = buf_len;
        for entry in head.chain(dir).skip(cookie) {
            let (ref entry, mut path) = entry?;

            assert_eq!(
                1,
                size_of_val(&entry.d_type),
                "Dirent member d_type should be endian-invariant"
            );
            let entry_len = cap.min(DIRENT_SIZE);
            let entry = entry as *const _ as _;
            let entry = unsafe { slice::from_raw_parts(entry, entry_len as _) };
            cap = cap.checked_sub(entry_len).unwrap();
            buf = write_bytes(buf, entry)?;
            if cap == 0 {
                return Ok(buf_len);
            }

            if let Ok(cap) = cap.try_into() {
                // `path` cannot be longer than `usize`, only truncate if `cap` fits in `usize`
                path.truncate(cap);
            }
            cap = cap.checked_sub(path.len() as _).unwrap();
            buf = write_bytes(buf, path)?;
            if cap == 0 {
                return Ok(buf_len);
            }
        }
        Ok(buf_len.checked_sub(cap).unwrap())
    }

    #[instrument(skip(self))]
    async fn path_create_directory<'a>(
        &mut self,
        dirfd: types::Fd,
        path: &GuestPtr<'a, str>,
    ) -> Result<(), types::Error> {
        let dirfd = self.get_dir_fd(dirfd).await?;
        let path = read_string(path)?;
        self.create_directory_at(dirfd, path).await.map_err(|e| {
            e.try_into()
                .context("failed to call `create-directory-at`")
                .unwrap_or_else(types::Error::trap)
        })
    }

    /// Return the attributes of a file or directory.
    /// NOTE: This is similar to `stat` in POSIX.
    #[instrument(skip(self))]
    async fn path_filestat_get<'a>(
        &mut self,
        dirfd: types::Fd,
        flags: types::Lookupflags,
        path: &GuestPtr<'a, str>,
    ) -> Result<types::Filestat, types::Error> {
        let dirfd = self.get_dir_fd(dirfd).await?;
        let path = read_string(path)?;
        let wasi::filesystem::DescriptorStat {
            device: dev,
            inode: ino,
            type_,
            link_count: nlink,
            size,
            data_access_timestamp,
            data_modification_timestamp,
            status_change_timestamp,
        } = self.stat_at(dirfd, flags.into(), path).await.map_err(|e| {
            e.try_into()
                .context("failed to call `stat-at`")
                .unwrap_or_else(types::Error::trap)
        })?;
        let filetype = type_.try_into().map_err(types::Error::trap)?;
        let atim = data_access_timestamp.try_into()?;
        let mtim = data_modification_timestamp.try_into()?;
        let ctim = status_change_timestamp.try_into()?;
        Ok(types::Filestat {
            dev,
            ino,
            filetype,
            nlink,
            size,
            atim,
            mtim,
            ctim,
        })
    }

    /// Adjust the timestamps of a file or directory.
    /// NOTE: This is similar to `utimensat` in POSIX.
    #[instrument(skip(self))]
    async fn path_filestat_set_times<'a>(
        &mut self,
        dirfd: types::Fd,
        flags: types::Lookupflags,
        path: &GuestPtr<'a, str>,
        atim: types::Timestamp,
        mtim: types::Timestamp,
        fst_flags: types::Fstflags,
    ) -> Result<(), types::Error> {
        let atim = systimespec(
            fst_flags.contains(types::Fstflags::ATIM),
            atim,
            fst_flags.contains(types::Fstflags::ATIM_NOW),
        )?;
        let mtim = systimespec(
            fst_flags.contains(types::Fstflags::MTIM),
            mtim,
            fst_flags.contains(types::Fstflags::MTIM_NOW),
        )?;

        let dirfd = self.get_dir_fd(dirfd).await?;
        let path = read_string(path)?;
        self.set_times_at(dirfd, flags.into(), path, atim, mtim)
            .await
            .map_err(|e| {
                e.try_into()
                    .context("failed to call `set-times-at`")
                    .unwrap_or_else(types::Error::trap)
            })
    }

    /// Create a hard link.
    /// NOTE: This is similar to `linkat` in POSIX.
    #[instrument(skip(self))]
    async fn path_link<'a>(
        &mut self,
        src_fd: types::Fd,
        src_flags: types::Lookupflags,
        src_path: &GuestPtr<'a, str>,
        target_fd: types::Fd,
        target_path: &GuestPtr<'a, str>,
    ) -> Result<(), types::Error> {
        let src_fd = self.get_dir_fd(src_fd).await?;
        let target_fd = self.get_dir_fd(target_fd).await?;
        let src_path = read_string(src_path)?;
        let target_path = read_string(target_path)?;
        self.link_at(src_fd, src_flags.into(), src_path, target_fd, target_path)
            .await
            .map_err(|e| {
                e.try_into()
                    .context("failed to call `link-at`")
                    .unwrap_or_else(types::Error::trap)
            })
    }

    /// Open a file or directory.
    /// NOTE: This is similar to `openat` in POSIX.
    #[instrument(skip(self))]
    async fn path_open<'a>(
        &mut self,
        dirfd: types::Fd,
        dirflags: types::Lookupflags,
        path: &GuestPtr<'a, str>,
        oflags: types::Oflags,
        fs_rights_base: types::Rights,
        _fs_rights_inheriting: types::Rights,
        fdflags: types::Fdflags,
    ) -> Result<types::Fd, types::Error> {
        let path = read_string(path)?;

        let mut flags = wasi::filesystem::DescriptorFlags::empty();
        if fs_rights_base.contains(types::Rights::FD_READ) {
            flags |= wasi::filesystem::DescriptorFlags::READ;
        }
        if fs_rights_base.contains(types::Rights::FD_WRITE) {
            flags |= wasi::filesystem::DescriptorFlags::WRITE;
        }
        if fdflags.contains(types::Fdflags::SYNC) {
            flags |= wasi::filesystem::DescriptorFlags::FILE_INTEGRITY_SYNC;
        }
        if fdflags.contains(types::Fdflags::DSYNC) {
            flags |= wasi::filesystem::DescriptorFlags::DATA_INTEGRITY_SYNC;
        }
        if fdflags.contains(types::Fdflags::RSYNC) {
            flags |= wasi::filesystem::DescriptorFlags::REQUESTED_WRITE_SYNC;
        }

        let desc = self.transact().await?.get_descriptor(dirfd)?.clone();
        let dirfd = match desc {
            Descriptor::PreopenDirectory((fd, _)) => fd,
            Descriptor::File(File { fd, .. }) if self.table().is_dir(fd) => fd,
            Descriptor::File(File { fd, .. }) if !self.table().is_dir(fd) => {
                // NOTE: Unlike most other methods, legacy implementation returns `NOTDIR` here
                return Err(types::Errno::Notdir.into());
            }
            _ => return Err(types::Errno::Badf.into()),
        };
        let fd = self
            .open_at(
                dirfd,
                dirflags.into(),
                path,
                oflags.into(),
                flags,
                wasi::filesystem::Modes::READABLE | wasi::filesystem::Modes::WRITABLE,
            )
            .await
            .map_err(|e| {
                e.try_into()
                    .context("failed to call `open-at`")
                    .unwrap_or_else(types::Error::trap)
            })?;
        let fd = self
            .transact()
            .await?
            .descriptors
            .get_mut()
            .push_file(File {
                fd,
                position: Default::default(),
                append: fdflags.contains(types::Fdflags::APPEND),
                blocking: !fdflags.contains(types::Fdflags::NONBLOCK),
            })?;
        Ok(fd.into())
    }

    /// Read the contents of a symbolic link.
    /// NOTE: This is similar to `readlinkat` in POSIX.
    #[instrument(skip(self))]
    async fn path_readlink<'a>(
        &mut self,
        dirfd: types::Fd,
        path: &GuestPtr<'a, str>,
        buf: &GuestPtr<'a, u8>,
        buf_len: types::Size,
    ) -> Result<types::Size, types::Error> {
        let dirfd = self.get_dir_fd(dirfd).await?;
        let path = read_string(path)?;
        let mut path = self.readlink_at(dirfd, path).await.map_err(|e| {
            e.try_into()
                .context("failed to call `readlink-at`")
                .unwrap_or_else(types::Error::trap)
        })?;
        if let Ok(buf_len) = buf_len.try_into() {
            // `path` cannot be longer than `usize`, only truncate if `buf_len` fits in `usize`
            path.truncate(buf_len);
        }
        let n = path.len().try_into().map_err(|_| types::Errno::Overflow)?;
        write_bytes(buf, &path)?;
        Ok(n)
    }

    #[instrument(skip(self))]
    async fn path_remove_directory<'a>(
        &mut self,
        dirfd: types::Fd,
        path: &GuestPtr<'a, str>,
    ) -> Result<(), types::Error> {
        let dirfd = self.get_dir_fd(dirfd).await?;
        let path = read_string(path)?;
        self.remove_directory_at(dirfd, path).await.map_err(|e| {
            e.try_into()
                .context("failed to call `remove-directory-at`")
                .unwrap_or_else(types::Error::trap)
        })
    }

    /// Rename a file or directory.
    /// NOTE: This is similar to `renameat` in POSIX.
    #[instrument(skip(self))]
    async fn path_rename<'a>(
        &mut self,
        src_fd: types::Fd,
        src_path: &GuestPtr<'a, str>,
        dest_fd: types::Fd,
        dest_path: &GuestPtr<'a, str>,
    ) -> Result<(), types::Error> {
        let src_fd = self.get_dir_fd(src_fd).await?;
        let dest_fd = self.get_dir_fd(dest_fd).await?;
        let src_path = read_string(src_path)?;
        let dest_path = read_string(dest_path)?;
        self.rename_at(src_fd, src_path, dest_fd, dest_path)
            .await
            .map_err(|e| {
                e.try_into()
                    .context("failed to call `rename-at`")
                    .unwrap_or_else(types::Error::trap)
            })
    }

    #[instrument(skip(self))]
    async fn path_symlink<'a>(
        &mut self,
        src_path: &GuestPtr<'a, str>,
        dirfd: types::Fd,
        dest_path: &GuestPtr<'a, str>,
    ) -> Result<(), types::Error> {
        let dirfd = self.get_dir_fd(dirfd).await?;
        let src_path = read_string(src_path)?;
        let dest_path = read_string(dest_path)?;
        self.symlink_at(dirfd, src_path, dest_path)
            .await
            .map_err(|e| {
                e.try_into()
                    .context("failed to call `symlink-at`")
                    .unwrap_or_else(types::Error::trap)
            })
    }

    #[instrument(skip(self))]
    async fn path_unlink_file<'a>(
        &mut self,
        dirfd: types::Fd,
        path: &GuestPtr<'a, str>,
    ) -> Result<(), types::Error> {
        let dirfd = self.get_dir_fd(dirfd).await?;
        let path = path.as_cow().map_err(|_| types::Errno::Inval)?.to_string();
        self.unlink_file_at(dirfd, path).await.map_err(|e| {
            e.try_into()
                .context("failed to call `unlink-file-at`")
                .unwrap_or_else(types::Error::trap)
        })
    }

    #[allow(unused_variables)]
    #[instrument(skip(self))]
    async fn poll_oneoff<'a>(
        &mut self,
        subs: &GuestPtr<'a, types::Subscription>,
        events: &GuestPtr<'a, types::Event>,
        nsubscriptions: types::Size,
    ) -> Result<types::Size, types::Error> {
        todo!()
    }

    #[instrument(skip(self))]
    async fn proc_exit(&mut self, status: types::Exitcode) -> anyhow::Error {
        let status = match status {
            0 => Ok(()),
            _ => Err(()),
        };
        match self.exit(status).await {
            Err(e) => e,
            Ok(()) => anyhow!("`exit` did not return an error"),
        }
    }

    #[instrument(skip(self))]
    async fn proc_raise(&mut self, _sig: types::Signal) -> Result<(), types::Error> {
        Err(types::Errno::Notsup.into())
    }

    #[instrument(skip(self))]
    async fn sched_yield(&mut self) -> Result<(), types::Error> {
        // TODO: This is not yet covered in Preview2.
        Ok(())
    }

    #[instrument(skip(self))]
    async fn random_get<'a>(
        &mut self,
        buf: &GuestPtr<'a, u8>,
        buf_len: types::Size,
    ) -> Result<(), types::Error> {
        let rand = self
            .get_random_bytes(buf_len.into())
            .await
            .context("failed to call `get-random-bytes`")
            .map_err(types::Error::trap)?;
        write_bytes(buf, rand)?;
        Ok(())
    }

    #[allow(unused_variables)]
    #[instrument(skip(self))]
    async fn sock_accept(
        &mut self,
        fd: types::Fd,
        flags: types::Fdflags,
    ) -> Result<types::Fd, types::Error> {
        todo!()
    }

    #[allow(unused_variables)]
    #[instrument(skip(self))]
    async fn sock_recv<'a>(
        &mut self,
        fd: types::Fd,
        ri_data: &types::IovecArray<'a>,
        ri_flags: types::Riflags,
    ) -> Result<(types::Size, types::Roflags), types::Error> {
        todo!()
    }

    #[allow(unused_variables)]
    #[instrument(skip(self))]
    async fn sock_send<'a>(
        &mut self,
        fd: types::Fd,
        si_data: &types::CiovecArray<'a>,
        _si_flags: types::Siflags,
    ) -> Result<types::Size, types::Error> {
        todo!()
    }

    #[allow(unused_variables)]
    #[instrument(skip(self))]
    async fn sock_shutdown(
        &mut self,
        fd: types::Fd,
        how: types::Sdflags,
    ) -> Result<(), types::Error> {
        todo!()
    }
}
