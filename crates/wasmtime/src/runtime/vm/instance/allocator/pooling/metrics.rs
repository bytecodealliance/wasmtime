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

    fn allocator(&self) -> &PoolingInstanceAllocator {
        self.engine
            .allocator()
            .as_pooling()
            .expect("engine should have pooling allocator")
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        Config, InstanceAllocationStrategy, Store,
        component::{Component, Linker},
    };

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

    #[test]
    #[cfg_attr(miri, ignore)]
    fn smoke_test() {
        // Start with nothing
        let engine =
            Engine::new(&Config::new().allocation_strategy(InstanceAllocationStrategy::pooling()))
                .unwrap();
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
}
