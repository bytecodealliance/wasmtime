use crate::old::snapshot_0::sys::dev_null;
use crate::old::snapshot_0::sys::fdentry_impl::{
    descriptor_as_oshandle, determine_type_and_access_rights, OsHandle,
};
use crate::old::snapshot_0::{wasi, Error, Result};
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::{fs, io};

#[derive(Debug)]
pub(crate) enum Descriptor {
    OsHandle(OsHandle),
    Stdin,
    Stdout,
    Stderr,
}

impl Descriptor {
    /// Return a reference to the `OsHandle` treating it as an actual file/dir, and
    /// allowing operations which require an actual file and not just a stream or
    /// socket file descriptor.
    pub(crate) fn as_file(&self) -> Result<&OsHandle> {
        match self {
            Self::OsHandle(file) => Ok(file),
            _ => Err(Error::EBADF),
        }
    }

    /// Like `as_file`, but return a mutable reference.
    pub(crate) fn as_file_mut(&mut self) -> Result<&mut OsHandle> {
        match self {
            Self::OsHandle(file) => Ok(file),
            _ => Err(Error::EBADF),
        }
    }

    /// Return an `OsHandle`, which may be a stream or socket file descriptor.
    pub(crate) fn as_os_handle<'descriptor>(&'descriptor self) -> OsHandleRef<'descriptor> {
        descriptor_as_oshandle(self)
    }
}

/// An abstraction struct serving as a wrapper for a host `Descriptor` object which requires
/// certain base rights `rights_base` and inheriting rights `rights_inheriting` in order to be
/// accessed correctly.
///
/// Here, the `descriptor` field stores the host `Descriptor` object (such as a file descriptor, or
/// stdin handle), and accessing it can only be done via the provided `FdEntry::as_descriptor` and
/// `FdEntry::as_descriptor_mut` methods which require a set of base and inheriting rights to be
/// specified, verifying whether the stored `Descriptor` object is valid for the rights specified.
#[derive(Debug)]
pub(crate) struct FdEntry {
    pub(crate) file_type: wasi::__wasi_filetype_t,
    descriptor: Descriptor,
    pub(crate) rights_base: wasi::__wasi_rights_t,
    pub(crate) rights_inheriting: wasi::__wasi_rights_t,
    pub(crate) preopen_path: Option<PathBuf>,
    // TODO: directories
}

impl FdEntry {
    pub(crate) fn from(file: fs::File) -> Result<Self> {
        unsafe { determine_type_and_access_rights(&file) }.map(
            |(file_type, rights_base, rights_inheriting)| Self {
                file_type,
                descriptor: Descriptor::OsHandle(OsHandle::from(file)),
                rights_base,
                rights_inheriting,
                preopen_path: None,
            },
        )
    }

    pub(crate) fn duplicate_stdin() -> Result<Self> {
        unsafe { determine_type_and_access_rights(&io::stdin()) }.map(
            |(file_type, rights_base, rights_inheriting)| Self {
                file_type,
                descriptor: Descriptor::Stdin,
                rights_base,
                rights_inheriting,
                preopen_path: None,
            },
        )
    }

    pub(crate) fn duplicate_stdout() -> Result<Self> {
        unsafe { determine_type_and_access_rights(&io::stdout()) }.map(
            |(file_type, rights_base, rights_inheriting)| Self {
                file_type,
                descriptor: Descriptor::Stdout,
                rights_base,
                rights_inheriting,
                preopen_path: None,
            },
        )
    }

    pub(crate) fn duplicate_stderr() -> Result<Self> {
        unsafe { determine_type_and_access_rights(&io::stderr()) }.map(
            |(file_type, rights_base, rights_inheriting)| Self {
                file_type,
                descriptor: Descriptor::Stderr,
                rights_base,
                rights_inheriting,
                preopen_path: None,
            },
        )
    }

    pub(crate) fn null() -> Result<Self> {
        Self::from(dev_null()?)
    }

    /// Convert this `FdEntry` into a host `Descriptor` object provided the specified
    /// `rights_base` and `rights_inheriting` rights are set on this `FdEntry` object.
    ///
    /// The `FdEntry` can only be converted into a valid `Descriptor` object if
    /// the specified set of base rights `rights_base`, and inheriting rights `rights_inheriting`
    /// is a subset of rights attached to this `FdEntry`. The check is performed using
    /// `FdEntry::validate_rights` method. If the check fails, `Error::ENOTCAPABLE` is returned.
    pub(crate) fn as_descriptor(
        &self,
        rights_base: wasi::__wasi_rights_t,
        rights_inheriting: wasi::__wasi_rights_t,
    ) -> Result<&Descriptor> {
        self.validate_rights(rights_base, rights_inheriting)?;
        Ok(&self.descriptor)
    }

    /// Convert this `FdEntry` into a mutable host `Descriptor` object provided the specified
    /// `rights_base` and `rights_inheriting` rights are set on this `FdEntry` object.
    ///
    /// The `FdEntry` can only be converted into a valid `Descriptor` object if
    /// the specified set of base rights `rights_base`, and inheriting rights `rights_inheriting`
    /// is a subset of rights attached to this `FdEntry`. The check is performed using
    /// `FdEntry::validate_rights` method. If the check fails, `Error::ENOTCAPABLE` is returned.
    pub(crate) fn as_descriptor_mut(
        &mut self,
        rights_base: wasi::__wasi_rights_t,
        rights_inheriting: wasi::__wasi_rights_t,
    ) -> Result<&mut Descriptor> {
        self.validate_rights(rights_base, rights_inheriting)?;
        Ok(&mut self.descriptor)
    }

    /// Check if this `FdEntry` object satisfies the specified base rights `rights_base`, and
    /// inheriting rights `rights_inheriting`; i.e., if rights attached to this `FdEntry` object
    /// are a superset.
    ///
    /// Upon unsuccessful check, `Error::ENOTCAPABLE` is returned.
    fn validate_rights(
        &self,
        rights_base: wasi::__wasi_rights_t,
        rights_inheriting: wasi::__wasi_rights_t,
    ) -> Result<()> {
        if !self.rights_base & rights_base != 0 || !self.rights_inheriting & rights_inheriting != 0
        {
            Err(Error::ENOTCAPABLE)
        } else {
            Ok(())
        }
    }

    /// Test whether this descriptor is considered a tty within WASI.
    /// Note that since WASI itself lacks an `isatty` syscall and relies
    /// on a conservative approximation, we use the same approximation here.
    pub(crate) fn isatty(&self) -> bool {
        self.file_type == wasi::__WASI_FILETYPE_CHARACTER_DEVICE
            && (self.rights_base & (wasi::__WASI_RIGHTS_FD_SEEK | wasi::__WASI_RIGHTS_FD_TELL)) == 0
    }
}

/// This allows an `OsHandle` to be temporarily borrowed from a
/// `Descriptor`. The `Descriptor` continues to own the resource,
/// and `OsHandleRef`'s lifetime parameter ensures that it doesn't
/// outlive the `Descriptor`.
pub(crate) struct OsHandleRef<'descriptor> {
    handle: ManuallyDrop<OsHandle>,
    _ref: PhantomData<&'descriptor Descriptor>,
}

impl<'descriptor> OsHandleRef<'descriptor> {
    pub(crate) fn new(handle: ManuallyDrop<OsHandle>) -> Self {
        OsHandleRef {
            handle,
            _ref: PhantomData,
        }
    }
}

impl<'descriptor> Deref for OsHandleRef<'descriptor> {
    type Target = fs::File;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}

impl<'descriptor> DerefMut for OsHandleRef<'descriptor> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.handle
    }
}
