//! Generate instance limits for the pooling allocation strategy.

use arbitrary::{Arbitrary, Unstructured};
use wasmtime::MpkEnabled;

/// Configuration for `wasmtime::PoolingAllocationStrategy`.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[allow(missing_docs)]
pub struct PoolingAllocationConfig {
    pub total_component_instances: u32,
    pub total_core_instances: u32,
    pub total_memories: u32,
    pub total_tables: u32,
    pub total_stacks: u32,

    pub max_memory_size: usize,
    pub table_elements: usize,

    pub component_instance_size: usize,
    pub max_memories_per_component: u32,
    pub max_tables_per_component: u32,

    pub core_instance_size: usize,
    pub max_memories_per_module: u32,
    pub max_tables_per_module: u32,

    pub table_keep_resident: usize,
    pub linear_memory_keep_resident: usize,

    pub decommit_batch_size: usize,
    pub max_unused_warm_slots: u32,

    pub async_stack_zeroing: bool,
    pub async_stack_keep_resident: usize,

    pub memory_protection_keys: MpkEnabled,
    pub max_memory_protection_keys: usize,
}

impl PoolingAllocationConfig {
    /// Convert the generated limits to Wasmtime limits.
    pub fn to_wasmtime(&self) -> wasmtime::PoolingAllocationConfig {
        let mut cfg = wasmtime::PoolingAllocationConfig::default();

        cfg.total_component_instances(self.total_component_instances);
        cfg.total_core_instances(self.total_core_instances);
        cfg.total_memories(self.total_memories);
        cfg.total_tables(self.total_tables);
        cfg.total_stacks(self.total_stacks);

        cfg.max_memory_size(self.max_memory_size);
        cfg.table_elements(self.table_elements);

        cfg.max_component_instance_size(self.component_instance_size);
        cfg.max_memories_per_component(self.max_memories_per_component);
        cfg.max_tables_per_component(self.max_tables_per_component);

        cfg.max_core_instance_size(self.core_instance_size);
        cfg.max_memories_per_module(self.max_memories_per_module);
        cfg.max_tables_per_module(self.max_tables_per_module);

        cfg.table_keep_resident(self.table_keep_resident);
        cfg.linear_memory_keep_resident(self.linear_memory_keep_resident);

        cfg.decommit_batch_size(self.decommit_batch_size);
        cfg.max_unused_warm_slots(self.max_unused_warm_slots);

        cfg.async_stack_zeroing(self.async_stack_zeroing);
        cfg.async_stack_keep_resident(self.async_stack_keep_resident);

        cfg.memory_protection_keys(self.memory_protection_keys);
        cfg.max_memory_protection_keys(self.max_memory_protection_keys);

        cfg
    }
}

impl<'a> Arbitrary<'a> for PoolingAllocationConfig {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        const MAX_COUNT: u32 = 100;
        const MAX_TABLES: u32 = 100;
        const MAX_MEMORIES: u32 = 100;
        const MAX_ELEMENTS: usize = 1000;
        const MAX_MEMORY_SIZE: usize = 10 * (1 << 20); // 10 MiB
        const MAX_SIZE: usize = 1 << 20; // 1 MiB
        const MAX_INSTANCE_MEMORIES: u32 = 10;
        const MAX_INSTANCE_TABLES: u32 = 10;

        let total_memories = u.int_in_range(1..=MAX_MEMORIES)?;

        Ok(Self {
            total_component_instances: u.int_in_range(1..=MAX_COUNT)?,
            total_core_instances: u.int_in_range(1..=MAX_COUNT)?,
            total_memories,
            total_tables: u.int_in_range(1..=MAX_TABLES)?,
            total_stacks: u.int_in_range(1..=MAX_COUNT)?,

            max_memory_size: u.int_in_range(0..=MAX_MEMORY_SIZE)?,
            table_elements: u.int_in_range(0..=MAX_ELEMENTS)?,

            component_instance_size: u.int_in_range(0..=MAX_SIZE)?,
            max_memories_per_component: u.int_in_range(1..=MAX_INSTANCE_MEMORIES)?,
            max_tables_per_component: u.int_in_range(1..=MAX_INSTANCE_TABLES)?,

            core_instance_size: u.int_in_range(0..=MAX_SIZE)?,
            max_memories_per_module: u.int_in_range(1..=MAX_INSTANCE_MEMORIES)?,
            max_tables_per_module: u.int_in_range(1..=MAX_INSTANCE_TABLES)?,

            table_keep_resident: u.int_in_range(0..=1 << 20)?,
            linear_memory_keep_resident: u.int_in_range(0..=1 << 20)?,

            decommit_batch_size: u.int_in_range(1..=1000)?,
            max_unused_warm_slots: u.int_in_range(0..=total_memories + 10)?,

            async_stack_zeroing: u.arbitrary()?,
            async_stack_keep_resident: u.int_in_range(0..=1 << 20)?,

            memory_protection_keys: *u.choose(&[MpkEnabled::Auto, MpkEnabled::Disable])?,
            max_memory_protection_keys: u.int_in_range(1..=20)?,
        })
    }
}
