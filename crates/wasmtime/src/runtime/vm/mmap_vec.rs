use crate::prelude::*;
#[cfg(not(feature = "signals-based-traps"))]
use crate::runtime::vm::send_sync_ptr::SendSyncPtr;
#[cfg(feature = "signals-based-traps")]
use crate::runtime::vm::{mmap::UnalignedLength, Mmap};
#[cfg(not(feature = "signals-based-traps"))]
use alloc::alloc::Layout;
use alloc::sync::Arc;
use core::ops::{Deref, Range};
#[cfg(not(feature = "signals-based-traps"))]
use core::ptr::NonNull;
#[cfg(feature = "std")]
use std::fs::File;

/// A type which prefers to store backing memory in an OS-backed memory mapping
/// but can fall back to the regular memory allocator as well.
///
/// This type is used to store code in Wasmtime and manage read-only and
/// executable permissions of compiled images. This is created from either an
/// in-memory compilation or by deserializing an artifact from disk. Methods
/// are provided for managing VM permissions when the `signals-based-traps`
/// Cargo feature is enabled.
///
/// The length of an `MmapVec` is not guaranteed to be page-aligned. That means
/// that if the contents are not themselves page-aligned, which compiled images
/// are typically not, then the remaining bytes in the final page for
/// mmap-backed instances are unused.
///
/// Note that when `signals-based-traps` is disabled then this type is
/// backed by the regular memory allocator via `alloc` APIs. In such a
/// scenario this type does not support read-only or executable bits
/// and the methods are not available. However, the `CustomCodeMemory`
/// mechanism may be used by the embedder to set up and tear down
/// executable permissions on parts of this storage.
pub enum MmapVec {
    #[doc(hidden)]
    #[cfg(not(feature = "signals-based-traps"))]
    Alloc {
        base: SendSyncPtr<u8>,
        layout: Layout,
    },
    #[doc(hidden)]
    #[cfg(feature = "signals-based-traps")]
    Mmap {
        mmap: Mmap<UnalignedLength>,
        len: usize,
    },
}

impl MmapVec {
    /// Consumes an existing `mmap` and wraps it up into an `MmapVec`.
    ///
    /// The returned `MmapVec` will have the `size` specified, which can be
    /// smaller than the region mapped by the `Mmap`. The returned `MmapVec`
    /// will only have at most `size` bytes accessible.
    #[cfg(feature = "signals-based-traps")]
    fn new_mmap<M>(mmap: M, len: usize) -> MmapVec
    where
        M: Into<Mmap<UnalignedLength>>,
    {
        let mmap = mmap.into();
        assert!(len <= mmap.len());
        MmapVec::Mmap { mmap, len }
    }

    #[cfg(not(feature = "signals-based-traps"))]
    fn new_alloc(len: usize, alignment: usize) -> MmapVec {
        let layout = Layout::from_size_align(len, alignment)
            .expect("Invalid size or alignment for MmapVec allocation");
        let base = SendSyncPtr::new(
            NonNull::new(unsafe { alloc::alloc::alloc_zeroed(layout.clone()) })
                .expect("Allocation of MmapVec storage failed"),
        );
        MmapVec::Alloc { base, layout }
    }

    /// Creates a new zero-initialized `MmapVec` with the given `size`
    /// and `alignment`.
    ///
    /// This commit will return a new `MmapVec` suitably sized to hold `size`
    /// bytes. All bytes will be initialized to zero since this is a fresh OS
    /// page allocation.
    pub fn with_capacity_and_alignment(size: usize, alignment: usize) -> Result<MmapVec> {
        #[cfg(feature = "signals-based-traps")]
        {
            assert!(alignment <= crate::runtime::vm::host_page_size());
            return Ok(MmapVec::new_mmap(Mmap::with_at_least(size)?, size));
        }
        #[cfg(not(feature = "signals-based-traps"))]
        {
            return Ok(MmapVec::new_alloc(size, alignment));
        }
    }

    /// Creates a new `MmapVec` from the contents of an existing `slice`.
    ///
    /// A new `MmapVec` is allocated to hold the contents of `slice` and then
    /// `slice` is copied into the new mmap. It's recommended to avoid this
    /// method if possible to avoid the need to copy data around.
    pub fn from_slice(slice: &[u8]) -> Result<MmapVec> {
        MmapVec::from_slice_with_alignment(slice, 1)
    }

    /// Creates a new `MmapVec` from the contents of an existing
    /// `slice`, with a minimum alignment.
    ///
    /// `align` must be a power of two. This is useful when page
    /// alignment is required when the system otherwise does not use
    /// virtual memory but has a custom code publish handler.
    ///
    /// A new `MmapVec` is allocated to hold the contents of `slice` and then
    /// `slice` is copied into the new mmap. It's recommended to avoid this
    /// method if possible to avoid the need to copy data around.  pub
    pub fn from_slice_with_alignment(slice: &[u8], align: usize) -> Result<MmapVec> {
        let mut result = MmapVec::with_capacity_and_alignment(slice.len(), align)?;
        // SAFETY: The mmap hasn't been made readonly yet so this should be
        // safe to call.
        unsafe {
            result.as_mut_slice().copy_from_slice(slice);
        }
        Ok(result)
    }

    /// Creates a new `MmapVec` which is the given `File` mmap'd into memory.
    ///
    /// This function will determine the file's size and map the full contents
    /// into memory. This will return an error if the file is too large to be
    /// fully mapped into memory.
    ///
    /// The file is mapped into memory with a "private mapping" meaning that
    /// changes are not persisted back to the file itself and are only visible
    /// within this process.
    #[cfg(feature = "std")]
    pub fn from_file(file: File) -> Result<MmapVec> {
        let file = Arc::new(file);
        let mmap = Mmap::from_file(Arc::clone(&file))
            .with_context(move || format!("failed to create mmap for file {file:?}"))?;
        let len = mmap.len();
        Ok(MmapVec::new_mmap(mmap, len))
    }

    /// Makes the specified `range` within this `mmap` to be read/execute.
    #[cfg(feature = "signals-based-traps")]
    pub unsafe fn make_executable(
        &self,
        range: Range<usize>,
        enable_branch_protection: bool,
    ) -> Result<()> {
        let (mmap, len) = match self {
            MmapVec::Mmap { mmap, len } => (mmap, *len),
        };
        assert!(range.start <= range.end);
        assert!(range.end <= len);
        mmap.make_executable(range.start..range.end, enable_branch_protection)
    }

    /// Makes the specified `range` within this `mmap` to be read-only.
    #[cfg(feature = "signals-based-traps")]
    pub unsafe fn make_readonly(&self, range: Range<usize>) -> Result<()> {
        let (mmap, len) = match self {
            MmapVec::Mmap { mmap, len } => (mmap, *len),
        };
        assert!(range.start <= range.end);
        assert!(range.end <= len);
        mmap.make_readonly(range.start..range.end)
    }

    /// Returns the underlying file that this mmap is mapping, if present.
    #[cfg(feature = "std")]
    pub fn original_file(&self) -> Option<&Arc<File>> {
        match self {
            #[cfg(not(feature = "signals-based-traps"))]
            MmapVec::Alloc { .. } => None,
            #[cfg(feature = "signals-based-traps")]
            MmapVec::Mmap { mmap, .. } => mmap.original_file(),
        }
    }

    /// Returns the bounds, in host memory, of where this mmap
    /// image resides.
    pub fn image_range(&self) -> Range<*const u8> {
        let base = self.as_ptr();
        let len = self.len();
        base..base.wrapping_add(len)
    }

    /// Views this region of memory as a mutable slice.
    ///
    /// # Unsafety
    ///
    /// This method is only safe if `make_readonly` hasn't been called yet to
    /// ensure that the memory is indeed writable
    pub unsafe fn as_mut_slice(&mut self) -> &mut [u8] {
        match self {
            #[cfg(not(feature = "signals-based-traps"))]
            MmapVec::Alloc { base, layout } => {
                core::slice::from_raw_parts_mut(base.as_mut(), layout.size())
            }
            #[cfg(feature = "signals-based-traps")]
            MmapVec::Mmap { mmap, len } => mmap.slice_mut(0..*len),
        }
    }
}

impl Deref for MmapVec {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &[u8] {
        match self {
            #[cfg(not(feature = "signals-based-traps"))]
            MmapVec::Alloc { base, layout } => unsafe {
                core::slice::from_raw_parts(base.as_ptr(), layout.size())
            },
            #[cfg(feature = "signals-based-traps")]
            MmapVec::Mmap { mmap, len } => {
                // SAFETY: all bytes for this mmap, which is owned by
                // `MmapVec`, are always at least readable.
                unsafe { mmap.slice(0..*len) }
            }
        }
    }
}

impl Drop for MmapVec {
    fn drop(&mut self) {
        match self {
            #[cfg(not(feature = "signals-based-traps"))]
            MmapVec::Alloc { base, layout, .. } => unsafe {
                alloc::alloc::dealloc(base.as_mut(), layout.clone());
            },
            #[cfg(feature = "signals-based-traps")]
            MmapVec::Mmap { .. } => {
                // Drop impl on the `mmap` takes care of this case.
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::MmapVec;

    #[test]
    fn smoke() {
        let mut mmap = MmapVec::with_capacity_and_alignment(10, 1).unwrap();
        assert_eq!(mmap.len(), 10);
        assert_eq!(&mmap[..], &[0; 10]);

        unsafe {
            mmap.as_mut_slice()[0] = 1;
            mmap.as_mut_slice()[2] = 3;
        }
        assert!(mmap.get(10).is_none());
        assert_eq!(mmap[0], 1);
        assert_eq!(mmap[2], 3);
    }

    #[test]
    fn alignment() {
        let mmap = MmapVec::with_capacity_and_alignment(10, 4096).unwrap();
        let raw_ptr = &mmap[0] as *const _ as usize;
        assert_eq!(raw_ptr & (4096 - 1), 0);
    }
}
