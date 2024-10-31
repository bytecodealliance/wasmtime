use crate::store::StoreOpaque;
use crate::{AsContext, Engine, ExternType, Func, Memory, SharedMemory};

mod global;
mod table;

pub use global::Global;
pub use table::Table;

// Externals

/// An external item to a WebAssembly module, or a list of what can possibly be
/// exported from a wasm module.
///
/// This is both returned from [`Instance::exports`](crate::Instance::exports)
/// as well as required by [`Instance::new`](crate::Instance::new). In other
/// words, this is the type of extracted values from an instantiated module, and
/// it's also used to provide imported values when instantiating a module.
#[derive(Clone, Debug)]
pub enum Extern {
    /// A WebAssembly `func` which can be called.
    Func(Func),
    /// A WebAssembly `global` which acts like a `Cell<T>` of sorts, supporting
    /// `get` and `set` operations.
    Global(Global),
    /// A WebAssembly `table` which is an array of `Val` reference types.
    Table(Table),
    /// A WebAssembly linear memory.
    Memory(Memory),
    /// A WebAssembly shared memory; these are handled separately from
    /// [`Memory`].
    SharedMemory(SharedMemory),
}

impl Extern {
    /// Returns the underlying `Func`, if this external is a function.
    ///
    /// Returns `None` if this is not a function.
    pub fn into_func(self) -> Option<Func> {
        match self {
            Extern::Func(func) => Some(func),
            _ => None,
        }
    }

    /// Returns the underlying `Global`, if this external is a global.
    ///
    /// Returns `None` if this is not a global.
    pub fn into_global(self) -> Option<Global> {
        match self {
            Extern::Global(global) => Some(global),
            _ => None,
        }
    }

    /// Returns the underlying `Table`, if this external is a table.
    ///
    /// Returns `None` if this is not a table.
    pub fn into_table(self) -> Option<Table> {
        match self {
            Extern::Table(table) => Some(table),
            _ => None,
        }
    }

    /// Returns the underlying `Memory`, if this external is a memory.
    ///
    /// Returns `None` if this is not a memory.
    pub fn into_memory(self) -> Option<Memory> {
        match self {
            Extern::Memory(memory) => Some(memory),
            _ => None,
        }
    }

    /// Returns the underlying `SharedMemory`, if this external is a shared
    /// memory.
    ///
    /// Returns `None` if this is not a shared memory.
    pub fn into_shared_memory(self) -> Option<SharedMemory> {
        match self {
            Extern::SharedMemory(memory) => Some(memory),
            _ => None,
        }
    }

    /// Returns the type associated with this `Extern`.
    ///
    /// The `store` argument provided must own this `Extern` and is used to look
    /// up type information.
    ///
    /// # Panics
    ///
    /// Panics if this item does not belong to the `store` provided.
    pub fn ty(&self, store: impl AsContext) -> ExternType {
        let store = store.as_context();
        match self {
            Extern::Func(ft) => ExternType::Func(ft.ty(store)),
            Extern::Memory(ft) => ExternType::Memory(ft.ty(store)),
            Extern::SharedMemory(ft) => ExternType::Memory(ft.ty()),
            Extern::Table(tt) => ExternType::Table(tt.ty(store)),
            Extern::Global(gt) => ExternType::Global(gt.ty(store)),
        }
    }

    pub(crate) unsafe fn from_wasmtime_export(
        wasmtime_export: crate::runtime::vm::Export,
        store: &mut StoreOpaque,
    ) -> Extern {
        match wasmtime_export {
            crate::runtime::vm::Export::Function(f) => {
                Extern::Func(Func::from_wasmtime_function(f, store))
            }
            crate::runtime::vm::Export::Memory(m) => {
                if m.memory.shared {
                    Extern::SharedMemory(SharedMemory::from_wasmtime_memory(m, store))
                } else {
                    Extern::Memory(Memory::from_wasmtime_memory(m, store))
                }
            }
            crate::runtime::vm::Export::Global(g) => {
                Extern::Global(Global::from_wasmtime_global(g, store))
            }
            crate::runtime::vm::Export::Table(t) => {
                Extern::Table(Table::from_wasmtime_table(t, store))
            }
        }
    }

    pub(crate) fn comes_from_same_store(&self, store: &StoreOpaque) -> bool {
        match self {
            Extern::Func(f) => f.comes_from_same_store(store),
            Extern::Global(g) => store.store_data().contains(g.0),
            Extern::Memory(m) => m.comes_from_same_store(store),
            Extern::SharedMemory(m) => Engine::same(m.engine(), store.engine()),
            Extern::Table(t) => store.store_data().contains(t.0),
        }
    }
}

impl From<Func> for Extern {
    fn from(r: Func) -> Self {
        Extern::Func(r)
    }
}

impl From<Global> for Extern {
    fn from(r: Global) -> Self {
        Extern::Global(r)
    }
}

impl From<Memory> for Extern {
    fn from(r: Memory) -> Self {
        Extern::Memory(r)
    }
}

impl From<SharedMemory> for Extern {
    fn from(r: SharedMemory) -> Self {
        Extern::SharedMemory(r)
    }
}

impl From<Table> for Extern {
    fn from(r: Table) -> Self {
        Extern::Table(r)
    }
}

// Exports

/// An exported WebAssembly value.
///
/// This type is primarily accessed from the
/// [`Instance::exports`](crate::Instance::exports) accessor and describes what
/// names and items are exported from a wasm instance.
#[derive(Clone)]
pub struct Export<'instance> {
    /// The name of the export.
    name: &'instance str,

    /// The definition of the export.
    definition: Extern,
}

impl<'instance> Export<'instance> {
    /// Creates a new export which is exported with the given `name` and has the
    /// given `definition`.
    pub(crate) fn new(name: &'instance str, definition: Extern) -> Export<'instance> {
        Export { name, definition }
    }

    /// Returns the name by which this export is known.
    pub fn name(&self) -> &'instance str {
        self.name
    }

    /// Return the `ExternType` of this export.
    ///
    /// # Panics
    ///
    /// Panics if `store` does not own this `Extern`.
    pub fn ty(&self, store: impl AsContext) -> ExternType {
        self.definition.ty(store)
    }

    /// Consume this `Export` and return the contained `Extern`.
    pub fn into_extern(self) -> Extern {
        self.definition
    }

    /// Consume this `Export` and return the contained `Func`, if it's a function,
    /// or `None` otherwise.
    pub fn into_func(self) -> Option<Func> {
        self.definition.into_func()
    }

    /// Consume this `Export` and return the contained `Table`, if it's a table,
    /// or `None` otherwise.
    pub fn into_table(self) -> Option<Table> {
        self.definition.into_table()
    }

    /// Consume this `Export` and return the contained `Memory`, if it's a memory,
    /// or `None` otherwise.
    pub fn into_memory(self) -> Option<Memory> {
        self.definition.into_memory()
    }

    /// Consume this `Export` and return the contained `Global`, if it's a global,
    /// or `None` otherwise.
    pub fn into_global(self) -> Option<Global> {
        self.definition.into_global()
    }
}
