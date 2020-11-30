use crate::vmcontext::{VMFunctionImport, VMGlobalImport, VMMemoryImport, VMTableImport};
use crate::InstanceHandle;
use std::any::Any;
use wasmtime_environ::entity::PrimaryMap;
use wasmtime_environ::wasm::{InstanceIndex, ModuleIndex};

/// Resolved import pointers.
///
/// Note that some of these fields are slices, not `PrimaryMap`. They should be
/// stored in index-order as with the module that we're providing the imports
/// for, and indexing is all done the same way as the main module's index
/// spaces.
///
/// Also note that the way we compile modules means that for the module linking
/// proposal all `alias` directives should map to imported items. This means
/// that each of these items aren't necessarily directly imported, but may be
/// aliased.
#[derive(Default)]
pub struct Imports<'a> {
    /// Resolved addresses for imported functions.
    pub functions: &'a [VMFunctionImport],

    /// Resolved addresses for imported tables.
    pub tables: &'a [VMTableImport],

    /// Resolved addresses for imported memories.
    pub memories: &'a [VMMemoryImport],

    /// Resolved addresses for imported globals.
    pub globals: &'a [VMGlobalImport],

    /// Resolved imported instances.
    pub instances: PrimaryMap<InstanceIndex, InstanceHandle>,

    /// Resolved imported modules.
    ///
    /// Note that `Box<Any>` here is chosen to allow the embedder of this crate
    /// to pick an appropriate representation of what module type should be. For
    /// example for the `wasmtime` crate it's `wasmtime::Module` but that's not
    /// defined way down here in this low crate.
    pub modules: PrimaryMap<ModuleIndex, Box<dyn Any>>,
}
