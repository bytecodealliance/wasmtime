use anyhow::{Error, Result};
use object::write::{Object, WritableBuffer};
use std::ops::{Deref, DerefMut, Range, RangeTo};
use std::sync::Arc;
use wasmtime_runtime::Mmap;

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
pub struct MmapVec {
    mmap: Arc<Mmap>,
    range: Range<usize>,
}

impl MmapVec {
    /// Consumes an existing `mmap` and wraps it up into an `MmapVec`.
    ///
    /// The returned `MmapVec` will have the `size` specified, which can be
    /// smaller than the region mapped by the `Mmap`. The returned `MmapVec`
    /// will only have at most `size` bytes accessible.
    pub fn new(mmap: Mmap, size: usize) -> MmapVec {
        assert!(size <= mmap.len());
        MmapVec {
            mmap: Arc::new(mmap),
            range: 0..size,
        }
    }

    /// Creates a new zero-initialized `MmapVec` with the given `size`.
    ///
    /// This commit will return a new `MmapVec` suitably sized to hold `size`
    /// bytes. All bytes will be initialized to zero since this is a fresh OS
    /// page allocation.
    pub fn with_capacity(size: usize) -> Result<MmapVec> {
        Ok(MmapVec::new(Mmap::with_at_least(size)?, size))
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

    /// Creates a new `MmapVec` from serializing the specified `obj`.
    ///
    /// The returned `MmapVec` will contain the serialized version of `obj` and
    /// is sized appropriately to the exact size of the object serialized.
    pub fn from_obj(obj: Object) -> Result<MmapVec> {
        let mut result = ObjectMmap::default();
        match obj.emit(&mut result) {
            Ok(()) => {
                assert!(result.mmap.is_some(), "no reserve");
                let mmap = result.mmap.expect("reserve not called");
                assert_eq!(mmap.len(), result.len);
                Ok(mmap)
            }
            Err(e) => match result.err.take() {
                Some(original) => Err(original.context(e)),
                None => Err(e.into()),
            },
        }
    }

    /// "Drains" leading bytes up to the end specified in `range` from this
    /// `MmapVec`, returning a separately owned `MmapVec` which retains access
    /// to the bytes.
    ///
    /// This method is similar to the `Vec` type's `drain` method, except that
    /// the return value is not an iterator but rather a new `MmapVec`. The
    /// purpose of this method is the ability to split-off new `MmapVec` values
    /// which are sub-slices of the original one.
    ///
    /// Once data has been drained from an `MmapVec` it is no longer accessible
    /// from the original `MmapVec`, it's only accessible from the returned
    /// `MmapVec`. In other words ownership of the drain'd bytes is returned
    /// through the `MmapVec` return value.
    ///
    /// This `MmapVec` will shrink by `range.end` bytes, and it will only refer
    /// to the bytes that come after the drain range.
    ///
    /// This is an `O(1)` operation which does not involve copies.
    pub fn drain(&mut self, range: RangeTo<usize>) -> MmapVec {
        let amt = range.end;
        assert!(amt <= (self.range.end - self.range.start));

        // Create a new `MmapVec` which refers to the same underlying mmap, but
        // has a disjoint range from ours. Our own range is adjusted to be
        // disjoint just after `ret` is created.
        let ret = MmapVec {
            mmap: self.mmap.clone(),
            range: self.range.start..self.range.start + amt,
        };
        self.range.start += amt;
        return ret;
    }
}

impl Deref for MmapVec {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.mmap.as_slice()[self.range.clone()]
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
            let slice = std::slice::from_raw_parts_mut(self.mmap.as_mut_ptr(), self.mmap.len());
            &mut slice[self.range.clone()]
        }
    }
}

/// Helper struct to implement the `WritableBuffer` trait from the `object`
/// crate.
///
/// This enables writing an object directly into an mmap'd memory so it's
/// immediately usable for execution after compilation. This implementation
/// relies on a call to `reserve` happening once up front with all the needed
/// data, and the mmap internally does not attempt to grow afterwards.
#[derive(Default)]
struct ObjectMmap {
    mmap: Option<MmapVec>,
    len: usize,
    err: Option<Error>,
}

impl WritableBuffer for ObjectMmap {
    fn len(&self) -> usize {
        self.len
    }

    fn reserve(&mut self, additional: usize) -> Result<(), ()> {
        assert!(self.mmap.is_none(), "cannot reserve twice");
        self.mmap = match MmapVec::with_capacity(additional) {
            Ok(mmap) => Some(mmap),
            Err(e) => {
                self.err = Some(e);
                return Err(());
            }
        };
        Ok(())
    }

    fn resize(&mut self, new_len: usize, value: u8) {
        if new_len <= self.len {
            return;
        }
        let mmap = self.mmap.as_mut().expect("write before reserve");

        // new mmaps are automatically filled with zeros, so if we're asked to
        // fill with zeros then we can skip the actual fill step.
        if value != 0 {
            mmap[self.len..][..new_len - self.len].fill(value);
        }
        self.len = new_len;
    }

    fn write_bytes(&mut self, val: &[u8]) {
        let mmap = self.mmap.as_mut().expect("write before reserve");
        mmap[self.len..][..val.len()].copy_from_slice(val);
        self.len += val.len();
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

    #[test]
    fn drain() {
        let mut mmap = MmapVec::from_slice(&[1, 2, 3, 4]).unwrap();
        assert_eq!(mmap.len(), 4);
        assert!(mmap.drain(..0).is_empty());
        assert_eq!(mmap.len(), 4);
        let one = mmap.drain(..1);
        assert_eq!(one.len(), 1);
        assert_eq!(one[0], 1);
        assert_eq!(mmap.len(), 3);
        assert_eq!(&mmap[..], &[2, 3, 4]);
        drop(one);
        assert_eq!(mmap.len(), 3);

        let two = mmap.drain(..2);
        assert_eq!(two.len(), 2);
        assert_eq!(two[0], 2);
        assert_eq!(two[1], 3);
        assert_eq!(mmap.len(), 1);
        assert_eq!(mmap[0], 4);
        drop(two);
        assert!(mmap.drain(..0).is_empty());
        assert!(mmap.drain(..1).len() == 1);
        assert!(mmap.is_empty());
        assert!(mmap.drain(..0).is_empty());
    }
}
