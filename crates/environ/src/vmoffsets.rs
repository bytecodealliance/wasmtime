//! Offsets and sizes of various structs in `wasmtime::runtime::vm::*` that are
//! accessed directly by compiled Wasm code.

// Currently the `VMContext` allocation by field looks like this:
//
// struct VMContext {
//      magic: u32,
//      _padding: u32, // (On 64-bit systems)
//      runtime_limits: *const VMRuntimeLimits,
//      builtins: *mut VMBuiltinFunctionsArray,
//      callee: *mut VMFunctionBody,
//      gc_heap_base: *mut u8,
//      gc_heap_bound: *mut u8,
//      gc_heap_data: *mut T, // Collector-specific pointer
//      store: *mut dyn Store,
//      type_ids: *const VMSharedTypeIndex,
//      imported_functions: [VMFunctionImport; module.num_imported_functions],
//      imported_tables: [VMTableImport; module.num_imported_tables],
//      imported_memories: [VMMemoryImport; module.num_imported_memories],
//      imported_globals: [VMGlobalImport; module.num_imported_globals],
//      tables: [VMTableDefinition; module.num_defined_tables],
//      memories: [*mut VMMemoryDefinition; module.num_defined_memories],
//      owned_memories: [VMMemoryDefinition; module.num_owned_memories],
//      globals: [VMGlobalDefinition; module.num_defined_globals],
//      func_refs: [VMFuncRef; module.num_escaped_funcs],
//      call_indirect_caches: [VMCallIndirectCache; module.num_call_indirect_caches],
// }

use crate::{
    CallIndirectSiteIndex, DefinedGlobalIndex, DefinedMemoryIndex, DefinedTableIndex, FuncIndex,
    FuncRefIndex, GlobalIndex, MemoryIndex, Module, TableIndex,
};
use cranelift_entity::packed_option::ReservedValue;
use wasmtime_types::OwnedMemoryIndex;

#[cfg(target_pointer_width = "32")]
fn cast_to_u32(sz: usize) -> u32 {
    u32::try_from(sz).unwrap()
}
#[cfg(target_pointer_width = "64")]
fn cast_to_u32(sz: usize) -> u32 {
    u32::try_from(sz).expect("overflow in cast from usize to u32")
}

/// Align an offset used in this module to a specific byte-width by rounding up
#[inline]
fn align(offset: u32, width: u32) -> u32 {
    (offset + (width - 1)) / width * width
}

/// This class computes offsets to fields within `VMContext` and other
/// related structs that JIT code accesses directly.
#[derive(Debug, Clone, Copy)]
pub struct VMOffsets<P> {
    /// The size in bytes of a pointer on the target.
    pub ptr: P,
    /// The number of imported functions in the module.
    pub num_imported_functions: u32,
    /// The number of imported tables in the module.
    pub num_imported_tables: u32,
    /// The number of imported memories in the module.
    pub num_imported_memories: u32,
    /// The number of imported globals in the module.
    pub num_imported_globals: u32,
    /// The number of defined tables in the module.
    pub num_defined_tables: u32,
    /// The number of defined memories in the module.
    pub num_defined_memories: u32,
    /// The number of memories owned by the module instance.
    pub num_owned_memories: u32,
    /// The number of defined globals in the module.
    pub num_defined_globals: u32,
    /// The number of escaped functions in the module, the size of the func_refs
    /// array.
    pub num_escaped_funcs: u32,
    /// The number of call_indirect cache entries in the cache array.
    pub num_call_indirect_caches: u32,

    // precalculated offsets of various member fields
    magic: u32,
    runtime_limits: u32,
    builtin_functions: u32,
    callee: u32,
    epoch_ptr: u32,
    gc_heap_base: u32,
    gc_heap_bound: u32,
    gc_heap_data: u32,
    store: u32,
    type_ids: u32,
    imported_functions: u32,
    imported_tables: u32,
    imported_memories: u32,
    imported_globals: u32,
    defined_tables: u32,
    defined_memories: u32,
    owned_memories: u32,
    defined_globals: u32,
    defined_func_refs: u32,
    call_indirect_caches: u32,
    size: u32,
}

/// Trait used for the `ptr` representation of the field of `VMOffsets`
pub trait PtrSize {
    /// Returns the pointer size, in bytes, for the target.
    fn size(&self) -> u8;

    /// The offset of the `VMContext::runtime_limits` field
    fn vmcontext_runtime_limits(&self) -> u8 {
        u8::try_from(align(
            u32::try_from(core::mem::size_of::<u32>()).unwrap(),
            u32::from(self.size()),
        ))
        .unwrap()
    }

    /// The offset of the `VMContext::builtin_functions` field
    fn vmcontext_builtin_functions(&self) -> u8 {
        self.vmcontext_runtime_limits() + self.size()
    }

    /// The offset of the `array_call` field.
    #[inline]
    fn vm_func_ref_array_call(&self) -> u8 {
        0 * self.size()
    }

    /// The offset of the `wasm_call` field.
    #[inline]
    fn vm_func_ref_wasm_call(&self) -> u8 {
        1 * self.size()
    }

    /// The offset of the `type_index` field.
    #[inline]
    fn vm_func_ref_type_index(&self) -> u8 {
        2 * self.size()
    }

    /// The offset of the `vmctx` field.
    #[inline]
    fn vm_func_ref_vmctx(&self) -> u8 {
        3 * self.size()
    }

    /// Return the size of `VMFuncRef`.
    #[inline]
    fn size_of_vm_func_ref(&self) -> u8 {
        4 * self.size()
    }

    /// Return the size of `VMGlobalDefinition`; this is the size of the largest value type (i.e. a
    /// V128).
    #[inline]
    fn size_of_vmglobal_definition(&self) -> u8 {
        16
    }

    // Offsets within `VMRuntimeLimits`

    /// Return the offset of the `stack_limit` field of `VMRuntimeLimits`
    #[inline]
    fn vmruntime_limits_stack_limit(&self) -> u8 {
        0
    }

    /// Return the offset of the `fuel_consumed` field of `VMRuntimeLimits`
    #[inline]
    fn vmruntime_limits_fuel_consumed(&self) -> u8 {
        self.size()
    }

    /// Return the offset of the `epoch_deadline` field of `VMRuntimeLimits`
    #[inline]
    fn vmruntime_limits_epoch_deadline(&self) -> u8 {
        self.vmruntime_limits_fuel_consumed() + 8 // `stack_limit` is a pointer; `fuel_consumed` is an `i64`
    }

    /// Return the offset of the `last_wasm_exit_fp` field of `VMRuntimeLimits`.
    fn vmruntime_limits_last_wasm_exit_fp(&self) -> u8 {
        self.vmruntime_limits_epoch_deadline() + 8
    }

    /// Return the offset of the `last_wasm_exit_pc` field of `VMRuntimeLimits`.
    fn vmruntime_limits_last_wasm_exit_pc(&self) -> u8 {
        self.vmruntime_limits_last_wasm_exit_fp() + self.size()
    }

    /// Return the offset of the `last_enty_sp` field of `VMRuntimeLimits`.
    fn vmruntime_limits_last_wasm_entry_sp(&self) -> u8 {
        self.vmruntime_limits_last_wasm_exit_pc() + self.size()
    }

    // Offsets within `VMMemoryDefinition`

    /// The offset of the `base` field.
    #[inline]
    fn vmmemory_definition_base(&self) -> u8 {
        0 * self.size()
    }

    /// The offset of the `current_length` field.
    #[inline]
    fn vmmemory_definition_current_length(&self) -> u8 {
        1 * self.size()
    }

    /// Return the size of `VMMemoryDefinition`.
    #[inline]
    fn size_of_vmmemory_definition(&self) -> u8 {
        3 * self.size()
    }

    /// Return the size of `*mut VMMemoryDefinition`.
    #[inline]
    fn size_of_vmmemory_pointer(&self) -> u8 {
        self.size()
    }

    // Offsets within `VMArrayCallHostFuncContext`.

    /// Return the offset of `VMArrayCallHostFuncContext::func_ref`.
    fn vmarray_call_host_func_context_func_ref(&self) -> u8 {
        u8::try_from(align(
            u32::try_from(core::mem::size_of::<u32>()).unwrap(),
            u32::from(self.size()),
        ))
        .unwrap()
    }

    // Offsets within `VMCallIndirectCache`.

    /// Return the offset of `VMCallIndirectCache::wasm_call`.
    fn vmcall_indirect_cache_wasm_call(&self) -> u8 {
        0
    }

    /// Return the offset of `VMCallIndirectCache::index`.
    fn vmcall_indirect_cache_index(&self) -> u8 {
        self.size()
    }

    /// Return the size of a `VMCallIndirectCache`.
    fn size_of_vmcall_indirect_cache(&self) -> u8 {
        2 * self.size()
    }
}

/// Type representing the size of a pointer for the current compilation host
#[derive(Clone, Copy)]
pub struct HostPtr;

impl PtrSize for HostPtr {
    #[inline]
    fn size(&self) -> u8 {
        core::mem::size_of::<usize>() as u8
    }
}

impl PtrSize for u8 {
    #[inline]
    fn size(&self) -> u8 {
        *self
    }
}

/// Used to construct a `VMOffsets`
#[derive(Debug, Clone, Copy)]
pub struct VMOffsetsFields<P> {
    /// The size in bytes of a pointer on the target.
    pub ptr: P,
    /// The number of imported functions in the module.
    pub num_imported_functions: u32,
    /// The number of imported tables in the module.
    pub num_imported_tables: u32,
    /// The number of imported memories in the module.
    pub num_imported_memories: u32,
    /// The number of imported globals in the module.
    pub num_imported_globals: u32,
    /// The number of defined tables in the module.
    pub num_defined_tables: u32,
    /// The number of defined memories in the module.
    pub num_defined_memories: u32,
    /// The number of memories owned by the module instance.
    pub num_owned_memories: u32,
    /// The number of defined globals in the module.
    pub num_defined_globals: u32,
    /// The number of escaped functions in the module, the size of the function
    /// references array.
    pub num_escaped_funcs: u32,
    /// The number of call_indirect cache entries in the cache array.
    pub num_call_indirect_caches: u32,
}

impl<P: PtrSize> VMOffsets<P> {
    /// Return a new `VMOffsets` instance, for a given pointer size.
    pub fn new(ptr: P, module: &Module) -> Self {
        let num_owned_memories = module
            .memory_plans
            .iter()
            .skip(module.num_imported_memories)
            .filter(|p| !p.1.memory.shared)
            .count()
            .try_into()
            .unwrap();
        VMOffsets::from(VMOffsetsFields {
            ptr,
            num_imported_functions: cast_to_u32(module.num_imported_funcs),
            num_imported_tables: cast_to_u32(module.num_imported_tables),
            num_imported_memories: cast_to_u32(module.num_imported_memories),
            num_imported_globals: cast_to_u32(module.num_imported_globals),
            num_defined_tables: cast_to_u32(module.table_plans.len() - module.num_imported_tables),
            num_defined_memories: cast_to_u32(
                module.memory_plans.len() - module.num_imported_memories,
            ),
            num_owned_memories,
            num_defined_globals: cast_to_u32(module.globals.len() - module.num_imported_globals),
            num_escaped_funcs: cast_to_u32(module.num_escaped_funcs),
            num_call_indirect_caches: cast_to_u32(module.num_call_indirect_caches),
        })
    }

    /// Returns the size, in bytes, of the target
    #[inline]
    pub fn pointer_size(&self) -> u8 {
        self.ptr.size()
    }

    /// Returns an iterator which provides a human readable description and a
    /// byte size. The iterator returned will iterate over the bytes allocated
    /// to the entire `VMOffsets` structure to explain where each byte size is
    /// coming from.
    pub fn region_sizes(&self) -> impl Iterator<Item = (&str, u32)> {
        macro_rules! calculate_sizes {
            ($($name:ident: $desc:tt,)*) => {{
                let VMOffsets {
                    // These fields are metadata not talking about specific
                    // offsets of specific fields.
                    ptr: _,
                    num_imported_functions: _,
                    num_imported_tables: _,
                    num_imported_memories: _,
                    num_imported_globals: _,
                    num_defined_tables: _,
                    num_defined_globals: _,
                    num_defined_memories: _,
                    num_owned_memories: _,
                    num_escaped_funcs: _,
                    num_call_indirect_caches: _,

                    // used as the initial size below
                    size,

                    // exhaustively match the rest of the fields with input from
                    // the macro
                    $($name,)*
                } = *self;

                // calculate the size of each field by relying on the inputs to
                // the macro being in reverse order and determining the size of
                // the field as the offset from the field to the last field.
                let mut last = size;
                $(
                    assert!($name <= last);
                    let tmp = $name;
                    let $name = last - $name;
                    last = tmp;
                )*
                assert_eq!(last, 0);
                IntoIterator::into_iter([$(($desc, $name),)*])
            }};
        }

        calculate_sizes! {
            call_indirect_caches: "call_indirect caches",
            defined_func_refs: "module functions",
            defined_globals: "defined globals",
            owned_memories: "owned memories",
            defined_memories: "defined memories",
            defined_tables: "defined tables",
            imported_globals: "imported globals",
            imported_memories: "imported memories",
            imported_tables: "imported tables",
            imported_functions: "imported functions",
            type_ids: "module types",
            store: "jit store state",
            gc_heap_data: "GC implementation-specific data",
            gc_heap_bound: "GC heap bound",
            gc_heap_base: "GC heap base",
            epoch_ptr: "jit current epoch state",
            callee: "callee function pointer",
            builtin_functions: "jit builtin functions state",
            runtime_limits: "jit runtime limits state",
            magic: "magic value",
        }
    }
}

impl<P: PtrSize> From<VMOffsetsFields<P>> for VMOffsets<P> {
    fn from(fields: VMOffsetsFields<P>) -> VMOffsets<P> {
        let mut ret = Self {
            ptr: fields.ptr,
            num_imported_functions: fields.num_imported_functions,
            num_imported_tables: fields.num_imported_tables,
            num_imported_memories: fields.num_imported_memories,
            num_imported_globals: fields.num_imported_globals,
            num_defined_tables: fields.num_defined_tables,
            num_defined_memories: fields.num_defined_memories,
            num_owned_memories: fields.num_owned_memories,
            num_defined_globals: fields.num_defined_globals,
            num_escaped_funcs: fields.num_escaped_funcs,
            num_call_indirect_caches: fields.num_call_indirect_caches,
            magic: 0,
            runtime_limits: 0,
            builtin_functions: 0,
            callee: 0,
            epoch_ptr: 0,
            gc_heap_base: 0,
            gc_heap_bound: 0,
            gc_heap_data: 0,
            store: 0,
            type_ids: 0,
            imported_functions: 0,
            imported_tables: 0,
            imported_memories: 0,
            imported_globals: 0,
            defined_tables: 0,
            defined_memories: 0,
            owned_memories: 0,
            defined_globals: 0,
            defined_func_refs: 0,
            call_indirect_caches: 0,
            size: 0,
        };

        // Convenience functions for checked addition and multiplication.
        // As side effect this reduces binary size by using only a single
        // `#[track_caller]` location for each function instead of one for
        // each individual invocation.
        #[inline]
        fn cadd(count: u32, size: u32) -> u32 {
            count.checked_add(size).unwrap()
        }

        #[inline]
        fn cmul(count: u32, size: u8) -> u32 {
            count.checked_mul(u32::from(size)).unwrap()
        }

        let mut next_field_offset = 0;

        macro_rules! fields {
            (size($field:ident) = $size:expr, $($rest:tt)*) => {
                ret.$field = next_field_offset;
                next_field_offset = cadd(next_field_offset, u32::from($size));
                fields!($($rest)*);
            };
            (align($align:expr), $($rest:tt)*) => {
                next_field_offset = align(next_field_offset, $align);
                fields!($($rest)*);
            };
            () => {};
        }

        fields! {
            size(magic) = 4u32,
            align(u32::from(ret.ptr.size())),
            size(runtime_limits) = ret.ptr.size(),
            size(builtin_functions) = ret.ptr.size(),
            size(callee) = ret.ptr.size(),
            size(epoch_ptr) = ret.ptr.size(),
            size(gc_heap_base) = ret.ptr.size(),
            size(gc_heap_bound) = ret.ptr.size(),
            size(gc_heap_data) = ret.ptr.size(),
            size(store) = ret.ptr.size() * 2,
            size(type_ids) = ret.ptr.size(),
            size(imported_functions)
                = cmul(ret.num_imported_functions, ret.size_of_vmfunction_import()),
            size(imported_tables)
                = cmul(ret.num_imported_tables, ret.size_of_vmtable_import()),
            size(imported_memories)
                = cmul(ret.num_imported_memories, ret.size_of_vmmemory_import()),
            size(imported_globals)
                = cmul(ret.num_imported_globals, ret.size_of_vmglobal_import()),
            size(defined_tables)
                = cmul(ret.num_defined_tables, ret.size_of_vmtable_definition()),
            size(defined_memories)
                = cmul(ret.num_defined_memories, ret.ptr.size_of_vmmemory_pointer()),
            size(owned_memories)
                = cmul(ret.num_owned_memories, ret.ptr.size_of_vmmemory_definition()),
            align(16),
            size(defined_globals)
                = cmul(ret.num_defined_globals, ret.ptr.size_of_vmglobal_definition()),
            size(defined_func_refs) = cmul(
                ret.num_escaped_funcs,
                ret.ptr.size_of_vm_func_ref(),
            ),
            size(call_indirect_caches) = cmul(
                ret.num_call_indirect_caches,
                ret.ptr.size_of_vmcall_indirect_cache(),
            ),
        }

        ret.size = next_field_offset;

        // This is required by the implementation of `VMContext::instance` and
        // `VMContext::instance_mut`. If this value changes then those locations
        // need to be updated.
        assert_eq!(ret.magic, 0);

        return ret;
    }
}

impl<P: PtrSize> VMOffsets<P> {
    /// The offset of the `wasm_call` field.
    #[inline]
    pub fn vmfunction_import_wasm_call(&self) -> u8 {
        0 * self.pointer_size()
    }

    /// The offset of the `array_call` field.
    #[inline]
    pub fn vmfunction_import_array_call(&self) -> u8 {
        1 * self.pointer_size()
    }

    /// The offset of the `vmctx` field.
    #[inline]
    pub fn vmfunction_import_vmctx(&self) -> u8 {
        2 * self.pointer_size()
    }

    /// Return the size of `VMFunctionImport`.
    #[inline]
    pub fn size_of_vmfunction_import(&self) -> u8 {
        3 * self.pointer_size()
    }
}

/// Offsets for `*const VMFunctionBody`.
impl<P: PtrSize> VMOffsets<P> {
    /// The size of the `current_elements` field.
    pub fn size_of_vmfunction_body_ptr(&self) -> u8 {
        1 * self.pointer_size()
    }
}

/// Offsets for `VMTableImport`.
impl<P: PtrSize> VMOffsets<P> {
    /// The offset of the `from` field.
    #[inline]
    pub fn vmtable_import_from(&self) -> u8 {
        0 * self.pointer_size()
    }

    /// The offset of the `vmctx` field.
    #[inline]
    pub fn vmtable_import_vmctx(&self) -> u8 {
        1 * self.pointer_size()
    }

    /// Return the size of `VMTableImport`.
    #[inline]
    pub fn size_of_vmtable_import(&self) -> u8 {
        2 * self.pointer_size()
    }
}

/// Offsets for `VMTableDefinition`.
impl<P: PtrSize> VMOffsets<P> {
    /// The offset of the `base` field.
    #[inline]
    pub fn vmtable_definition_base(&self) -> u8 {
        0 * self.pointer_size()
    }

    /// The offset of the `current_elements` field.
    pub fn vmtable_definition_current_elements(&self) -> u8 {
        1 * self.pointer_size()
    }

    /// The size of the `current_elements` field.
    #[inline]
    pub fn size_of_vmtable_definition_current_elements(&self) -> u8 {
        4
    }

    /// Return the size of `VMTableDefinition`.
    #[inline]
    pub fn size_of_vmtable_definition(&self) -> u8 {
        2 * self.pointer_size()
    }
}

/// Offsets for `VMMemoryImport`.
impl<P: PtrSize> VMOffsets<P> {
    /// The offset of the `from` field.
    #[inline]
    pub fn vmmemory_import_from(&self) -> u8 {
        0 * self.pointer_size()
    }

    /// The offset of the `vmctx` field.
    #[inline]
    pub fn vmmemory_import_vmctx(&self) -> u8 {
        1 * self.pointer_size()
    }

    /// Return the size of `VMMemoryImport`.
    #[inline]
    pub fn size_of_vmmemory_import(&self) -> u8 {
        3 * self.pointer_size()
    }
}

/// Offsets for `VMGlobalImport`.
impl<P: PtrSize> VMOffsets<P> {
    /// The offset of the `from` field.
    #[inline]
    pub fn vmglobal_import_from(&self) -> u8 {
        0 * self.pointer_size()
    }

    /// Return the size of `VMGlobalImport`.
    #[inline]
    pub fn size_of_vmglobal_import(&self) -> u8 {
        1 * self.pointer_size()
    }
}

/// Offsets for `VMSharedTypeIndex`.
impl<P: PtrSize> VMOffsets<P> {
    /// Return the size of `VMSharedTypeIndex`.
    #[inline]
    pub fn size_of_vmshared_type_index(&self) -> u8 {
        4
    }
}

/// Offsets for `VMContext`.
impl<P: PtrSize> VMOffsets<P> {
    /// Return the offset to the `magic` value in this `VMContext`.
    #[inline]
    pub fn vmctx_magic(&self) -> u32 {
        self.magic
    }

    /// Return the offset to the `VMRuntimeLimits` structure
    #[inline]
    pub fn vmctx_runtime_limits(&self) -> u32 {
        self.runtime_limits
    }

    /// Return the offset to the `callee` member in this `VMContext`.
    pub fn vmctx_callee(&self) -> u32 {
        self.callee
    }

    /// Return the offset to the `*const AtomicU64` epoch-counter
    /// pointer.
    #[inline]
    pub fn vmctx_epoch_ptr(&self) -> u32 {
        self.epoch_ptr
    }

    /// Return the offset to the GC heap base in this `VMContext`.
    #[inline]
    pub fn vmctx_gc_heap_base(&self) -> u32 {
        self.gc_heap_base
    }

    /// Return the offset to the GC heap bound in this `VMContext`.
    #[inline]
    pub fn vmctx_gc_heap_bound(&self) -> u32 {
        self.gc_heap_bound
    }

    /// Return the offset to the `*mut T` collector-specific data.
    ///
    /// This is a pointer that different collectors can use however they see
    /// fit.
    #[inline]
    pub fn vmctx_gc_heap_data(&self) -> u32 {
        self.gc_heap_data
    }

    /// The offset of the `*const dyn Store` member.
    #[inline]
    pub fn vmctx_store(&self) -> u32 {
        self.store
    }

    /// The offset of the `type_ids` array pointer.
    #[inline]
    pub fn vmctx_type_ids_array(&self) -> u32 {
        self.type_ids
    }

    /// The offset of the `tables` array.
    #[inline]
    pub fn vmctx_imported_functions_begin(&self) -> u32 {
        self.imported_functions
    }

    /// The offset of the `tables` array.
    #[inline]
    pub fn vmctx_imported_tables_begin(&self) -> u32 {
        self.imported_tables
    }

    /// The offset of the `memories` array.
    #[inline]
    pub fn vmctx_imported_memories_begin(&self) -> u32 {
        self.imported_memories
    }

    /// The offset of the `globals` array.
    #[inline]
    pub fn vmctx_imported_globals_begin(&self) -> u32 {
        self.imported_globals
    }

    /// The offset of the `tables` array.
    #[inline]
    pub fn vmctx_tables_begin(&self) -> u32 {
        self.defined_tables
    }

    /// The offset of the `memories` array.
    #[inline]
    pub fn vmctx_memories_begin(&self) -> u32 {
        self.defined_memories
    }

    /// The offset of the `owned_memories` array.
    #[inline]
    pub fn vmctx_owned_memories_begin(&self) -> u32 {
        self.owned_memories
    }

    /// The offset of the `globals` array.
    #[inline]
    pub fn vmctx_globals_begin(&self) -> u32 {
        self.defined_globals
    }

    /// The offset of the `func_refs` array.
    #[inline]
    pub fn vmctx_func_refs_begin(&self) -> u32 {
        self.defined_func_refs
    }

    /// The offset of the builtin functions array.
    #[inline]
    pub fn vmctx_builtin_functions(&self) -> u32 {
        self.builtin_functions
    }

    /// The offset of the `call_indirect_caches` array.
    #[inline]
    pub fn vmctx_call_indirec_caches_begin(&self) -> u32 {
        self.call_indirect_caches
    }

    /// Return the size of the `VMContext` allocation.
    #[inline]
    pub fn size_of_vmctx(&self) -> u32 {
        self.size
    }

    /// Return the offset to `VMFunctionImport` index `index`.
    #[inline]
    pub fn vmctx_vmfunction_import(&self, index: FuncIndex) -> u32 {
        assert!(index.as_u32() < self.num_imported_functions);
        self.vmctx_imported_functions_begin()
            + index.as_u32() * u32::from(self.size_of_vmfunction_import())
    }

    /// Return the offset to `VMTableImport` index `index`.
    #[inline]
    pub fn vmctx_vmtable_import(&self, index: TableIndex) -> u32 {
        assert!(index.as_u32() < self.num_imported_tables);
        self.vmctx_imported_tables_begin()
            + index.as_u32() * u32::from(self.size_of_vmtable_import())
    }

    /// Return the offset to `VMMemoryImport` index `index`.
    #[inline]
    pub fn vmctx_vmmemory_import(&self, index: MemoryIndex) -> u32 {
        assert!(index.as_u32() < self.num_imported_memories);
        self.vmctx_imported_memories_begin()
            + index.as_u32() * u32::from(self.size_of_vmmemory_import())
    }

    /// Return the offset to `VMGlobalImport` index `index`.
    #[inline]
    pub fn vmctx_vmglobal_import(&self, index: GlobalIndex) -> u32 {
        assert!(index.as_u32() < self.num_imported_globals);
        self.vmctx_imported_globals_begin()
            + index.as_u32() * u32::from(self.size_of_vmglobal_import())
    }

    /// Return the offset to `VMTableDefinition` index `index`.
    #[inline]
    pub fn vmctx_vmtable_definition(&self, index: DefinedTableIndex) -> u32 {
        assert!(index.as_u32() < self.num_defined_tables);
        self.vmctx_tables_begin() + index.as_u32() * u32::from(self.size_of_vmtable_definition())
    }

    /// Return the offset to the `*mut VMMemoryDefinition` at index `index`.
    #[inline]
    pub fn vmctx_vmmemory_pointer(&self, index: DefinedMemoryIndex) -> u32 {
        assert!(index.as_u32() < self.num_defined_memories);
        self.vmctx_memories_begin()
            + index.as_u32() * u32::from(self.ptr.size_of_vmmemory_pointer())
    }

    /// Return the offset to the owned `VMMemoryDefinition` at index `index`.
    #[inline]
    pub fn vmctx_vmmemory_definition(&self, index: OwnedMemoryIndex) -> u32 {
        assert!(index.as_u32() < self.num_owned_memories);
        self.vmctx_owned_memories_begin()
            + index.as_u32() * u32::from(self.ptr.size_of_vmmemory_definition())
    }

    /// Return the offset to the `VMGlobalDefinition` index `index`.
    #[inline]
    pub fn vmctx_vmglobal_definition(&self, index: DefinedGlobalIndex) -> u32 {
        assert!(index.as_u32() < self.num_defined_globals);
        self.vmctx_globals_begin()
            + index.as_u32() * u32::from(self.ptr.size_of_vmglobal_definition())
    }

    /// Return the offset to the `VMFuncRef` for the given function
    /// index (either imported or defined).
    #[inline]
    pub fn vmctx_func_ref(&self, index: FuncRefIndex) -> u32 {
        assert!(!index.is_reserved_value());
        assert!(index.as_u32() < self.num_escaped_funcs);
        self.vmctx_func_refs_begin() + index.as_u32() * u32::from(self.ptr.size_of_vm_func_ref())
    }

    /// Return the offset to the `wasm_call` field in `*const VMFunctionBody` index `index`.
    #[inline]
    pub fn vmctx_vmfunction_import_wasm_call(&self, index: FuncIndex) -> u32 {
        self.vmctx_vmfunction_import(index) + u32::from(self.vmfunction_import_wasm_call())
    }

    /// Return the offset to the `array_call` field in `*const VMFunctionBody` index `index`.
    #[inline]
    pub fn vmctx_vmfunction_import_array_call(&self, index: FuncIndex) -> u32 {
        self.vmctx_vmfunction_import(index) + u32::from(self.vmfunction_import_array_call())
    }

    /// Return the offset to the `vmctx` field in `*const VMFunctionBody` index `index`.
    #[inline]
    pub fn vmctx_vmfunction_import_vmctx(&self, index: FuncIndex) -> u32 {
        self.vmctx_vmfunction_import(index) + u32::from(self.vmfunction_import_vmctx())
    }

    /// Return the offset to the `from` field in `VMTableImport` index `index`.
    #[inline]
    pub fn vmctx_vmtable_import_from(&self, index: TableIndex) -> u32 {
        self.vmctx_vmtable_import(index) + u32::from(self.vmtable_import_from())
    }

    /// Return the offset to the `base` field in `VMTableDefinition` index `index`.
    #[inline]
    pub fn vmctx_vmtable_definition_base(&self, index: DefinedTableIndex) -> u32 {
        self.vmctx_vmtable_definition(index) + u32::from(self.vmtable_definition_base())
    }

    /// Return the offset to the `current_elements` field in `VMTableDefinition` index `index`.
    #[inline]
    pub fn vmctx_vmtable_definition_current_elements(&self, index: DefinedTableIndex) -> u32 {
        self.vmctx_vmtable_definition(index) + u32::from(self.vmtable_definition_current_elements())
    }

    /// Return the offset to the `from` field in `VMMemoryImport` index `index`.
    #[inline]
    pub fn vmctx_vmmemory_import_from(&self, index: MemoryIndex) -> u32 {
        self.vmctx_vmmemory_import(index) + u32::from(self.vmmemory_import_from())
    }

    /// Return the offset to the `vmctx` field in `VMMemoryImport` index `index`.
    #[inline]
    pub fn vmctx_vmmemory_import_vmctx(&self, index: MemoryIndex) -> u32 {
        self.vmctx_vmmemory_import(index) + u32::from(self.vmmemory_import_vmctx())
    }

    /// Return the offset to the `base` field in `VMMemoryDefinition` index `index`.
    #[inline]
    pub fn vmctx_vmmemory_definition_base(&self, index: OwnedMemoryIndex) -> u32 {
        self.vmctx_vmmemory_definition(index) + u32::from(self.ptr.vmmemory_definition_base())
    }

    /// Return the offset to the `current_length` field in `VMMemoryDefinition` index `index`.
    #[inline]
    pub fn vmctx_vmmemory_definition_current_length(&self, index: OwnedMemoryIndex) -> u32 {
        self.vmctx_vmmemory_definition(index)
            + u32::from(self.ptr.vmmemory_definition_current_length())
    }

    /// Return the offset to the `from` field in `VMGlobalImport` index `index`.
    #[inline]
    pub fn vmctx_vmglobal_import_from(&self, index: GlobalIndex) -> u32 {
        self.vmctx_vmglobal_import(index) + u32::from(self.vmglobal_import_from())
    }

    /// Return the offset to the `VMCallIndirectCache` for the given
    /// call-indirect site.
    #[inline]
    pub fn vmctx_call_indirect_cache(&self, call_site: CallIndirectSiteIndex) -> u32 {
        assert!(call_site.as_u32() < self.num_call_indirect_caches);
        self.vmctx_call_indirec_caches_begin()
            + call_site.as_u32() * u32::from(self.ptr.size_of_vmcall_indirect_cache())
    }

    /// Return the offset to the `wasm_call` field in `*const
    /// VMCallIndirectCache` with call-site ID `call_site`.
    #[inline]
    pub fn vmctx_call_indirect_cache_wasm_call(&self, call_site: CallIndirectSiteIndex) -> u32 {
        self.vmctx_call_indirect_cache(call_site)
            + u32::from(self.ptr.vmcall_indirect_cache_wasm_call())
    }

    /// Return the offset to the `index` field in `*const
    /// VMCallIndirectCache` with call-site ID `call_site`.
    #[inline]
    pub fn vmctx_call_indirect_cache_index(&self, call_site: CallIndirectSiteIndex) -> u32 {
        self.vmctx_call_indirect_cache(call_site)
            + u32::from(self.ptr.vmcall_indirect_cache_index())
    }
}

/// Offsets for `VMDrcHeader`.
///
/// Should only be used when the DRC collector is enabled.
impl<P: PtrSize> VMOffsets<P> {
    /// Return the offset for `VMDrcHeader::ref_count`.
    #[inline]
    pub fn vm_drc_header_ref_count(&self) -> u32 {
        8
    }
}

/// Offsets for `VMGcRefActivationsTable`.
///
/// These should only be used when the DRC collector is enabled.
impl<P: PtrSize> VMOffsets<P> {
    /// Return the offset for `VMGcRefActivationsTable::next`.
    #[inline]
    pub fn vm_gc_ref_activation_table_next(&self) -> u32 {
        0
    }

    /// Return the offset for `VMGcRefActivationsTable::end`.
    #[inline]
    pub fn vm_gc_ref_activation_table_end(&self) -> u32 {
        self.pointer_size().into()
    }
}

/// Magic value for core Wasm VM contexts.
///
/// This is stored at the start of all `VMContext` structures.
pub const VMCONTEXT_MAGIC: u32 = u32::from_le_bytes(*b"core");

/// Equivalent of `VMCONTEXT_MAGIC` except for array-call host functions.
///
/// This is stored at the start of all `VMArrayCallHostFuncContext` structures
/// and double-checked on `VMArrayCallHostFuncContext::from_opaque`.
pub const VM_ARRAY_CALL_HOST_FUNC_MAGIC: u32 = u32::from_le_bytes(*b"ACHF");

#[cfg(test)]
mod tests {
    use crate::vmoffsets::align;

    #[test]
    fn alignment() {
        fn is_aligned(x: u32) -> bool {
            x % 16 == 0
        }
        assert!(is_aligned(align(0, 16)));
        assert!(is_aligned(align(32, 16)));
        assert!(is_aligned(align(33, 16)));
        assert!(is_aligned(align(31, 16)));
    }
}
