use crate::sys::dev_null;
use crate::sys::entry::{descriptor_as_oshandle, determine_type_and_access_rights, OsHandle};
use crate::virtfs::VirtualFile;
use crate::wasi::types::{Filetype, Rights};
use crate::wasi::{Errno, Result};
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::{fmt, fs, io};

pub(crate) enum Descriptor {
    OsHandle(OsHandle),
    VirtualFile(Box<dyn VirtualFile>),
    Stdin,
    Stdout,
    Stderr,
}

impl From<OsHandle> for Descriptor {
    fn from(handle: OsHandle) -> Self {
        Self::OsHandle(handle)
    }
}

impl From<Box<dyn VirtualFile>> for Descriptor {
    fn from(virt: Box<dyn VirtualFile>) -> Self {
        Self::VirtualFile(virt)
    }
}

impl fmt::Debug for Descriptor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::OsHandle(handle) => write!(f, "{:?}", handle),
            Self::VirtualFile(_) => write!(f, "VirtualFile"),
            Self::Stdin => write!(f, "Stdin"),
            Self::Stdout => write!(f, "Stdout"),
            Self::Stderr => write!(f, "Stderr"),
        }
    }
}

impl Descriptor {
    pub(crate) fn try_clone(&self) -> io::Result<Self> {
        match self {
            Self::OsHandle(file) => file.try_clone().map(|f| OsHandle::from(f).into()),
            Self::VirtualFile(virt) => virt.try_clone().map(Self::VirtualFile),
            Self::Stdin => Ok(Self::Stdin),
            Self::Stdout => Ok(Self::Stdout),
            Self::Stderr => Ok(Self::Stderr),
        }
    }

    /// Return a reference to the `OsHandle` or `VirtualFile` treating it as an
    /// actual file/dir, and allowing operations which require an actual file and
    /// not just a stream or socket file descriptor.
    pub(crate) fn as_file<'descriptor>(&'descriptor self) -> Result<&'descriptor Self> {
        match self {
            Self::OsHandle(_) => Ok(self),
            Self::VirtualFile(_) => Ok(self),
            _ => Err(Errno::Badf),
        }
    }

    /// Like `as_file`, but return a mutable reference.
    pub(crate) fn as_file_mut<'descriptor>(&'descriptor mut self) -> Result<&'descriptor mut Self> {
        match self {
            Self::OsHandle(_) => Ok(self),
            Self::VirtualFile(_) => Ok(self),
            _ => Err(Errno::Badf),
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
/// stdin handle), and accessing it can only be done via the provided `Entry::as_descriptor` and
/// `Entry::as_descriptor_mut` methods which require a set of base and inheriting rights to be
/// specified, verifying whether the stored `Descriptor` object is valid for the rights specified.
#[derive(Debug)]
pub(crate) struct Entry {
    pub(crate) file_type: Filetype,
    descriptor: Descriptor,
    pub(crate) rights_base: Rights,
    pub(crate) rights_inheriting: Rights,
    pub(crate) preopen_path: Option<PathBuf>,
    // TODO: directories
}

impl Entry {
    pub(crate) fn from(file: Descriptor) -> io::Result<Self> {
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
                panic!("implementation error, stdin/stdout/stderr Entry must not be constructed from Entry::from");
            }
        }
    }

    pub(crate) fn duplicate_stdin() -> io::Result<Self> {
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

    pub(crate) fn duplicate_stdout() -> io::Result<Self> {
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

    pub(crate) fn duplicate_stderr() -> io::Result<Self> {
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

    pub(crate) fn null() -> io::Result<Self> {
        Self::from(OsHandle::from(dev_null()?).into())
    }

    /// Convert this `Entry` into a host `Descriptor` object provided the specified
    /// `rights_base` and `rights_inheriting` rights are set on this `Entry` object.
    ///
    /// The `Entry` can only be converted into a valid `Descriptor` object if
    /// the specified set of base rights `rights_base`, and inheriting rights `rights_inheriting`
    /// is a subset of rights attached to this `Entry`. The check is performed using
    /// `Entry::validate_rights` method. If the check fails, `Errno::Notcapable` is returned.
    pub(crate) fn as_descriptor(
        &self,
        rights_base: Rights,
        rights_inheriting: Rights,
    ) -> Result<&Descriptor> {
        self.validate_rights(rights_base, rights_inheriting)?;
        Ok(&self.descriptor)
    }

    /// Convert this `Entry` into a mutable host `Descriptor` object provided the specified
    /// `rights_base` and `rights_inheriting` rights are set on this `Entry` object.
    ///
    /// The `Entry` can only be converted into a valid `Descriptor` object if
    /// the specified set of base rights `rights_base`, and inheriting rights `rights_inheriting`
    /// is a subset of rights attached to this `Entry`. The check is performed using
    /// `Entry::validate_rights` method. If the check fails, `Errno::Notcapable` is returned.
    pub(crate) fn as_descriptor_mut(
        &mut self,
        rights_base: Rights,
        rights_inheriting: Rights,
    ) -> Result<&mut Descriptor> {
        self.validate_rights(rights_base, rights_inheriting)?;
        Ok(&mut self.descriptor)
    }

    /// Check if this `Entry` object satisfies the specified base rights `rights_base`, and
    /// inheriting rights `rights_inheriting`; i.e., if rights attached to this `Entry` object
    /// are a superset.
    ///
    /// Upon unsuccessful check, `Errno::Notcapable` is returned.
    pub(crate) fn validate_rights(
        &self,
        rights_base: Rights,
        rights_inheriting: Rights,
    ) -> Result<()> {
        let missing_base = !self.rights_base & rights_base;
        let missing_inheriting = !self.rights_inheriting & rights_inheriting;
        if missing_base != Rights::empty() || missing_inheriting != Rights::empty() {
            log::trace!(
                "     | validate_rights failed: required: \
                 rights_base = {}, rights_inheriting = {}; \
                 actual: rights_base = {}, rights_inheriting = {}; \
                 missing_base = {}, missing_inheriting = {}",
                rights_base,
                rights_inheriting,
                self.rights_base,
                self.rights_inheriting,
                missing_base,
                missing_inheriting
            );
            Err(Errno::Notcapable)
        } else {
            Ok(())
        }
    }

    /// Test whether this descriptor is considered a tty within WASI.
    /// Note that since WASI itself lacks an `isatty` syscall and relies
    /// on a conservative approximation, we use the same approximation here.
    pub(crate) fn isatty(&self) -> bool {
        self.file_type == Filetype::CharacterDevice
            && (self.rights_base & (Rights::FD_SEEK | Rights::FD_TELL)) == Rights::empty()
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
