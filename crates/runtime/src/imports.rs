use crate::vmcontext::{VMFunctionImport, VMGlobalImport, VMMemoryImport, VMTableImport};

/// Resolved import pointers.
///
/// Note that each of these fields are slices, not `PrimaryMap`. They should be
/// stored in index-order as with the module that we're providing the imports
/// for, and indexing is all done the same way as the main module's index
/// spaces.
#[derive(Clone, Default)]
pub struct Imports<'a> {
    /// Resolved addresses for imported functions.
    pub functions: &'a [VMFunctionImport],

    /// Resolved addresses for imported tables.
    pub tables: &'a [VMTableImport],

    /// Resolved addresses for imported memories.
    pub memories: &'a [VMMemoryImport],

    /// Resolved addresses for imported globals.
    pub globals: &'a [VMGlobalImport],
}
