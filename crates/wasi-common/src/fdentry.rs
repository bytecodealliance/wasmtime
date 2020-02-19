use crate::sys::dev_null;
use crate::sys::fdentry_impl::{
    descriptor_as_oshandle, determine_type_and_access_rights, OsHandle,
};
use crate::virtfs::VirtualFile;
use crate::{wasi, Error, Result};
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::{fmt, fs, io};

pub(crate) enum HandleMut<'handle> {
    OsHandle(&'handle mut OsHandle),
    VirtualFile(&'handle mut dyn VirtualFile),
    Stream(OsHandleRef<'handle>),
}

impl<'descriptor> fmt::Debug for HandleMut<'descriptor> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            HandleMut::OsHandle(file) => {
                // coerce to the target debug-printable type
                let file: &fs::File = file;
                write!(f, "{:?}", file)
            }
            HandleMut::Stream(stream) => {
                // coerce to the target debug-printable type
                let file: &fs::File = stream;
                write!(f, "{:?}", file)
            }
            HandleMut::VirtualFile(_) => write!(f, "VirtualFile"),
        }
    }
}

pub(crate) enum Handle<'handle> {
    OsHandle(&'handle OsHandle),
    VirtualFile(&'handle dyn VirtualFile),
    Stream(OsHandleRef<'handle>),
}

impl<'descriptor> Handle<'descriptor> {
    pub(crate) fn try_clone(&self) -> io::Result<Descriptor> {
        match self {
            Handle::OsHandle(file) => file.try_clone().map(|f| OsHandle::from(f).into()),
            Handle::Stream(stream) => stream.try_clone().map(|f| OsHandle::from(f).into()),
            Handle::VirtualFile(virt) => virt.try_clone().map(Descriptor::VirtualFile),
        }
    }
}

impl<'descriptor> fmt::Debug for Handle<'descriptor> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Handle::OsHandle(file) => {
                // coerce to the target debug-printable type
                let file: &fs::File = file;
                write!(f, "{:?}", file)
            }
            Handle::Stream(stream) => {
                // coerce to the target debug-printable type
                let file: &fs::File = stream;
                write!(f, "{:?}", file)
            }
            Handle::VirtualFile(_) => write!(f, "VirtualFile"),
        }
    }
}

pub(crate) enum Descriptor {
    OsHandle(OsHandle),
    VirtualFile(Box<dyn VirtualFile>),
    Stdin,
    Stdout,
    Stderr,
}

impl From<OsHandle> for Descriptor {
    fn from(handle: OsHandle) -> Self {
        Descriptor::OsHandle(handle)
    }
}

impl From<Box<dyn VirtualFile>> for Descriptor {
    fn from(virt: Box<dyn VirtualFile>) -> Self {
        Descriptor::VirtualFile(virt)
    }
}

impl fmt::Debug for Descriptor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Descriptor::OsHandle(handle) => write!(f, "{:?}", handle),
            Descriptor::VirtualFile(_) => write!(f, "VirtualFile"),
            Descriptor::Stdin => write!(f, "Stdin"),
            Descriptor::Stdout => write!(f, "Stdout"),
            Descriptor::Stderr => write!(f, "Stderr"),
        }
    }
}

impl Descriptor {
    /// Return a reference to the `OsHandle` or `VirtualFile` treating it as an
    /// actual file/dir, and allowing operations which require an actual file and
    /// not just a stream or socket file descriptor.
    pub(crate) fn as_file<'descriptor>(&'descriptor self) -> Result<Handle<'descriptor>> {
        match self {
            Self::OsHandle(handle) => Ok(Handle::OsHandle(handle)),
            Self::VirtualFile(virt) => Ok(Handle::VirtualFile(virt.as_ref())),
            _ => Err(Error::EBADF),
        }
    }

    /// Like `as_file`, but return a mutable reference.
    pub(crate) fn as_file_mut<'descriptor>(
        &'descriptor mut self,
    ) -> Result<HandleMut<'descriptor>> {
        match self {
            Self::OsHandle(handle) => Ok(HandleMut::OsHandle(handle)),
            Self::VirtualFile(virt) => Ok(HandleMut::VirtualFile(virt.as_mut())),
            _ => Err(Error::EBADF),
        }
    }

    pub(crate) fn as_handle<'descriptor>(&'descriptor self) -> Handle<'descriptor> {
        match self {
            Self::VirtualFile(virt) => Handle::VirtualFile(virt.as_ref()),
            Self::OsHandle(handle) => Handle::OsHandle(handle),
            other @ Self::Stdin | other @ Self::Stderr | other @ Self::Stdout => {
                Handle::Stream(other.as_os_handle())
            }
        }
    }

    pub(crate) fn as_handle_mut<'descriptor>(&'descriptor mut self) -> HandleMut<'descriptor> {
        match self {
            Self::VirtualFile(virt) => HandleMut::VirtualFile(virt.as_mut()),
            Self::OsHandle(handle) => HandleMut::OsHandle(handle),
            other @ Self::Stdin | other @ Self::Stderr | other @ Self::Stdout => {
                HandleMut::Stream(other.as_os_handle())
            }
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
    pub(crate) fn from(file: Descriptor) -> Result<Self> {
        match file {
            Descriptor::OsHandle(handle) => unsafe { determine_type_and_access_rights(&handle) }
                .map(|(file_type, rights_base, rights_inheriting)| Self {
                    file_type,
                    descriptor: handle.into(),
                    rights_base,
                    rights_inheriting,
                    preopen_path: None,
                }),
            Descriptor::VirtualFile(virt) => {
                let file_type = virt.get_file_type();
                let rights_base = virt.get_rights_base();
                let rights_inheriting = virt.get_rights_inheriting();

                Ok(Self {
                    file_type,
                    descriptor: virt.into(),
                    rights_base,
                    rights_inheriting,
                    preopen_path: None,
                })
            }
            Descriptor::Stdin | Descriptor::Stdout | Descriptor::Stderr => {
                panic!("implementation error, stdin/stdout/stderr FdEntry must not be constructed from FdEntry::from");
            }
        }
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
        Self::from(OsHandle::from(dev_null()?).into())
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
        let missing_base = !self.rights_base & rights_base;
        let missing_inheriting = !self.rights_inheriting & rights_inheriting;
        if missing_base != 0 || missing_inheriting != 0 {
            log::trace!(
                "     | validate_rights failed: required: \
                 rights_base = {:#x}, rights_inheriting = {:#x}; \
                 actual: rights_base = {:#x}, rights_inheriting = {:#x}; \
                 missing_base = {:#x}, missing_inheriting = {:#x}",
                rights_base,
                rights_inheriting,
                self.rights_base,
                self.rights_inheriting,
                missing_base,
                missing_inheriting
            );
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
