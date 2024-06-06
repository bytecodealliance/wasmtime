//! Generate various kinds of Wasm memory.

use anyhow::Result;
use arbitrary::{Arbitrary, Unstructured};
use std::ops::Range;
use wasmtime::{LinearMemory, MemoryCreator, MemoryType};

/// A description of a memory config, image, etc... that can be used to test
/// memory accesses.
#[derive(Debug)]
pub struct MemoryAccesses {
    /// The configuration to use with this test case.
    pub config: crate::generators::Config,
    /// The heap image to use with this test case.
    pub image: HeapImage,
    /// The offset immediate to encode in the `load{8,16,32,64}` functions'
    /// various load instructions.
    pub offset: u32,
    /// The amount (in pages) to grow the memory.
    pub growth: u32,
}

impl<'a> Arbitrary<'a> for MemoryAccesses {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(MemoryAccesses {
            config: u.arbitrary()?,
            image: u.arbitrary()?,
            offset: u.arbitrary()?,
            // Don't grow too much, since oss-fuzz/asan get upset if we try,
            // even if we allow it to fail.
            growth: u.int_in_range(0..=10)?,
        })
    }
}

/// A memory heap image.
pub struct HeapImage {
    /// The minimum size (in pages) of this memory.
    pub minimum: u32,
    /// The maximum size (in pages) of this memory.
    pub maximum: Option<u32>,
    /// Whether this memory should be indexed with `i64` (rather than `i32`).
    pub memory64: bool,
    /// Data segments for this memory.
    pub segments: Vec<(u32, Vec<u8>)>,
}

impl std::fmt::Debug for HeapImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        struct Segments<'a>(&'a [(u32, Vec<u8>)]);
        impl std::fmt::Debug for Segments<'_> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "[..; {}]", self.0.len())
            }
        }

        f.debug_struct("HeapImage")
            .field("minimum", &self.minimum)
            .field("maximum", &self.maximum)
            .field("memory64", &self.memory64)
            .field("segments", &Segments(&self.segments))
            .finish()
    }
}

impl<'a> Arbitrary<'a> for HeapImage {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let minimum = u.int_in_range(0..=4)?;
        let maximum = if u.arbitrary()? {
            Some(u.int_in_range(minimum..=10)?)
        } else {
            None
        };
        let memory64 = u.arbitrary()?;
        let mut segments = vec![];
        if minimum > 0 {
            for _ in 0..u.int_in_range(0..=4)? {
                const WASM_PAGE_SIZE: u32 = 65536;
                let last_addressable = WASM_PAGE_SIZE * minimum - 1;
                let offset = u.int_in_range(0..=last_addressable)?;
                let max_len =
                    std::cmp::min(u.len(), usize::try_from(last_addressable - offset).unwrap());
                let len = u.int_in_range(0..=max_len)?;
                let data = u.bytes(len)?.to_vec();
                segments.push((offset, data));
            }
        }
        Ok(HeapImage {
            minimum,
            maximum,
            memory64,
            segments,
        })
    }
}

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
    pub dynamic_memory_reserved_for_growth: Option<u64>,
    pub guard_before_linear_memory: bool,
    pub memory_init_cow: bool,
}

impl<'a> Arbitrary<'a> for NormalMemoryConfig {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        // This attempts to limit memory and guard sizes to 32-bit ranges so
        // we don't exhaust a 64-bit address space easily.
        let mut ret = Self {
            static_memory_maximum_size: <Option<u32> as Arbitrary>::arbitrary(u)?.map(Into::into),
            static_memory_guard_size: <Option<u32> as Arbitrary>::arbitrary(u)?.map(Into::into),
            dynamic_memory_guard_size: <Option<u32> as Arbitrary>::arbitrary(u)?.map(Into::into),
            dynamic_memory_reserved_for_growth: <Option<u32> as Arbitrary>::arbitrary(u)?
                .map(Into::into),
            guard_before_linear_memory: u.arbitrary()?,
            memory_init_cow: u.arbitrary()?,
        };

        if let Some(dynamic) = ret.dynamic_memory_guard_size {
            let statik = ret.static_memory_guard_size.unwrap_or(2 << 30);
            ret.static_memory_guard_size = Some(statik.max(dynamic));
        }

        Ok(ret)
    }
}

impl NormalMemoryConfig {
    /// Apply this memory configuration to the given `wasmtime::Config`.
    pub fn apply_to(&self, config: &mut wasmtime::Config) {
        config
            .static_memory_maximum_size(self.static_memory_maximum_size.unwrap_or(0))
            .static_memory_guard_size(self.static_memory_guard_size.unwrap_or(0))
            .dynamic_memory_guard_size(self.dynamic_memory_guard_size.unwrap_or(0))
            .dynamic_memory_reserved_for_growth(
                self.dynamic_memory_reserved_for_growth.unwrap_or(0),
            )
            .guard_before_linear_memory(self.guard_before_linear_memory)
            .memory_init_cow(self.memory_init_cow);
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

    fn wasm_accessible(&self) -> Range<usize> {
        let base = self.as_ptr() as usize;
        let len = self.byte_size();
        base..base + len
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
