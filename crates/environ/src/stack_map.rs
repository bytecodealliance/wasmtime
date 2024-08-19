use cranelift_bitset::CompoundBitSet;
use serde_derive::{Deserialize, Serialize};

/// A map for determining where live GC references live in a stack frame.
///
/// Note that this is currently primarily documented as cranelift's
/// `binemit::StackMap`, so for detailed documentation about this please read
/// the docs over there.
#[derive(Debug, Serialize, Deserialize)]
pub struct StackMap {
    bits: CompoundBitSet,
    frame_size: u32,
}

impl StackMap {
    /// Creates a new `StackMap`, typically from a preexisting
    /// `binemit::StackMap`.
    pub fn new(frame_size: u32, bits: CompoundBitSet) -> StackMap {
        StackMap { bits, frame_size }
    }

    /// Returns the byte size of this stack map's frame.
    pub fn frame_size(&self) -> u32 {
        self.frame_size
    }

    /// Given a frame pointer, get the stack pointer.
    ///
    /// # Safety
    ///
    /// The `fp` must be the frame pointer at the code offset that this stack
    /// map is associated with.
    pub unsafe fn sp(&self, fp: *mut usize) -> *mut usize {
        let frame_size = usize::try_from(self.frame_size).unwrap();
        fp.byte_sub(frame_size)
    }

    /// Given the stack pointer, get a reference to each live GC reference in
    /// the stack frame.
    ///
    /// # Safety
    ///
    /// The `sp` must be the stack pointer at the code offset that this stack
    /// map is associated with.
    pub unsafe fn live_gc_refs(&self, sp: *mut usize) -> impl Iterator<Item = *mut u32> + '_ {
        self.bits.iter().map(move |i| {
            log::trace!("Live GC ref in frame at frame offset {:#x}", i);
            let ptr_to_gc_ref = sp.byte_add(i);

            // Assert that the pointer is inside this stack map's frame.
            assert!({
                let delta = ptr_to_gc_ref as usize - sp as usize;
                let frame_size = usize::try_from(self.frame_size).unwrap();
                delta < frame_size
            });

            ptr_to_gc_ref.cast::<u32>()
        })
    }
}
