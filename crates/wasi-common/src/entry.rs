use crate::handle::{Filetype, Handle, HandleRights};
use crate::{Error, Result};
use std::ops::Deref;
use std::path::PathBuf;
use std::rc::Rc;

pub struct EntryHandle(Rc<dyn Handle>);

impl EntryHandle {
    #[allow(dead_code)]
    pub(crate) fn new<T: Handle + 'static>(handle: T) -> Self {
        Self(Rc::new(handle))
    }

    pub(crate) fn get(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}

impl std::fmt::Debug for EntryHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("EntryHandle").field("opaque", &()).finish()
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

/// An abstraction struct serving as a wrapper for a `Handle` object.
///
/// Here, the `handle` field stores an instance of `Handle` type (such as a file descriptor, or
/// stdin handle), and accessing it can only be done via the provided `Entry::as_handle` method
/// which require a set of base and inheriting rights to be specified, verifying whether the stored
/// `Handle` object is valid for the rights specified.
pub(crate) struct Entry {
    handle: EntryHandle,
    pub(crate) preopen_path: Option<PathBuf>,
    // TODO: directories
}

impl Entry {
    pub(crate) fn new(handle: EntryHandle) -> Self {
        let preopen_path = None;
        Self {
            handle,
            preopen_path,
        }
    }

    pub(crate) fn get_file_type(&self) -> Filetype {
        self.handle.get_file_type()
    }

    pub(crate) fn get_rights(&self) -> HandleRights {
        self.handle.get_rights()
    }

    pub(crate) fn set_rights(&self, rights: HandleRights) {
        self.handle.set_rights(rights)
    }

    /// Convert this `Entry` into a `Handle` object provided the specified
    /// `rights` rights are set on this `Entry` object.
    ///
    /// The `Entry` can only be converted into a valid `Handle` object if
    /// the specified set of base rights, and inheriting rights encapsulated within `rights`
    /// `HandleRights` structure is a subset of rights attached to this `Entry`. The check is
    /// performed using `Entry::validate_rights` method. If the check fails, `Error::Notcapable`
    /// is returned.
    pub(crate) fn as_handle(&self, rights: HandleRights) -> Result<EntryHandle> {
        self.validate_rights(rights)?;
        Ok(self.handle.get())
    }

    /// Check if this `Entry` object satisfies the specified `HandleRights`; i.e., if
    /// rights attached to this `Entry` object are a superset.
    ///
    /// Upon unsuccessful check, `Error::Notcapable` is returned.
    pub(crate) fn validate_rights(&self, rights: HandleRights) -> Result<()> {
        let this_rights = self.handle.get_rights();
        if this_rights.contains(rights) {
            Ok(())
        } else {
            tracing::trace!(
                required = tracing::field::display(rights),
                actual = tracing::field::display(this_rights),
                "validate_rights failed",
            );
            Err(Error::Notcapable)
        }
    }
}
