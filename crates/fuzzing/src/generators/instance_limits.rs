//! Generate instance limits for the pooling allocation strategy.

use arbitrary::{Arbitrary, Unstructured};

/// Configuration for `wasmtime::PoolingAllocationStrategy`.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[allow(missing_docs)]
pub struct InstanceLimits {
    pub count: u32,
    pub memories: u32,
    pub tables: u32,
    pub memory_pages: u64,
    pub table_elements: u32,
    pub size: usize,
}

impl InstanceLimits {
    /// Convert the generated limits to Wasmtime limits.
    pub fn to_wasmtime(&self) -> wasmtime::InstanceLimits {
        wasmtime::InstanceLimits {
            count: self.count,
            memories: self.memories,
            tables: self.tables,
            memory_pages: self.memory_pages,
            table_elements: self.table_elements,
            size: self.size,
        }
    }
}

impl<'a> Arbitrary<'a> for InstanceLimits {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        const MAX_COUNT: u32 = 100;
        const MAX_TABLES: u32 = 10;
        const MAX_MEMORIES: u32 = 10;
        const MAX_ELEMENTS: u32 = 1000;
        const MAX_MEMORY_PAGES: u64 = 160; // 10 MiB
        const MAX_SIZE: usize = 1 << 20; // 1 MiB

        Ok(Self {
            tables: u.int_in_range(0..=MAX_TABLES)?,
            memories: u.int_in_range(0..=MAX_MEMORIES)?,
            table_elements: u.int_in_range(0..=MAX_ELEMENTS)?,
            memory_pages: u.int_in_range(0..=MAX_MEMORY_PAGES)?,
            count: u.int_in_range(1..=MAX_COUNT)?,
            size: u.int_in_range(0..=MAX_SIZE)?,
        })
    }
}
