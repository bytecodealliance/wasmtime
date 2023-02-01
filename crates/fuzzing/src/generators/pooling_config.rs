//! Generate instance limits for the pooling allocation strategy.

use arbitrary::{Arbitrary, Unstructured};

/// Configuration for `wasmtime::PoolingAllocationStrategy`.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[allow(missing_docs)]
pub struct PoolingAllocationConfig {
    pub max_unused_warm_slots: u32,
    pub instance_count: u32,
    pub instance_memories: u32,
    pub instance_tables: u32,
    pub instance_memory_pages: u64,
    pub instance_table_elements: u32,
    pub instance_size: usize,
    pub async_stack_zeroing: bool,
    pub async_stack_keep_resident: usize,
    pub linear_memory_keep_resident: usize,
    pub table_keep_resident: usize,
}

impl PoolingAllocationConfig {
    /// Convert the generated limits to Wasmtime limits.
    pub fn to_wasmtime(&self) -> wasmtime::PoolingAllocationConfig {
        let mut cfg = wasmtime::PoolingAllocationConfig::default();

        cfg.max_unused_warm_slots(self.max_unused_warm_slots)
            .instance_count(self.instance_count)
            .instance_memories(self.instance_memories)
            .instance_tables(self.instance_tables)
            .instance_memory_pages(self.instance_memory_pages)
            .instance_table_elements(self.instance_table_elements)
            .instance_size(self.instance_size)
            .async_stack_zeroing(self.async_stack_zeroing)
            .async_stack_keep_resident(self.async_stack_keep_resident)
            .linear_memory_keep_resident(self.linear_memory_keep_resident)
            .table_keep_resident(self.table_keep_resident);
        cfg
    }
}

impl<'a> Arbitrary<'a> for PoolingAllocationConfig {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        const MAX_COUNT: u32 = 100;
        const MAX_TABLES: u32 = 10;
        const MAX_MEMORIES: u32 = 10;
        const MAX_ELEMENTS: u32 = 1000;
        const MAX_MEMORY_PAGES: u64 = 160; // 10 MiB
        const MAX_SIZE: usize = 1 << 20; // 1 MiB

        let instance_count = u.int_in_range(1..=MAX_COUNT)?;

        Ok(Self {
            max_unused_warm_slots: u.int_in_range(0..=instance_count + 10)?,
            instance_tables: u.int_in_range(0..=MAX_TABLES)?,
            instance_memories: u.int_in_range(0..=MAX_MEMORIES)?,
            instance_table_elements: u.int_in_range(0..=MAX_ELEMENTS)?,
            instance_memory_pages: u.int_in_range(0..=MAX_MEMORY_PAGES)?,
            instance_count,
            instance_size: u.int_in_range(0..=MAX_SIZE)?,
            async_stack_zeroing: u.arbitrary()?,
            async_stack_keep_resident: u.int_in_range(0..=1 << 20)?,
            linear_memory_keep_resident: u.int_in_range(0..=1 << 20)?,
            table_keep_resident: u.int_in_range(0..=1 << 20)?,
        })
    }
}
