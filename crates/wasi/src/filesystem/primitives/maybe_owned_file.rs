use crate::filesystem::primitives::{OpenOptions, open_unchecked};
use std::ops::Deref;
use std::path::Component;
use std::{fmt, fs, io, mem};

/// Several places in the code need to be able to handle either owned or
/// borrowed [`std::fs::File]`s. Cloning a `File` to let them always have an
/// owned `File` is expensive and fallible, so use this `struct` to hold either
/// one, and implement [`Deref`] to allow them to be handled in a uniform way.
///
/// This is similar to [`Cow`], except without the copy-on-write part ;-).
/// `Cow` requires a `Clone` implementation, which `File` doesn't have, and
/// most users of this type don't need copy-on-write behavior.
///
/// And, this type has the special `descend_to`, which just does an assignment,
/// but also some useful assertion checks.
///
/// [`Deref`]: std::ops::Deref
/// [`Cow`]: std::borrow::Cow
pub(super) enum MaybeOwnedFile<'borrow> {
    Borrowed(&'borrow fs::File),
    Owned(fs::File),
}

impl<'borrow> MaybeOwnedFile<'borrow> {
    /// Constructs a new `MaybeOwnedFile` which is not owned.
    pub(super) fn borrowed(file: &'borrow fs::File) -> Self {
        Self::Borrowed(file)
    }

    /// Constructs a new `MaybeOwnedFile` which is owned.
    pub(super) fn owned(file: fs::File) -> Self {
        Self::Owned(file)
    }

    /// Set this `MaybeOwnedFile` to a new owned file which is from a subtree
    /// of the current file. Return a `MaybeOwnedFile` representing the
    /// previous state.
    pub(super) fn descend_to(&mut self, to: MaybeOwnedFile<'borrow>) -> Self {
        mem::replace(self, to)
    }

    /// Produce an owned `File`. This uses `open` on "." if needed to convert a
    /// borrowed `File` to an owned one.
    #[cfg_attr(windows, allow(dead_code))]
    pub(super) fn into_file(self, options: &OpenOptions) -> io::Result<fs::File> {
        match self {
            Self::Owned(file) => Ok(file),
            Self::Borrowed(file) => {
                // The only situation in which we'd be asked to produce an owned
                // `File` is when there's a need to open "." within a directory
                // to obtain a new handle.
                open_unchecked(file, Component::CurDir.as_ref(), options).map_err(Into::into)
            }
        }
    }
}

impl<'borrow> Deref for MaybeOwnedFile<'borrow> {
    type Target = fs::File;

    #[inline]
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Borrowed(file) => file,
            Self::Owned(file) => file,
        }
    }
}

impl<'borrow> fmt::Debug for MaybeOwnedFile<'borrow> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.deref().fmt(f)
    }
}
