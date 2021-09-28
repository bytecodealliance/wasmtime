/// Value returned by [`ResourceLimiter::instances`] default method
pub const DEFAULT_INSTANCE_LIMIT: usize = 10000;
/// Value returned by [`ResourceLimiter::tables`] default method
pub const DEFAULT_TABLE_LIMIT: usize = 10000;
/// Value returned by [`ResourceLimiter::memories`] default method
pub const DEFAULT_MEMORY_LIMIT: usize = 10000;

/// Used by hosts to limit resource consumption of instances.
///
/// An instance can be created with a resource limiter so that hosts can take into account
/// non-WebAssembly resource usage to determine if a linear memory or table should grow.
pub trait ResourceLimiter {
    /// Notifies the resource limiter that an instance's linear memory has been
    /// requested to grow.
    ///
    /// * `current` is the current size of the linear memory in bytes.
    /// * `desired` is the desired size of the linear memory in bytes.
    /// * `maximum` is either the linear memory's maximum or a maximum from an
    ///   instance allocator, also in bytes. A value of `None`
    ///   indicates that the linear memory is unbounded.
    ///
    /// This function should return `true` to indicate that the growing
    /// operation is permitted or `false` if not permitted. Returning `true`
    /// when a maximum has been exceeded will have no effect as the linear
    /// memory will not grow.
    ///
    /// This function is not guaranteed to be invoked for all requests to
    /// `memory.grow`. Requests where the allocation requested size doesn't fit
    /// in `usize` or exceeds the memory's listed maximum size may not invoke
    /// this method.
    fn memory_growing(&mut self, current: usize, desired: usize, maximum: Option<usize>) -> bool;

    /// Notifies the resource limiter that growing a linear memory, permitted by
    /// the `memory_growing` method, has failed.
    ///
    /// Reasons for failure include: the growth exceeds the `maximum` passed to
    /// `memory_growing`, or the operating system failed to allocate additional
    /// memory. In that case, `error` might be downcastable to a `std::io::Error`.
    fn memory_grow_failed(&mut self, _error: &anyhow::Error) {}

    /// Notifies the resource limiter that an instance's table has been requested to grow.
    ///
    /// * `current` is the current number of elements in the table.
    /// * `desired` is the desired number of elements in the table.
    /// * `maximum` is either the table's maximum or a maximum from an instance allocator.
    ///   A value of `None` indicates that the table is unbounded.
    ///
    /// This function should return `true` to indicate that the growing operation is permitted or
    /// `false` if not permitted. Returning `true` when a maximum has been exceeded will have no
    /// effect as the table will not grow.
    fn table_growing(&mut self, current: u32, desired: u32, maximum: Option<u32>) -> bool;

    /// The maximum number of instances that can be created for a `Store`.
    ///
    /// Module instantiation will fail if this limit is exceeded.
    ///
    /// This value defaults to 10,000.
    fn instances(&self) -> usize {
        DEFAULT_INSTANCE_LIMIT
    }

    /// The maximum number of tables that can be created for a `Store`.
    ///
    /// Module instantiation will fail if this limit is exceeded.
    ///
    /// This value defaults to 10,000.
    fn tables(&self) -> usize {
        DEFAULT_TABLE_LIMIT
    }

    /// The maximum number of linear memories that can be created for a `Store`
    ///
    /// Instantiation will fail with an error if this limit is exceeded.
    ///
    /// This value defaults to 10,000.
    fn memories(&self) -> usize {
        DEFAULT_MEMORY_LIMIT
    }
}

#[cfg(feature = "async")]
/// Used by hosts to limit resource consumption of instances.
/// Identical to [`ResourceLimiter`], except that the `memory_growing` and `table_growing`
/// functions are async. Must be used with an async [`Store`].
#[async_trait::async_trait]
pub trait ResourceLimiterAsync {
    /// Async version of [`ResourceLimiter::memory_growing`]
    async fn memory_growing(
        &mut self,
        current: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> bool;

    /// Identical to [`ResourceLimiter::memory_grow_failed`]
    fn memory_grow_failed(&mut self, error: &anyhow::Error);

    /// Asynchronous version of [`ResourceLimiter::table_growing`]
    async fn table_growing(&mut self, current: u32, desired: u32, maximum: Option<u32>) -> bool;

    /// Identical to [`ResourceLimiter::instances`]`
    fn instances(&self) -> usize {
        DEFAULT_INSTANCE_LIMIT
    }

    /// Identical to [`ResourceLimiter::tables`]`
    fn tables(&self) -> usize {
        DEFAULT_TABLE_LIMIT
    }

    /// Identical to [`ResourceLimiter::memories`]`
    fn memories(&self) -> usize {
        DEFAULT_MEMORY_LIMIT
    }
}

/// Used to build [`StoreLimits`].
pub struct StoreLimitsBuilder(StoreLimits);

impl StoreLimitsBuilder {
    /// Creates a new [`StoreLimitsBuilder`].
    pub fn new() -> Self {
        Self(StoreLimits::default())
    }

    /// The maximum number of bytes a linear memory can grow to.
    ///
    /// Growing a linear memory beyond this limit will fail.
    ///
    /// By default, linear memory will not be limited.
    pub fn memory_size(mut self, limit: usize) -> Self {
        self.0.memory_size = Some(limit);
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
    memory_size: Option<usize>,
    table_elements: Option<u32>,
    instances: usize,
    tables: usize,
    memories: usize,
}

impl Default for StoreLimits {
    fn default() -> Self {
        Self {
            memory_size: None,
            table_elements: None,
            instances: DEFAULT_INSTANCE_LIMIT,
            tables: DEFAULT_TABLE_LIMIT,
            memories: DEFAULT_MEMORY_LIMIT,
        }
    }
}

#[cfg_attr(feature = "async", async_trait::async_trait)]
impl ResourceLimiter for StoreLimits {
    fn memory_growing(&mut self, _current: usize, desired: usize, _maximum: Option<usize>) -> bool {
        match self.memory_size {
            Some(limit) if desired > limit => false,
            _ => true,
        }
    }

    fn table_growing(&mut self, _current: u32, desired: u32, _maximum: Option<u32>) -> bool {
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
