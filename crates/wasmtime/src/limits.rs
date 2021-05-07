pub(crate) const DEFAULT_INSTANCE_LIMIT: usize = 10000;
pub(crate) const DEFAULT_TABLE_LIMIT: usize = 10000;
pub(crate) const DEFAULT_MEMORY_LIMIT: usize = 10000;

/// Used by hosts to limit resource consumption of instances at runtime.
///
/// [`Store::new_with_limits`](crate::Store::new_with_limits) can be used
/// with a resource limiter to take into account non-WebAssembly resource
/// usage to determine if a linear memory or table should be grown.
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
    /// `false` if not permitted.
    ///
    /// Note that this function will be called even when the desired count exceeds the given maximum.
    ///
    /// Returning `true` when a maximum has been exceeded will have no effect as the linear memory
    /// will not be grown.
    fn memory_growing(&self, current: u32, desired: u32, maximum: Option<u32>) -> bool;

    /// Notifies the resource limiter that an instance's table has been requested to grow.
    ///
    /// * `current` is the current number of elements in the table.
    /// * `desired` is the desired number of elements in the table.
    /// * `maximum` is either the table's maximum or a maximum from an instance allocator,
    ///   A value of `None` indicates that the table is unbounded.
    ///
    /// This function should return `true` to indicate that the growing operation is permitted or
    /// `false` if not permitted.
    ///
    /// Note that this function will be called even when the desired count exceeds the given maximum.
    ///
    /// Returning `true` when a maximum has been exceeded will have no effect as the table will
    /// not be grown.
    fn table_growing(&self, current: u32, desired: u32, maximum: Option<u32>) -> bool;

    /// The maximum number of instances that can be created for a [`Store`](crate::Store).
    ///
    /// Module instantiation will fail if this limit is exceeded.
    ///
    /// This value defaults to 10,000.
    fn instances(&self) -> usize {
        DEFAULT_INSTANCE_LIMIT
    }

    /// The maximum number of tables that can be created for a [`Store`](crate::Store).
    ///
    /// Module instantiation will fail if this limit is exceeded.
    ///
    /// This value defaults to 10,000.
    fn tables(&self) -> usize {
        DEFAULT_TABLE_LIMIT
    }

    /// The maximum number of linear memories that can be created for a [`Store`](crate::Store).
    ///
    /// Instantiation will fail with an error if this limit is exceeded.
    ///
    /// This value defaults to 10,000.
    fn memories(&self) -> usize {
        DEFAULT_MEMORY_LIMIT
    }
}

pub(crate) struct ResourceLimiterProxy<T>(pub T);

impl<T: ResourceLimiter> wasmtime_runtime::ResourceLimiter for ResourceLimiterProxy<T> {
    fn memory_growing(&self, current: u32, desired: u32, maximum: Option<u32>) -> bool {
        self.0.memory_growing(current, desired, maximum)
    }

    fn table_growing(&self, current: u32, desired: u32, maximum: Option<u32>) -> bool {
        self.0.table_growing(current, desired, maximum)
    }

    fn instances(&self) -> usize {
        self.0.instances()
    }

    fn tables(&self) -> usize {
        self.0.tables()
    }

    fn memories(&self) -> usize {
        self.0.memories()
    }
}

/// Used to build [`StoreLimits`].
pub struct StoreLimitsBuilder(StoreLimits);

impl StoreLimitsBuilder {
    /// Creates a new [`StoreLimitsBuilder`].
    pub fn new() -> Self {
        Self(StoreLimits::default())
    }

    /// The maximum number of WebAssembly pages a linear memory can grow to.
    ///
    /// Growing a linear memory beyond this limit will fail.
    ///
    /// By default, linear memory pages will not be limited.
    pub fn memory_pages(mut self, limit: u32) -> Self {
        self.0.memory_pages = Some(limit);
        self
    }

    /// The maximum number of elements in a table.
    ///
    /// Growing a table beyond this limit will fail.
    ///
    /// By default, table elements will not be limited.
    pub fn table_elements(mut self, limit: u32) -> Self {
        self.0.table_elements = Some(limit);
        self
    }

    /// The maximum number of instances that can be created for a [`Store`](crate::Store).
    ///
    /// Module instantiation will fail if this limit is exceeded.
    ///
    /// This value defaults to 10,000.
    pub fn instances(mut self, limit: usize) -> Self {
        self.0.instances = limit;
        self
    }

    /// The maximum number of tables that can be created for a [`Store`](crate::Store).
    ///
    /// Module instantiation will fail if this limit is exceeded.
    ///
    /// This value defaults to 10,000.
    pub fn tables(mut self, tables: usize) -> Self {
        self.0.tables = tables;
        self
    }

    /// The maximum number of linear memories that can be created for a [`Store`](crate::Store).
    ///
    /// Instantiation will fail with an error if this limit is exceeded.
    ///
    /// This value defaults to 10,000.
    pub fn memories(mut self, memories: usize) -> Self {
        self.0.memories = memories;
        self
    }

    /// Consumes this builder and returns the [`StoreLimits`].
    pub fn build(self) -> StoreLimits {
        self.0
    }
}

/// Provides limits for a [`Store`](crate::Store).
pub struct StoreLimits {
    memory_pages: Option<u32>,
    table_elements: Option<u32>,
    instances: usize,
    tables: usize,
    memories: usize,
}

impl Default for StoreLimits {
    fn default() -> Self {
        Self {
            memory_pages: None,
            table_elements: None,
            instances: DEFAULT_INSTANCE_LIMIT,
            tables: DEFAULT_TABLE_LIMIT,
            memories: DEFAULT_MEMORY_LIMIT,
        }
    }
}

impl ResourceLimiter for StoreLimits {
    fn memory_growing(&self, _current: u32, desired: u32, _maximum: Option<u32>) -> bool {
        match self.memory_pages {
            Some(limit) if desired > limit => false,
            _ => true,
        }
    }

    fn table_growing(&self, _current: u32, desired: u32, _maximum: Option<u32>) -> bool {
        match self.table_elements {
            Some(limit) if desired > limit => false,
            _ => true,
        }
    }

    fn instances(&self) -> usize {
        self.instances
    }

    fn tables(&self) -> usize {
        self.tables
    }

    fn memories(&self) -> usize {
        self.memories
    }
}
