/// Value returned by [`ResourceLimiter::instances`] default method
pub const DEFAULT_INSTANCE_LIMIT: usize = 10000;
/// Value returned by [`ResourceLimiter::tables`] default method
pub const DEFAULT_TABLE_LIMIT: usize = 10000;
/// Value returned by [`ResourceLimiter::memories`] default method
pub const DEFAULT_MEMORY_LIMIT: usize = 10000;

/// Used by hosts to limit resource consumption of instances.
///
/// This trait is used in conjunction with the
/// [`Store::limiter`](crate::Store::limiter) to synchronously limit the
/// allocation of resources within a store. As a store-level limit this means
/// that all creation of instances, memories, and tables are limited within the
/// store. Resources limited via this trait are primarily related to memory and
/// limiting CPU resources needs to be done with something such as
/// [`Config::consume_fuel`](crate::Config::consume_fuel) or
/// [`Config::epoch_interruption`](crate::Config::epoch_interruption).
///
/// Note that this trait does not limit 100% of memory allocated via a
/// [`Store`](crate::Store). Wasmtime will still allocate memory to track data
/// structures and additionally embedder-specific memory allocations are not
/// tracked via this trait. This trait only limits resources allocated by a
/// WebAssembly instance itself.
///
/// This trait is intended for synchronously limiting the resources of a module.
/// If your use case requires blocking to answer whether a request is permitted
/// or not and you're otherwise working in an asynchronous context the
/// [`ResourceLimiterAsync`] trait is also provided to avoid blocking an OS
/// thread while a limit is determined.
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
    /// The `current` and `desired` amounts are guaranteed to always be
    /// multiples of the WebAssembly page size, 64KiB.
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
    ///
    /// Returning `false` from this method will cause the `memory.grow`
    /// instruction in a module to return -1 (failure), or in the case of an
    /// embedder API calling [`Memory::new`](crate::Memory::new) or
    /// [`Memory::grow`](crate::Memory::grow) an error will be returned from
    /// those methods.
    fn memory_growing(&mut self, current: usize, desired: usize, maximum: Option<usize>) -> bool;

    /// Notifies the resource limiter that growing a linear memory, permitted by
    /// the `memory_growing` method, has failed.
    ///
    /// Reasons for failure include: the growth exceeds the `maximum` passed to
    /// `memory_growing`, or the operating system failed to allocate additional
    /// memory. In that case, `error` might be downcastable to a `std::io::Error`.
    fn memory_grow_failed(&mut self, _error: &anyhow::Error) {}

    /// Notifies the resource limiter that an instance's table has been
    /// requested to grow.
    ///
    /// * `current` is the current number of elements in the table.
    /// * `desired` is the desired number of elements in the table.
    /// * `maximum` is either the table's maximum or a maximum from an instance
    ///   allocator.  A value of `None` indicates that the table is unbounded.
    ///
    /// This function should return `true` to indicate that the growing
    /// operation is permitted or `false` if not permitted. Returning `true`
    /// when a maximum has been exceeded will have no effect as the table will
    /// not grow.
    ///
    /// Currently in Wasmtime each table element requires a pointer's worth of
    /// space (e.g. `mem::size_of::<usize>()`).
    ///
    /// Like `memory_growing` returning `false` from this function will cause
    /// `table.grow` to return -1 or embedder APIs will return an error.
    fn table_growing(&mut self, current: u32, desired: u32, maximum: Option<u32>) -> bool;

    /// Notifies the resource limiter that growing a linear memory, permitted by
    /// the `table_growing` method, has failed.
    ///
    /// Reasons for failure include: the growth exceeds the `maximum` passed to
    /// `table_growing`. This could expand in the future.
    fn table_grow_failed(&mut self, _error: &anyhow::Error) {}

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
    /// Creation of tables will fail if this limit is exceeded.
    ///
    /// This value defaults to 10,000.
    fn tables(&self) -> usize {
        DEFAULT_TABLE_LIMIT
    }

    /// The maximum number of linear memories that can be created for a `Store`
    ///
    /// Creation of memories will fail with an error if this limit is exceeded.
    ///
    /// This value defaults to 10,000.
    fn memories(&self) -> usize {
        DEFAULT_MEMORY_LIMIT
    }
}

/// Used by hosts to limit resource consumption of instances, blocking
/// asynchronously if necessary.
///
/// This trait is identical to [`ResourceLimiter`], except that the
/// `memory_growing` and `table_growing` functions are `async`. Must be used
/// with an async [`Store`](`crate::Store`) configured via
/// [`Config::async_support`](crate::Config::async_support).
///
/// This trait is used with
/// [`Store::limiter_async`](`crate::Store::limiter_async`)`: see those docs
/// for restrictions on using other Wasmtime interfaces with an async resource
/// limiter. Additionally see [`ResourceLimiter`] for more information about
/// limiting resources from WebAssembly.
///
/// The `async` here enables embedders that are already using asynchronous
/// execution of WebAssembly to block the WebAssembly, but no the OS thread, to
/// answer the question whether growing a memory or table is allowed.
#[cfg(feature = "async")]
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
    fn memory_grow_failed(&mut self, _error: &anyhow::Error) {}

    /// Asynchronous version of [`ResourceLimiter::table_growing`]
    async fn table_growing(&mut self, current: u32, desired: u32, maximum: Option<u32>) -> bool;

    /// Identical to [`ResourceLimiter::table_grow_failed`]
    fn table_grow_failed(&mut self, _error: &anyhow::Error) {}

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
    ///
    /// See the documentation on each builder method for the default for each
    /// value.
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
///
/// This type is created with a [`StoreLimitsBuilder`] and is typically used in
/// conjunction with [`Store::limiter`](crate::Store::limiter).
///
/// This is a convenience type included to avoid needing to implement the
/// [`ResourceLimiter`] trait if your use case fits in the static configuration
/// that this [`StoreLimits`] provides.
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
