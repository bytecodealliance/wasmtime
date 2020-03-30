use crate::handle::Handle;
use crate::wasi::types::{Filetype, Rights};
use crate::wasi::{Errno, Result};
use std::cell::Cell;
use std::ops::Deref;
use std::path::PathBuf;
use std::rc::Rc;
use std::{fmt, io};

pub(crate) struct EntryHandle(Rc<dyn Handle>);

impl EntryHandle {
    pub(crate) fn new<T: Handle + 'static>(handle: T) -> Self {
        Self(Rc::new(handle))
    }

    pub(crate) fn get(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}

impl From<Box<dyn Handle>> for EntryHandle {
    fn from(handle: Box<dyn Handle>) -> Self {
        Self(handle.into())
    }
}

impl Deref for EntryHandle {
    type Target = dyn Handle;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

/// An abstraction struct serving as a wrapper for a host `Descriptor` object which requires
/// certain rights `rights` in order to be accessed correctly.
///
/// Here, the `descriptor` field stores the host `Descriptor` object (such as a file descriptor, or
/// stdin handle), and accessing it can only be done via the provided `Entry::as_descriptor` method
/// which require a set of base and inheriting rights to be specified, verifying whether the stored
/// `Descriptor` object is valid for the rights specified.
pub(crate) struct Entry {
    pub(crate) file_type: Filetype,
    handle: EntryHandle,
    pub(crate) rights: Cell<EntryRights>,
    pub(crate) preopen_path: Option<PathBuf>,
    // TODO: directories
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

impl Entry {
    pub(crate) fn from(handle: EntryHandle) -> io::Result<Self> {
        let file_type = handle.get_file_type()?;
        let rights = handle.get_rights()?;
        Ok(Self {
            file_type,
            handle,
            rights: Cell::new(rights),
            preopen_path: None,
        })
    }

    /// Convert this `Entry` into a host `Descriptor` object provided the specified
    /// `rights` rights are set on this `Entry` object.
    ///
    /// The `Entry` can only be converted into a valid `Descriptor` object if
    /// the specified set of base rights, and inheriting rights encapsulated within `rights`
    /// `EntryRights` structure is a subset of rights attached to this `Entry`. The check is
    /// performed using `Entry::validate_rights` method. If the check fails, `Errno::Notcapable`
    /// is returned.
    pub(crate) fn as_handle(&self, rights: &EntryRights) -> Result<EntryHandle> {
        self.validate_rights(rights)?;
        Ok(self.handle.get())
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
