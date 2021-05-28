pub use wasmtime_runtime::ResourceLimiter;

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
            instances: wasmtime_runtime::DEFAULT_INSTANCE_LIMIT,
            tables: wasmtime_runtime::DEFAULT_TABLE_LIMIT,
            memories: wasmtime_runtime::DEFAULT_MEMORY_LIMIT,
        }
    }
}

impl ResourceLimiter for StoreLimits {
    fn memory_growing(&mut self, _current: u32, desired: u32, _maximum: Option<u32>) -> bool {
        match self.memory_pages {
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
