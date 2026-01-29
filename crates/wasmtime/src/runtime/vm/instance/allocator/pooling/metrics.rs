use core::sync::atomic::Ordering;

use crate::{Engine, vm::PoolingInstanceAllocator};

/// `PoolingAllocatorMetrics` provides access to runtime metrics of a pooling
/// allocator configured with [`crate::InstanceAllocationStrategy::Pooling`].
///
/// This is a cheap cloneable handle which can be obtained with
/// [`Engine::pooling_allocator_metrics`].
#[derive(Clone)]
pub struct PoolingAllocatorMetrics {
    engine: Engine,
}

impl PoolingAllocatorMetrics {
    pub(crate) fn new(engine: &Engine) -> Option<Self> {
        engine.allocator().as_pooling().map(|_| Self {
            engine: engine.clone(),
        })
    }

    /// Returns the number of core (module) instances currently allocated.
    pub fn core_instances(&self) -> u64 {
        self.allocator().live_core_instances.load(Ordering::Relaxed)
    }

    /// Returns the number of component instances currently allocated.
    pub fn component_instances(&self) -> u64 {
        self.allocator()
            .live_component_instances
            .load(Ordering::Relaxed)
    }

    /// Returns the number of WebAssembly memories currently allocated.
    pub fn memories(&self) -> usize {
        self.allocator().live_memories.load(Ordering::Relaxed)
    }

    /// Returns the number of WebAssembly tables currently allocated.
    pub fn tables(&self) -> usize {
        self.allocator().live_tables.load(Ordering::Relaxed)
    }

    /// Returns the number of WebAssembly stacks currently allocated.
    #[cfg(feature = "async")]
    pub fn stacks(&self) -> usize {
        self.allocator().live_stacks.load(Ordering::Relaxed)
    }

    /// Returns the number of WebAssembly GC heaps currently allocated.
    #[cfg(feature = "gc")]
    pub fn gc_heaps(&self) -> usize {
        self.allocator().live_gc_heaps.load(Ordering::Relaxed)
    }

    /// Returns the number of slots for linear memories in this allocator which
    /// are not currently in use but were previously used.
    ///
    /// A "warm" slot means that there was a previous instantiation of a memory
    /// in that slot. Warm slots are favored in general for allocating new
    /// memories over using a slot that has never been used before.
    pub fn unused_warm_memories(&self) -> u32 {
        self.allocator().memories.unused_warm_slots()
    }

    /// Returns the number of bytes in this pooling allocator which are not part
    /// of any in-used linear memory slot but were previously used and are kept
    /// resident via the `*_keep_resident` configuration options.
    pub fn unused_memory_bytes_resident(&self) -> usize {
        self.allocator().memories.unused_bytes_resident()
    }

    /// Returns the number of slots for tables in this allocator which are not
    /// currently in use but were previously used.
    ///
    /// A "warm" slot means that there was a previous instantiation of a table
    /// in that slot. Warm slots are favored in general for allocating new
    /// tables over using a slot that has never been used before.
    pub fn unused_warm_tables(&self) -> u32 {
        self.allocator().tables.unused_warm_slots()
    }

    /// Returns the number of bytes in this pooling allocator which are not part
    /// of any in-used linear table slot but were previously used and are kept
    /// resident via the `*_keep_resident` configuration options.
    pub fn unused_table_bytes_resident(&self) -> usize {
        self.allocator().tables.unused_bytes_resident()
    }

    /// Returns the number of slots for stacks in this allocator which are not
    /// currently in use but were previously used.
    ///
    /// A "warm" slot means that there was a previous use of a stack
    /// in that slot. Warm slots are favored in general for allocating new
    /// stacks over using a slot that has never been used before.
    #[cfg(feature = "async")]
    pub fn unused_warm_stacks(&self) -> u32 {
        self.allocator().stacks.unused_warm_slots()
    }

    /// Returns the number of bytes in this pooling allocator which are not part
    /// of any in-used linear stack slot but were previously used and are kept
    /// resident via the `*_keep_resident` configuration options.
    ///
    /// This returns `None` if the `async_stack_zeroing` option is disabled or
    /// if the platform doesn't manage stacks (e.g. Windows returns `None`).
    #[cfg(feature = "async")]
    pub fn unused_stack_bytes_resident(&self) -> Option<usize> {
        self.allocator().stacks.unused_bytes_resident()
    }

    fn allocator(&self) -> &PoolingInstanceAllocator {
        self.engine
            .allocator()
            .as_pooling()
            .expect("engine should have pooling allocator")
    }
}

#[cfg(test)]
mod tests {
    use crate::vm::instance::allocator::pooling::StackPool;
    use crate::{
        Config, Enabled, InstanceAllocationStrategy, Module, PoolingAllocationConfig, Result,
        Store,
        component::{Component, Linker},
    };
    use std::vec::Vec;

    use super::*;

    // A component with 1 core instance, 1 memory, 1 table
    const TEST_COMPONENT: &[u8] = b"
        (component
            (core module $m
                (memory 1)
                (table 1 funcref)
            )
            (core instance (instantiate (module $m)))
        )
    ";

    pub(crate) fn small_pool_config() -> PoolingAllocationConfig {
        let mut config = PoolingAllocationConfig::new();

        config.total_memories(10);
        config.max_memory_size(2 << 16);
        config.total_tables(10);
        config.table_elements(10);
        config.total_stacks(1);

        config
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn smoke_test() {
        // Start with nothing
        let engine = Engine::new(&Config::new().allocation_strategy(small_pool_config())).unwrap();
        let metrics = engine.pooling_allocator_metrics().unwrap();

        assert_eq!(metrics.core_instances(), 0);
        assert_eq!(metrics.component_instances(), 0);
        assert_eq!(metrics.memories(), 0);
        assert_eq!(metrics.tables(), 0);

        // Instantiate one of each
        let mut store = Store::new(&engine, ());
        let component = Component::new(&engine, TEST_COMPONENT).unwrap();
        let linker = Linker::new(&engine);
        let instance = linker.instantiate(&mut store, &component).unwrap();

        assert_eq!(metrics.core_instances(), 1);
        assert_eq!(metrics.component_instances(), 1);
        assert_eq!(metrics.memories(), 1);
        assert_eq!(metrics.tables(), 1);

        // Back to nothing
        let _ = (instance, store);

        assert_eq!(metrics.core_instances(), 0);
        assert_eq!(metrics.component_instances(), 0);
        assert_eq!(metrics.memories(), 0);
        assert_eq!(metrics.tables(), 0);
    }

    #[test]
    fn test_non_pooling_allocator() {
        let engine =
            Engine::new(&Config::new().allocation_strategy(InstanceAllocationStrategy::OnDemand))
                .unwrap();

        let maybe_metrics = engine.pooling_allocator_metrics();
        assert!(maybe_metrics.is_none());
    }

    #[test]
    #[cfg_attr(any(miri, not(target_os = "linux")), ignore)]
    fn unused_memories_tables_and_more() -> Result<()> {
        let mut pool = small_pool_config();
        pool.linear_memory_keep_resident(65536);
        pool.table_keep_resident(65536);
        pool.pagemap_scan(Enabled::Auto);
        let mut config = Config::new();
        config.allocation_strategy(pool);
        let engine = Engine::new(&config)?;

        let metrics = engine.pooling_allocator_metrics().unwrap();
        let host_page_size = crate::vm::host_page_size();

        assert_eq!(metrics.memories(), 0);
        assert_eq!(metrics.core_instances(), 0);
        assert_eq!(metrics.component_instances(), 0);
        assert_eq!(metrics.memories(), 0);
        assert_eq!(metrics.tables(), 0);
        assert_eq!(metrics.unused_warm_memories(), 0);
        assert_eq!(metrics.unused_memory_bytes_resident(), 0);
        assert_eq!(metrics.unused_warm_tables(), 0);
        assert_eq!(metrics.unused_table_bytes_resident(), 0);

        let m1 = Module::new(
            &engine,
            r#"
            (module (memory (export "m") 1) (table 1 funcref))
        "#,
        )?;

        let mut store = Store::new(&engine, ());
        crate::Instance::new(&mut store, &m1, &[])?;
        assert_eq!(metrics.memories(), 1);
        assert_eq!(metrics.tables(), 1);
        assert_eq!(metrics.core_instances(), 1);
        assert_eq!(metrics.component_instances(), 0);
        drop(store);

        assert_eq!(metrics.memories(), 0);
        assert_eq!(metrics.tables(), 0);
        assert_eq!(metrics.core_instances(), 0);
        assert_eq!(metrics.unused_warm_memories(), 1);
        assert_eq!(metrics.unused_warm_tables(), 1);
        if PoolingAllocationConfig::is_pagemap_scan_available() {
            assert_eq!(metrics.unused_memory_bytes_resident(), 0);
            assert_eq!(metrics.unused_table_bytes_resident(), host_page_size);
        } else {
            assert_eq!(metrics.unused_memory_bytes_resident(), 65536);
            assert_eq!(metrics.unused_table_bytes_resident(), host_page_size);
        }

        let mut store = Store::new(&engine, ());
        let i = crate::Instance::new(&mut store, &m1, &[])?;
        assert_eq!(metrics.memories(), 1);
        assert_eq!(metrics.tables(), 1);
        assert_eq!(metrics.core_instances(), 1);
        assert_eq!(metrics.component_instances(), 0);
        assert_eq!(metrics.unused_warm_memories(), 0);
        assert_eq!(metrics.unused_warm_tables(), 0);
        assert_eq!(metrics.unused_memory_bytes_resident(), 0);
        assert_eq!(metrics.unused_table_bytes_resident(), 0);
        let m = i.get_memory(&mut store, "m").unwrap();
        m.data_mut(&mut store)[0] = 1;
        m.grow(&mut store, 1)?;
        drop(store);

        assert_eq!(metrics.memories(), 0);
        assert_eq!(metrics.tables(), 0);
        assert_eq!(metrics.core_instances(), 0);
        assert_eq!(metrics.unused_warm_memories(), 1);
        assert_eq!(metrics.unused_warm_tables(), 1);
        if PoolingAllocationConfig::is_pagemap_scan_available() {
            assert_eq!(metrics.unused_memory_bytes_resident(), host_page_size);
            assert_eq!(metrics.unused_table_bytes_resident(), host_page_size);
        } else {
            assert_eq!(metrics.unused_memory_bytes_resident(), 65536);
            assert_eq!(metrics.unused_table_bytes_resident(), host_page_size);
        }

        let stores = (0..10)
            .map(|_| {
                let mut store = Store::new(&engine, ());
                crate::Instance::new(&mut store, &m1, &[]).unwrap();
                store
            })
            .collect::<Vec<_>>();

        assert_eq!(metrics.memories(), 10);
        assert_eq!(metrics.tables(), 10);
        assert_eq!(metrics.core_instances(), 10);
        assert_eq!(metrics.unused_warm_memories(), 0);
        assert_eq!(metrics.unused_warm_tables(), 0);
        assert_eq!(metrics.unused_memory_bytes_resident(), 0);
        assert_eq!(metrics.unused_table_bytes_resident(), 0);

        drop(stores);

        assert_eq!(metrics.memories(), 00);
        assert_eq!(metrics.tables(), 00);
        assert_eq!(metrics.core_instances(), 00);
        assert_eq!(metrics.unused_warm_memories(), 10);
        assert_eq!(metrics.unused_warm_tables(), 10);
        if PoolingAllocationConfig::is_pagemap_scan_available() {
            assert_eq!(metrics.unused_memory_bytes_resident(), host_page_size);
            assert_eq!(metrics.unused_table_bytes_resident(), 10 * host_page_size);
        } else {
            assert_eq!(metrics.unused_memory_bytes_resident(), 10 * 65536);
            assert_eq!(metrics.unused_table_bytes_resident(), 10 * host_page_size);
        }

        Ok(())
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn gc_heaps() -> Result<()> {
        let pool = small_pool_config();
        let mut config = Config::new();
        config.allocation_strategy(pool);
        let engine = Engine::new(&config)?;

        let metrics = engine.pooling_allocator_metrics().unwrap();

        assert_eq!(metrics.gc_heaps(), 0);
        let mut store = Store::new(&engine, ());
        crate::ExternRef::new(&mut store, ())?;
        assert_eq!(metrics.gc_heaps(), 1);
        drop(store);
        assert_eq!(metrics.gc_heaps(), 0);

        Ok(())
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn stacks() -> Result<()> {
        let pool = small_pool_config();
        let mut config = Config::new();
        config.allocation_strategy(pool);
        let engine = Engine::new(&config)?;

        let metrics = engine.pooling_allocator_metrics().unwrap();

        assert_eq!(metrics.stacks(), 0);
        assert_eq!(metrics.unused_warm_stacks(), 0);
        let mut store = Store::new(&engine, ());

        crate::Func::wrap(&mut store, || {})
            .call_async(&mut store, &[], &mut [])
            .await?;
        assert_eq!(metrics.stacks(), 1);
        drop(store);
        assert_eq!(metrics.stacks(), 0);
        assert_eq!(metrics.unused_stack_bytes_resident(), None);
        if StackPool::enabled() {
            assert_eq!(metrics.unused_warm_stacks(), 1);
        } else {
            assert_eq!(metrics.unused_warm_stacks(), 0);
        }

        Ok(())
    }
}
