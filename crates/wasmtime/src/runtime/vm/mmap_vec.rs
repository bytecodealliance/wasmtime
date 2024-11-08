use crate::prelude::*;
#[cfg(feature = "signals-based-traps")]
use crate::runtime::vm::Mmap;
use alloc::sync::Arc;
use core::ops::{Deref, DerefMut, Range};
#[cfg(feature = "std")]
use std::fs::File;

/// A type akin to `Vec<u8>`, but backed by `mmap` and able to be split.
///
/// This type is a non-growable owned list of bytes. It can be segmented into
/// disjoint separately owned views akin to the `split_at` method on slices in
/// Rust. An `MmapVec` is backed by an OS-level memory allocation and is not
/// suitable for lots of small allocation (since it works at the page
/// granularity).
///
/// An `MmapVec` is an owned value which means that owners have the ability to
/// get exclusive access to the underlying bytes, enabling mutation.
///
/// TODO: rename this type and reword docs now that this isn't always an mmap.
pub struct MmapVec {
    mmap: Arc<MmapVecStorage>,
    range: Range<usize>,
}

enum MmapVecStorage {
    #[cfg(not(feature = "signals-based-traps"))]
    Vec(Vec<u8>),
    #[cfg(feature = "signals-based-traps")]
    Mmap(Mmap),
}

impl MmapVec {
    /// Consumes an existing `mmap` and wraps it up into an `MmapVec`.
    ///
    /// The returned `MmapVec` will have the `size` specified, which can be
    /// smaller than the region mapped by the `Mmap`. The returned `MmapVec`
    /// will only have at most `size` bytes accessible.
    #[cfg(feature = "signals-based-traps")]
    fn new_mmap(mmap: Mmap, size: usize) -> MmapVec {
        assert!(size <= mmap.len());
        MmapVec {
            mmap: Arc::new(MmapVecStorage::Mmap(mmap)),
            range: 0..size,
        }
    }

    #[cfg(not(feature = "signals-based-traps"))]
    fn new_vec(vec: Vec<u8>) -> MmapVec {
        MmapVec {
            range: 0..vec.len(),
            mmap: Arc::new(MmapVecStorage::Vec(vec)),
        }
    }

    /// Creates a new zero-initialized `MmapVec` with the given `size`.
    ///
    /// This commit will return a new `MmapVec` suitably sized to hold `size`
    /// bytes. All bytes will be initialized to zero since this is a fresh OS
    /// page allocation.
    pub fn with_capacity(size: usize) -> Result<MmapVec> {
        #[cfg(feature = "signals-based-traps")]
        return Ok(MmapVec::new_mmap(Mmap::with_at_least(size)?, size));
        #[cfg(not(feature = "signals-based-traps"))]
        return Ok(MmapVec::new_vec(vec![0; size]));
    }

    /// Creates a new `MmapVec` from the contents of an existing `slice`.
    ///
    /// A new `MmapVec` is allocated to hold the contents of `slice` and then
    /// `slice` is copied into the new mmap. It's recommended to avoid this
    /// method if possible to avoid the need to copy data around.
    pub fn from_slice(slice: &[u8]) -> Result<MmapVec> {
        let mut result = MmapVec::with_capacity(slice.len())?;
        result.copy_from_slice(slice);
        Ok(result)
    }

    /// Creates a new `MmapVec` which is the given `File` mmap'd into memory.
    ///
    /// This function will determine the file's size and map the full contents
    /// into memory. This will return an error if the file is too large to be
    /// fully mapped into memory.
    #[cfg(feature = "std")]
    pub fn from_file(file: File) -> Result<MmapVec> {
        let file = Arc::new(file);
        let mmap = Mmap::from_file(Arc::clone(&file))
            .with_context(move || format!("failed to create mmap for file {file:?}"))?;
        let len = mmap.len();
        Ok(MmapVec::new(mmap, len))
    }

    /// Makes the specified `range` within this `mmap` to be read/execute.
    #[cfg(feature = "signals-based-traps")]
    pub unsafe fn make_executable(
        &self,
        range: Range<usize>,
        enable_branch_protection: bool,
    ) -> Result<()> {
        assert!(range.start <= range.end);
        assert!(range.end <= self.range.len());
        let mmap = match &*self.mmap {
            MmapVecStorage::Mmap(m) => m,
        };
        mmap.make_executable(
            range.start + self.range.start..range.end + self.range.start,
            enable_branch_protection,
        )
    }

    /// Makes the specified `range` within this `mmap` to be read-only.
    #[cfg(feature = "signals-based-traps")]
    pub unsafe fn make_readonly(&self, range: Range<usize>) -> Result<()> {
        assert!(range.start <= range.end);
        assert!(range.end <= self.range.len());
        let mmap = match &*self.mmap {
            MmapVecStorage::Mmap(m) => m,
        };
        mmap.make_readonly(range.start + self.range.start..range.end + self.range.start)
    }

    /// Returns the underlying file that this mmap is mapping, if present.
    #[cfg(feature = "std")]
    pub fn original_file(&self) -> Option<&Arc<File>> {
        match &*self.mmap {
            #[cfg(not(feature = "signals-based-traps"))]
            MmapVecStorage::Vec(_) => None,
            #[cfg(feature = "signals-based-traps")]
            MmapVecStorage::Mmap(m) => m.original_file(),
        }
    }

    /// Returns the offset within the original mmap that this `MmapVec` is
    /// created from.
    pub fn original_offset(&self) -> usize {
        self.range.start
    }

    /// Returns the bounds, in host memory, of where this mmap
    /// image resides.
    pub fn image_range(&self) -> Range<*const u8> {
        let base = self.as_ptr();
        let len = self.len();
        base..base.wrapping_add(len)
    }
}

impl Deref for MmapVec {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &[u8] {
        match &*self.mmap {
            #[cfg(not(feature = "signals-based-traps"))]
            MmapVecStorage::Vec(v) => v,
            #[cfg(feature = "signals-based-traps")]
            MmapVecStorage::Mmap(m) => {
                // SAFETY: this mmap owns its own range of the underlying mmap so it
                // should be all good-to-read.
                unsafe { m.slice(self.range.clone()) }
            }
        }
    }
}

impl DerefMut for MmapVec {
    fn deref_mut(&mut self) -> &mut [u8] {
        // SAFETY: The underlying mmap is protected behind an `Arc` which means
        // there there can be many references to it. We are guaranteed, though,
        // that each reference to the underlying `mmap` has a disjoint `range`
        // listed that it can access. This means that despite having shared
        // access to the mmap itself we have exclusive ownership of the bytes
        // specified in `self.range`. This should allow us to safely hand out
        // mutable access to these bytes if so desired.
        unsafe {
            let slice = match &*self.mmap {
                #[cfg(not(feature = "signals-based-traps"))]
                MmapVecStorage::Vec(v) => {
                    core::slice::from_raw_parts_mut(v.as_ptr().cast_mut(), v.len())
                }
                #[cfg(feature = "signals-based-traps")]
                MmapVecStorage::Mmap(m) => {
                    core::slice::from_raw_parts_mut(m.as_ptr().cast_mut(), m.len())
                }
            };
            &mut slice[self.range.clone()]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::MmapVec;

    #[test]
    fn smoke() {
        let mut mmap = MmapVec::with_capacity(10).unwrap();
        assert_eq!(mmap.len(), 10);
        assert_eq!(&mmap[..], &[0; 10]);

        mmap[0] = 1;
        mmap[2] = 3;
        assert!(mmap.get(10).is_none());
        assert_eq!(mmap[0], 1);
        assert_eq!(mmap[2], 3);
    }
}
