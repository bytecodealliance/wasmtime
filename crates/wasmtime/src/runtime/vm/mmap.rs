//! Low-level abstraction for allocating and managing zero-filled pages
//! of memory.

use crate::runtime::vm::sys::mmap;
use crate::{prelude::*, vm::usize_is_multiple_of_host_page_size};
use core::ops::Range;
#[cfg(feature = "std")]
use std::{fs::File, sync::Arc};

/// A simple struct consisting of a page-aligned pointer to page-aligned
/// and initially-zeroed memory and a length.
#[derive(Debug)]
pub struct Mmap {
    sys: mmap::Mmap,
    #[cfg(feature = "std")]
    file: Option<Arc<File>>,
}

impl Mmap {
    /// Create a new `Mmap` pointing to at least `size` bytes of page-aligned
    /// accessible memory.
    pub fn with_at_least(size: usize) -> Result<Self> {
        let rounded_size = crate::runtime::vm::round_usize_up_to_host_pages(size)?;
        Self::accessible_reserved(rounded_size, rounded_size)
    }

    /// Creates a new `Mmap` by opening the file located at `path` and mapping
    /// it into memory.
    ///
    /// The memory is mapped in read-only mode for the entire file. If portions
    /// of the file need to be modified then the `region` crate can be use to
    /// alter permissions of each page.
    ///
    /// The memory mapping and the length of the file within the mapping are
    /// returned.
    #[cfg(feature = "std")]
    pub fn from_file(file: Arc<File>) -> Result<Self> {
        let sys = mmap::Mmap::from_file(&file)?;
        Ok(Mmap {
            sys,
            file: Some(file),
        })
    }

    /// Create a new `Mmap` pointing to `accessible_size` bytes of page-aligned
    /// accessible memory, within a reserved mapping of `mapping_size` bytes.
    /// `accessible_size` and `mapping_size` must be native page-size multiples.
    ///
    /// # Panics
    ///
    /// This function will panic if `accessible_size` is greater than
    /// `mapping_size` or if either of them are not page-aligned.
    pub fn accessible_reserved(accessible_size: usize, mapping_size: usize) -> Result<Self> {
        assert!(accessible_size <= mapping_size);
        assert!(usize_is_multiple_of_host_page_size(mapping_size));
        assert!(usize_is_multiple_of_host_page_size(accessible_size));

        if mapping_size == 0 {
            Ok(Mmap {
                sys: mmap::Mmap::new_empty(),
                #[cfg(feature = "std")]
                file: None,
            })
        } else if accessible_size == mapping_size {
            Ok(Mmap {
                sys: mmap::Mmap::new(mapping_size)
                    .context(format!("mmap failed to allocate {mapping_size:#x} bytes"))?,
                #[cfg(feature = "std")]
                file: None,
            })
        } else {
            let mut result = Mmap {
                sys: mmap::Mmap::reserve(mapping_size)
                    .context(format!("mmap failed to reserve {mapping_size:#x} bytes"))?,
                #[cfg(feature = "std")]
                file: None,
            };
            if accessible_size > 0 {
                result.make_accessible(0, accessible_size).context(format!(
                    "mmap failed to allocate {accessible_size:#x} bytes"
                ))?;
            }
            Ok(result)
        }
    }

    /// Make the memory starting at `start` and extending for `len` bytes
    /// accessible. `start` and `len` must be native page-size multiples and
    /// describe a range within `self`'s reserved memory.
    ///
    /// # Panics
    ///
    /// This function will panic if `start` or `len` is not page aligned or if
    /// either are outside the bounds of this mapping.
    pub fn make_accessible(&mut self, start: usize, len: usize) -> Result<()> {
        let page_size = crate::runtime::vm::host_page_size();
        assert_eq!(start & (page_size - 1), 0);
        assert_eq!(len & (page_size - 1), 0);
        assert!(len <= self.len());
        assert!(start <= self.len() - len);

        self.sys.make_accessible(start, len)
    }

    /// Return the allocated memory as a slice of u8.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the range of bytes is accessible to the
    /// program and additionally has previously been initialized.
    ///
    /// # Panics
    ///
    /// Panics of the `range` provided is outside of the limits of this mmap.
    #[inline]
    pub unsafe fn slice(&self, range: Range<usize>) -> &[u8] {
        assert!(range.start <= range.end);
        assert!(range.end <= self.len());
        core::slice::from_raw_parts(self.as_ptr().add(range.start), range.end - range.start)
    }

    /// Return the allocated memory as a mutable slice of u8.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the range of bytes is accessible to the
    /// program and additionally has previously been initialized.
    ///
    /// # Panics
    ///
    /// Panics of the `range` provided is outside of the limits of this mmap.
    pub unsafe fn slice_mut(&mut self, range: Range<usize>) -> &mut [u8] {
        assert!(range.start <= range.end);
        assert!(range.end <= self.len());
        core::slice::from_raw_parts_mut(self.as_mut_ptr().add(range.start), range.end - range.start)
    }

    /// Return the allocated memory as a pointer to u8.
    #[inline]
    pub fn as_ptr(&self) -> *const u8 {
        self.sys.as_ptr()
    }

    /// Return the allocated memory as a mutable pointer to u8.
    #[inline]
    pub fn as_mut_ptr(&self) -> *mut u8 {
        self.sys.as_mut_ptr()
    }

    /// Return the length of the allocated memory.
    ///
    /// This is the byte length of this entire mapping which includes both
    /// addressable and non-addressable memory.
    #[inline]
    pub fn len(&self) -> usize {
        self.sys.len()
    }

    /// Return whether any memory has been allocated or reserved.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Makes the specified `range` within this `Mmap` to be read/execute.
    ///
    /// # Unsafety
    ///
    /// This method is unsafe as it's generally not valid to simply make memory
    /// executable, so it's up to the caller to ensure that everything is in
    /// order and this doesn't overlap with other memory that should only be
    /// read or only read/write.
    ///
    /// # Panics
    ///
    /// Panics of `range` is out-of-bounds or not page-aligned.
    pub unsafe fn make_executable(
        &self,
        range: Range<usize>,
        enable_branch_protection: bool,
    ) -> Result<()> {
        assert!(range.start <= self.len());
        assert!(range.end <= self.len());
        assert!(range.start <= range.end);
        assert!(
            range.start % crate::runtime::vm::host_page_size() == 0,
            "changing of protections isn't page-aligned",
        );
        self.sys
            .make_executable(range, enable_branch_protection)
            .context("failed to make memory executable")
    }

    /// Makes the specified `range` within this `Mmap` to be readonly.
    pub unsafe fn make_readonly(&self, range: Range<usize>) -> Result<()> {
        assert!(range.start <= self.len());
        assert!(range.end <= self.len());
        assert!(range.start <= range.end);
        assert!(
            range.start % crate::runtime::vm::host_page_size() == 0,
            "changing of protections isn't page-aligned",
        );
        self.sys
            .make_readonly(range)
            .context("failed to make memory readonly")
    }

    /// Returns the underlying file that this mmap is mapping, if present.
    #[cfg(feature = "std")]
    pub fn original_file(&self) -> Option<&Arc<File>> {
        self.file.as_ref()
    }
}

fn _assert() {
    fn _assert_send_sync<T: Send + Sync>() {}
    _assert_send_sync::<Mmap>();
}
