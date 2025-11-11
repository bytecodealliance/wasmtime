//! Utility module to create trampolines in/out WebAssembly module.

mod func;
mod global;
mod memory;
mod table;
mod tag;

pub use self::func::*;
pub use self::global::*;
pub(crate) use memory::MemoryCreatorProxy;

use self::memory::create_memory;
use self::table::create_table;
use self::tag::create_tag;
use crate::prelude::*;
use crate::runtime::vm::{ExportMemory, SharedMemory};
use crate::store::{StoreOpaque, StoreResourceLimiter};
use crate::{MemoryType, TableType, TagType};
use wasmtime_environ::{MemoryIndex, TableIndex, TagIndex};

pub async fn generate_memory_export(
    store: &mut StoreOpaque,
    limiter: Option<&mut StoreResourceLimiter<'_>>,
    m: &MemoryType,
    preallocation: Option<&SharedMemory>,
) -> Result<ExportMemory> {
    let id = store.id();
    let instance = create_memory(store, limiter, m, preallocation).await?;
    Ok(store
        .instance_mut(instance)
        .get_exported_memory(id, MemoryIndex::from_u32(0)))
}

pub async fn generate_table_export(
    store: &mut StoreOpaque,
    limiter: Option<&mut StoreResourceLimiter<'_>>,
    t: &TableType,
) -> Result<crate::Table> {
    let id = store.id();
    let instance = create_table(store, limiter, t).await?;
    Ok(store
        .instance_mut(instance)
        .get_exported_table(id, TableIndex::from_u32(0)))
}

pub fn generate_tag_export(store: &mut StoreOpaque, t: &TagType) -> Result<crate::Tag> {
    let id = store.id();
    let instance = create_tag(store, t)?;
    Ok(store
        .instance_mut(instance)
        .get_exported_tag(id, TagIndex::from_u32(0)))
}
