//! Generate various kinds of Wasm memory.

use anyhow::Result;
use arbitrary::{Arbitrary, Unstructured};
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
        let image = HeapImage::arbitrary(u)?;

        // Don't grow too much, since oss-fuzz/asan get upset if we try,
        // even if we allow it to fail.
        let one_mib = 1 << 20; // 1 MiB
        let max_growth = one_mib / (1 << image.page_size_log2.unwrap_or(16));
        let mut growth: u32 = u.int_in_range(0..=max_growth)?;

        // Occasionally, round to a power of two, since these tend to be
        // interesting numbers that overlap with the host page size and things
        // like that.
        if growth > 0 && u.ratio(1, 20)? {
            growth = (growth - 1).next_power_of_two();
        }

        Ok(MemoryAccesses {
            config: u.arbitrary()?,
            image,
            offset: u.arbitrary()?,
            growth,
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
    /// The log2 of the page size for this memory.
    pub page_size_log2: Option<u32>,
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
            .field("page_size_log2", &self.page_size_log2)
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
        let page_size_log2 = match u.int_in_range(0..=2)? {
            0 => None,
            1 => Some(0),
            2 => Some(16),
            _ => unreachable!(),
        };
        let mut segments = vec![];
        if minimum > 0 {
            for _ in 0..u.int_in_range(0..=4)? {
                let last_addressable = (1u32 << page_size_log2.unwrap_or(16)) * minimum - 1;
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
            page_size_log2,
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
#[expect(missing_docs, reason = "self-describing fields")]
pub struct NormalMemoryConfig {
    pub memory_reservation: Option<u64>,
    pub memory_guard_size: Option<u64>,
    pub memory_reservation_for_growth: Option<u64>,
    pub guard_before_linear_memory: bool,
    pub cranelift_enable_heap_access_spectre_mitigations: Option<bool>,
    pub memory_init_cow: bool,
}

impl<'a> Arbitrary<'a> for NormalMemoryConfig {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(Self {
            // Allow up to 8GiB reservations of the virtual address space for
            // the initial memory reservation.
            memory_reservation: interesting_virtual_memory_size(u, 33)?,

            // Allow up to 4GiB guard page reservations to be made.
            memory_guard_size: interesting_virtual_memory_size(u, 32)?,

            // Allow up up to 1GiB extra memory to grow into for dynamic
            // memories.
            memory_reservation_for_growth: interesting_virtual_memory_size(u, 30)?,

            guard_before_linear_memory: u.arbitrary()?,
            cranelift_enable_heap_access_spectre_mitigations: u.arbitrary()?,
            memory_init_cow: u.arbitrary()?,
        })
    }
}

/// Helper function to generate "interesting numbers" for virtual memory
/// configuration options that `Config` supports.
fn interesting_virtual_memory_size(
    u: &mut Unstructured<'_>,
    max_log2: u32,
) -> arbitrary::Result<Option<u64>> {
    // Most of the time return "none" meaning "use the default settings".
    if u.ratio(3, 4)? {
        return Ok(None);
    }

    // Otherwise do a split between various strategies.
    #[derive(Arbitrary)]
    enum Interesting {
        Zero,
        PowerOfTwo,
        Arbitrary,
    }

    let size = match u.arbitrary()? {
        Interesting::Zero => 0,
        Interesting::PowerOfTwo => 1 << u.int_in_range(0..=max_log2)?,
        Interesting::Arbitrary => u.int_in_range(0..=1 << max_log2)?,
    };
    Ok(Some(size))
}

impl NormalMemoryConfig {
    /// Apply this memory configuration to the given `wasmtime::Config`.
    pub fn apply_to(&self, config: &mut wasmtime::Config) {
        if let Some(n) = self.memory_reservation {
            config.memory_reservation(n);
        }
        if let Some(n) = self.memory_guard_size {
            config.memory_guard_size(n);
        }
        if let Some(n) = self.memory_reservation_for_growth {
            config.memory_reservation_for_growth(n);
        }

        config
            .guard_before_linear_memory(self.guard_before_linear_memory)
            .memory_init_cow(self.memory_init_cow);

        if let Some(enable) = self.cranelift_enable_heap_access_spectre_mitigations {
            unsafe {
                config.cranelift_flag_set(
                    "enable_heap_access_spectre_mitigation",
                    &enable.to_string(),
                );
            }
        }
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
}

unsafe impl LinearMemory for UnalignedMemory {
    fn byte_size(&self) -> usize {
        // Chop off the extra byte reserved for the true byte size of this
        // linear memory.
        self.src.len() - 1
    }

    fn byte_capacity(&self) -> usize {
        self.src.capacity() - 1
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
        _maximum: Option<usize>,
        reserved_size_in_bytes: Option<usize>,
        guard_size_in_bytes: usize,
    ) -> Result<Box<dyn LinearMemory>, String> {
        assert_eq!(guard_size_in_bytes, 0);
        assert!(reserved_size_in_bytes.is_none() || reserved_size_in_bytes == Some(0));
        Ok(Box::new(UnalignedMemory {
            src: vec![0; minimum + 1],
        }))
    }
}
