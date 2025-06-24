//! Utility module to create trampolines in/out WebAssembly module.

mod func;
mod global;
mod memory;
mod table;

pub use self::func::*;
pub use self::global::*;
pub(crate) use memory::MemoryCreatorProxy;

use self::memory::create_memory;
use self::table::create_table;
use crate::prelude::*;
use crate::runtime::vm::SharedMemory;
use crate::store::StoreOpaque;
use crate::{MemoryType, TableType};
use wasmtime_environ::{MemoryIndex, TableIndex};

pub fn generate_memory_export(
    store: &mut StoreOpaque,
    m: &MemoryType,
    preallocation: Option<&SharedMemory>,
) -> Result<crate::runtime::vm::ExportMemory> {
    let instance = create_memory(store, m, preallocation)?;
    Ok(store
        .instance_mut(instance)
        .get_exported_memory(MemoryIndex::from_u32(0)))
}

pub fn generate_table_export(
    store: &mut StoreOpaque,
    t: &TableType,
) -> Result<crate::runtime::vm::ExportTable> {
    let instance = create_table(store, t)?;
    Ok(store
        .instance_mut(instance)
        .get_exported_table(TableIndex::from_u32(0)))
}
