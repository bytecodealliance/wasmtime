use crate::{Store, StoreInner};
use std::rc::Weak;

/// Used by hosts to limit resource consumption of instances.
///
/// A [`Store`] can be created with a resource limiter so that hosts can take into account
/// non-WebAssembly resource usage to determine if a linear memory or table should grow.
pub trait ResourceLimiter {
    /// Notifies the resource limiter that an instance's linear memory has been requested to grow.
    ///
    /// * `current` is the current size of the linear memory in WebAssembly page units.
    /// * `desired` is the desired size of the linear memory in WebAssembly page units.
    /// * `maximum` is either the linear memory's maximum or a maximum from an instance allocator,
    ///   also in WebAssembly page units. A value of `None` indicates that the linear memory is
    ///   unbounded.
    ///
    /// This function should return `true` to indicate that the growing operation is permitted or
    /// `false` if not permitted. Returning `true` when a maximum has been exceeded will have no
    /// effect as the linear memory will not grow.
    fn memory_growing(
        &self,
        store: &Store,
        current: u32,
        desired: u32,
        maximum: Option<u32>,
    ) -> bool;

    /// Notifies the resource limiter that an instance's table has been requested to grow.
    ///
    /// * `current` is the current number of elements in the table.
    /// * `desired` is the desired number of elements in the table.
    /// * `maximum` is either the table's maximum or a maximum from an instance allocator,
    ///   A value of `None` indicates that the table is unbounded.
    ///
    /// This function should return `true` to indicate that the growing operation is permitted or
    /// `false` if not permitted. Returning `true` when a maximum has been exceeded will have no
    /// effect as the table will not grow.
    fn table_growing(
        &self,
        store: &Store,
        current: u32,
        desired: u32,
        maximum: Option<u32>,
    ) -> bool;
}

pub(crate) struct ResourceLimiterProxy {
    store: Weak<StoreInner>,
    limiter: Box<dyn ResourceLimiter>,
}

impl ResourceLimiterProxy {
    pub(crate) fn new(store: &Store, limiter: impl ResourceLimiter + 'static) -> Self {
        Self {
            store: store.weak(),
            limiter: Box::new(limiter),
        }
    }
}

impl wasmtime_runtime::ResourceLimiter for ResourceLimiterProxy {
    fn memory_growing(&self, current: u32, desired: u32, maximum: Option<u32>) -> bool {
        self.limiter.memory_growing(
            &Store::upgrade(&self.store).unwrap(),
            current,
            desired,
            maximum,
        )
    }

    fn table_growing(&self, current: u32, desired: u32, maximum: Option<u32>) -> bool {
        self.limiter.table_growing(
            &Store::upgrade(&self.store).unwrap(),
            current,
            desired,
            maximum,
        )
    }
}

/// A resource limiter that statically limits how much memories and tables can grow.
pub struct StaticResourceLimiter {
    memory_limit: Option<u32>,
    table_limit: Option<u32>,
}

impl StaticResourceLimiter {
    /// Creates a new [`StaticResourceLimiter`].
    ///
    /// The `memory_limit` parameter is the number of WebAssembly pages a linear memory can grow to.
    /// If `None`, the limiter will not limit linear memory growth.
    ///
    /// The `table_limit` parameter is the number of elements a table can grow to.
    /// If `None`, the limiter will not limit table growth.
    pub fn new(memory_limit: Option<u32>, table_limit: Option<u32>) -> Self {
        Self {
            memory_limit,
            table_limit,
        }
    }
}

impl ResourceLimiter for StaticResourceLimiter {
    fn memory_growing(
        &self,
        _store: &Store,
        _current: u32,
        desired: u32,
        _maximum: Option<u32>,
    ) -> bool {
        match self.memory_limit {
            Some(limit) if desired > limit => false,
            _ => true,
        }
    }

    fn table_growing(
        &self,
        _store: &Store,
        _current: u32,
        desired: u32,
        _maximum: Option<u32>,
    ) -> bool {
        match self.table_limit {
            Some(limit) if desired > limit => false,
            _ => true,
        }
    }
}
