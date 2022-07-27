//! Generate various kinds of Wasm memory.

use anyhow::Result;
use arbitrary::{Arbitrary, Unstructured};
use wasmtime::{LinearMemory, MemoryCreator, MemoryType};

/// Configuration for linear memories in Wasmtime.
#[derive(Arbitrary, Clone, Debug, Eq, Hash, PartialEq)]
pub enum MemoryConfig {
    /// Configuration for linear memories which correspond to normal
    /// configuration settings in `wasmtime` itself. This will tweak various
    /// parameters about static/dynamic memories.
    Normal(NormalMemoryConfig),

    /// Configuration to force use of a linear memory that's unaligned at its
    /// base address to force all wasm addresses to be unaligned at the hardware
    /// level, even if the wasm itself correctly aligns everything internally.
    CustomUnaligned,
}

/// Represents a normal memory configuration for Wasmtime with the given
/// static and dynamic memory sizes.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[allow(missing_docs)]
pub struct NormalMemoryConfig {
    pub static_memory_maximum_size: Option<u64>,
    pub static_memory_guard_size: Option<u64>,
    pub dynamic_memory_guard_size: Option<u64>,
    pub guard_before_linear_memory: bool,
}

impl<'a> Arbitrary<'a> for NormalMemoryConfig {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        // This attempts to limit memory and guard sizes to 32-bit ranges so
        // we don't exhaust a 64-bit address space easily.
        let mut ret = Self {
            static_memory_maximum_size: <Option<u32> as Arbitrary>::arbitrary(u)?.map(Into::into),
            static_memory_guard_size: <Option<u32> as Arbitrary>::arbitrary(u)?.map(Into::into),
            dynamic_memory_guard_size: <Option<u32> as Arbitrary>::arbitrary(u)?.map(Into::into),
            guard_before_linear_memory: u.arbitrary()?,
        };

        if let Some(dynamic) = ret.dynamic_memory_guard_size {
            let statik = ret.static_memory_guard_size.unwrap_or(2 << 30);
            ret.static_memory_guard_size = Some(statik.max(dynamic));
        }
        Ok(ret)
    }
}

/// A custom "linear memory allocator" for wasm which only works with the
/// "dynamic" mode of configuration where wasm always does explicit bounds
/// checks.
///
/// This memory attempts to always use unaligned host addresses for the base
/// address of linear memory with wasm. This means that all jit loads/stores
/// should be unaligned, which is a "big hammer way" of testing that all our JIT
/// code works with unaligned addresses since alignment is not required for
/// correctness in wasm itself.
pub struct UnalignedMemory {
    /// This memory is always one byte larger than the actual size of linear
    /// memory.
    src: Vec<u8>,
    maximum: Option<usize>,
}

unsafe impl LinearMemory for UnalignedMemory {
    fn byte_size(&self) -> usize {
        // Chop off the extra byte reserved for the true byte size of this
        // linear memory.
        self.src.len() - 1
    }

    fn maximum_byte_size(&self) -> Option<usize> {
        self.maximum
    }

    fn grow_to(&mut self, new_size: usize) -> Result<()> {
        // Make sure to allocate an extra byte for our "unalignment"
        self.src.resize(new_size + 1, 0);
        Ok(())
    }

    fn as_ptr(&self) -> *mut u8 {
        // Return our allocated memory, offset by one, so that the base address
        // of memory is always unaligned.
        self.src[1..].as_ptr() as *mut _
    }
}

/// A mechanism to generate [`UnalignedMemory`] at runtime.
pub struct UnalignedMemoryCreator;

unsafe impl MemoryCreator for UnalignedMemoryCreator {
    fn new_memory(
        &self,
        _ty: MemoryType,
        minimum: usize,
        maximum: Option<usize>,
        reserved_size_in_bytes: Option<usize>,
        guard_size_in_bytes: usize,
    ) -> Result<Box<dyn LinearMemory>, String> {
        assert_eq!(guard_size_in_bytes, 0);
        assert!(reserved_size_in_bytes.is_none() || reserved_size_in_bytes == Some(0));
        Ok(Box::new(UnalignedMemory {
            src: vec![0; minimum + 1],
            maximum,
        }))
    }
}
