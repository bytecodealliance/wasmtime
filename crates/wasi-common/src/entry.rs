use crate::sys::dev_null;
use crate::sys::entry::{descriptor_as_oshandle, determine_type_and_access_rights, OsHandle};
use crate::virtfs::VirtualFile;
use crate::wasi::types::{Filetype, Rights};
use crate::wasi::{Errno, Result};
use std::cell::{Cell, RefCell};
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::ops::Deref;
use std::path::PathBuf;
use std::rc::Rc;
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
    /// Return an `OsHandle`, which may be a stream or socket file descriptor.
    pub(crate) fn as_os_handle<'descriptor>(&'descriptor self) -> OsHandleRef<'descriptor> {
        descriptor_as_oshandle(self)
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

/// Represents rights of an `Entry` entity, either already held or
/// required.
#[derive(Debug, Copy, Clone)]
pub(crate) struct EntryRights {
    pub(crate) base: Rights,
    pub(crate) inheriting: Rights,
}

impl EntryRights {
    pub(crate) fn new(base: Rights, inheriting: Rights) -> Self {
        Self { base, inheriting }
    }

    /// Create new `EntryRights` instance from `base` rights only, keeping
    /// `inheriting` set to none.
    pub(crate) fn from_base(base: Rights) -> Self {
        Self {
            base,
            inheriting: Rights::empty(),
        }
    }

    /// Create new `EntryRights` instance with both `base` and `inheriting`
    /// rights set to none.
    pub(crate) fn empty() -> Self {
        Self {
            base: Rights::empty(),
            inheriting: Rights::empty(),
        }
    }

    /// Check if `other` is a subset of those rights.
    pub(crate) fn contains(&self, other: &Self) -> bool {
        self.base.contains(&other.base) && self.inheriting.contains(&other.inheriting)
    }
}

impl fmt::Display for EntryRights {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "EntryRights {{ base: {}, inheriting: {} }}",
            self.base, self.inheriting
        )
    }
}

/// An abstraction struct serving as a wrapper for a host `Descriptor` object which requires
/// certain rights `rights` in order to be accessed correctly.
///
/// Here, the `descriptor` field stores the host `Descriptor` object (such as a file descriptor, or
/// stdin handle), and accessing it can only be done via the provided `Entry::as_descriptor` method
/// which require a set of base and inheriting rights to be specified, verifying whether the stored
/// `Descriptor` object is valid for the rights specified.
#[derive(Debug)]
pub(crate) struct Entry {
    pub(crate) file_type: Filetype,
    descriptor: Rc<RefCell<Descriptor>>,
    pub(crate) rights: Cell<EntryRights>,
    pub(crate) preopen_path: Option<PathBuf>,
    // TODO: directories
}

impl Entry {
    pub(crate) fn from(file: Descriptor) -> io::Result<Self> {
        match file {
            Descriptor::OsHandle(handle) => unsafe { determine_type_and_access_rights(&handle) }
                .map(|(file_type, rights)| Self {
                    file_type,
                    descriptor: Rc::new(RefCell::new(handle.into())),
                    rights: Cell::new(rights),
                    preopen_path: None,
                }),
            Descriptor::VirtualFile(virt) => {
                let file_type = virt.get_file_type();
                let rights = EntryRights::new(virt.get_rights_base(), virt.get_rights_inheriting());

                Ok(Self {
                    file_type,
                    descriptor: Rc::new(RefCell::new(virt.into())),
                    rights: Cell::new(rights),
                    preopen_path: None,
                })
            }
            Descriptor::Stdin | Descriptor::Stdout | Descriptor::Stderr => {
                panic!("implementation error, stdin/stdout/stderr Entry must not be constructed from Entry::from");
            }
        }
    }

    pub(crate) fn duplicate_stdin() -> io::Result<Self> {
        unsafe { determine_type_and_access_rights(&io::stdin()) }.map(|(file_type, rights)| Self {
            file_type,
            descriptor: Rc::new(RefCell::new(Descriptor::Stdin)),
            rights: Cell::new(rights),
            preopen_path: None,
        })
    }

    pub(crate) fn duplicate_stdout() -> io::Result<Self> {
        unsafe { determine_type_and_access_rights(&io::stdout()) }.map(|(file_type, rights)| Self {
            file_type,
            descriptor: Rc::new(RefCell::new(Descriptor::Stdout)),
            rights: Cell::new(rights),
            preopen_path: None,
        })
    }

    pub(crate) fn duplicate_stderr() -> io::Result<Self> {
        unsafe { determine_type_and_access_rights(&io::stderr()) }.map(|(file_type, rights)| Self {
            file_type,
            descriptor: Rc::new(RefCell::new(Descriptor::Stderr)),
            rights: Cell::new(rights),
            preopen_path: None,
        })
    }

    pub(crate) fn null() -> io::Result<Self> {
        Self::from(OsHandle::from(dev_null()?).into())
    }

    /// Convert this `Entry` into a host `Descriptor` object provided the specified
    /// `rights` rights are set on this `Entry` object.
    ///
    /// The `Entry` can only be converted into a valid `Descriptor` object if
    /// the specified set of base rights, and inheriting rights encapsulated within `rights`
    /// `EntryRights` structure is a subset of rights attached to this `Entry`. The check is
    /// performed using `Entry::validate_rights` method. If the check fails, `Errno::Notcapable`
    /// is returned.
    pub(crate) fn as_descriptor(&self, rights: &EntryRights) -> Result<Rc<RefCell<Descriptor>>> {
        self.validate_rights(rights)?;
        Ok(Rc::clone(&self.descriptor))
    }

    /// Check if this `Entry` object satisfies the specified `EntryRights`; i.e., if
    /// rights attached to this `Entry` object are a superset.
    ///
    /// Upon unsuccessful check, `Errno::Notcapable` is returned.
    pub(crate) fn validate_rights(&self, rights: &EntryRights) -> Result<()> {
        if self.rights.get().contains(rights) {
            Ok(())
        } else {
            log::trace!(
                "     | validate_rights failed: required rights = {}; actual rights = {}",
                rights,
                self.rights.get(),
            );
            Err(Errno::Notcapable)
        }
    }

    /// Test whether this descriptor is considered a tty within WASI.
    /// Note that since WASI itself lacks an `isatty` syscall and relies
    /// on a conservative approximation, we use the same approximation here.
    pub(crate) fn isatty(&self) -> bool {
        self.file_type == Filetype::CharacterDevice
            && self
                .rights
                .get()
                .contains(&EntryRights::from_base(Rights::FD_SEEK | Rights::FD_TELL))
    }
}
