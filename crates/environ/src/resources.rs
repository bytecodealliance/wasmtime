/// A summary of the amount of resources required to instantiate a particular
/// [`Component`][crate::component::Component].
///
/// Example uses of this information:
///
/// * Determining whether your pooling allocator configuration supports
///   instantiating this module.
///
/// * Deciding how many of which `Module` you want to instantiate within a
///   fixed amount of resources, e.g. determining whether to create 5
///   instances of module `X` or 10 instances of module `Y`.
pub struct ComponentResourcesRequired {
    core_instances_aggregate_size: usize,
    num_core_instances: usize,
    module_resources: ResourcesRequired,
}

/// A summary of the amount of resources required to instantiate a particular
/// [`Module`][crate::Module].
///
/// Example uses of this information:
///
/// * Determining whether your pooling allocator configuration supports
///   instantiating this module.
///
/// * Deciding how many of which `Module` you want to instantiate within a
///   fixed amount of resources, e.g. determining whether to create 5
///   instances of module `X` or 10 instances of module `Y`.
pub struct ResourcesRequired {
    /// The number of memories that are required.
    pub num_memories: u32,
    /// The maximum initial size required by any memory, in units of Wasm pages.
    pub max_initial_memory_size: Option<u64>,
    /// The number of tables that are required.
    pub num_tables: u32,
    /// The maximum initial size required by any table.
    pub max_initial_table_size: Option<u64>,
}

impl ComponentResourcesRequired {
    /// Returns a requirement of zero resources.
    pub fn zero() -> Self {
        Self {
            core_instances_aggregate_size: 0,
            num_core_instances: 0,
            module_resources: ResourcesRequired::zero(),
        }
    }
}

impl ResourcesRequired {
    /// Returns a requirement of zero resources.
    pub fn zero() -> Self {
        Self {
            num_memories: 0,
            max_initial_memory_size: None,
            num_tables: 0,
            max_initial_table_size: None,
        }
    }

    /// Adds the provided resource requirements to this set of requirements.
    #[cfg(feature = "component-model")]
    pub fn add(&mut self, other: &ResourcesRequired) {
        self.num_memories += other.num_memories;
        self.max_initial_memory_size =
            core::cmp::max(self.max_initial_memory_size, other.max_initial_memory_size);
        self.num_tables += other.num_tables;
        self.max_initial_table_size =
            core::cmp::max(self.max_initial_table_size, other.max_initial_table_size);
    }
}

impl crate::Module {
    /// Returns a summary of the resources required to instantiate this
    /// [`Module`].
    pub fn resources_required(&self) -> ResourcesRequired {
        let num_memories = u32::try_from(self.num_defined_memories()).unwrap();
        let max_initial_memory_size = self
            .memories
            .values()
            .skip(self.num_imported_memories)
            .map(|memory| memory.limits.min)
            .max();
        let num_tables = u32::try_from(self.num_defined_tables()).unwrap();
        let max_initial_table_size = self
            .tables
            .values()
            .skip(self.num_imported_tables)
            .map(|table| table.limits.min)
            .max();
        ResourcesRequired {
            num_memories,
            max_initial_memory_size,
            num_tables,
            max_initial_table_size,
        }
    }
}

#[cfg(feature = "component-model")]
impl crate::component::Component {
    /// Returns a summary of the resources required to instantiate this
    /// [`Component`][crate::component::Component].
    pub fn resources_required<'a>(
        &self,
        get_module: &'a dyn Fn(crate::StaticModuleIndex) -> &'a crate::Module,
        get_size: &'a dyn Fn(&crate::Module) -> usize,
    ) -> ComponentResourcesRequired {
        use crate::component::{GlobalInitializer::*, InstantiateModule};
        let mut resources = ComponentResourcesRequired::zero();
        for init in &self.initializers {
            match init {
                InstantiateModule(inst, _) => match inst {
                    InstantiateModule::Static(index, _) => {
                        let module = get_module(*index);
                        resources.num_core_instances += 1;
                        resources.core_instances_aggregate_size += get_size(module);
                        resources.module_resources.add(&module.resources_required());
                    }
                    InstantiateModule::Import(_, _) => {
                        // We can't statically determine the resources required
                        // to instantiate this component.
                        resources.num_core_instances += 1;
                    }
                },
                LowerImport { .. }
                | ExtractMemory(_)
                | ExtractTable(_)
                | ExtractRealloc(_)
                | ExtractCallback(_)
                | ExtractPostReturn(_)
                | Resource(_) => {}
            }
        }
        resources
    }
}
