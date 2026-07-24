//! Centralized definitions of the various `VM*` types whose layout is shared
//! between the runtime (which uses the actual structures) and the compiler
//! (which uses the types' offsets and has per-type alias regions).
//!
//! To keep these in sync, the shape of each type is defined exactly once here,
//! via the higher-order [`for_each_vm_type!`] macro, and each consumer
//! generates its view of the type from that single source of truth.

/// Invoke the given macro `$mac` once, passing it the definitions of each of the
/// `VM*` types whose layout is shared between the runtime, compilation offsets,
/// and Cranelift alias regions.
///
/// This is a higher-order macro: callers define a `macro_rules!` macro that
/// matches the grammar defined below and pass its name as an argument to this
/// macro's invocation, e.g. `for_each_vm_type!(define_vm_types)`.
///
/// # Grammar
///
/// Each type is emitted as a struct definition preceded by:
///
/// * Doc-comment attributes (`#[doc = "..."]`).
///
/// * An optional `#[derive(...)]` attribute.
///
/// * A `#[repr(...)]` attribute.
///
/// * A `#[snake_name = <ident>]` attribute giving the type's name in
///   `snake_case`, used to generate accessor method names.
///
/// Each field may be preceded by doc-comment attributes and, optionally, the
/// marker attributes `#[readonly]` and/or `#[can_move]` (in that order) which
/// describe how Cranelift may treat loads and stores of that field.
#[macro_export]
macro_rules! for_each_vm_type {
    ($mac:ident) => {
        $mac! {
            /// The fields compiled code needs to access to utilize a WebAssembly linear
            /// memory defined within the instance, namely the start address and the
            /// size in bytes.
            #[derive(Debug)]
            #[repr(C)]
            #[snake_name = vm_memory_definition]
            pub struct VMMemoryDefinition {
                /// The start address.
                pub base: VmPtr<u8>,

                /// The current logical size of this linear memory in bytes.
                ///
                /// This is atomic because shared memories must be able to grow their length
                /// atomically. For relaxed access, see
                /// [`VMMemoryDefinition::current_length()`].
                pub current_length: AtomicUsize,
            }

            /// The fields compiled code needs to access to utilize a WebAssembly table
            /// defined within the instance.
            #[derive(Debug, Copy, Clone)]
            #[repr(C)]
            #[snake_name = vm_table_definition]
            pub struct VMTableDefinition {
                /// Pointer to the table data.
                pub base: VmPtr<u8>,

                /// The current number of elements in the table.
                pub current_elements: usize,
            }

            /// The storage for a WebAssembly global defined within the instance.
            ///
            /// TODO: Pack the globals more densely, rather than using the same size
            /// for every type.
            #[derive(Debug)]
            #[repr(C, align(16))]
            #[snake_name = vm_global_definition]
            pub struct VMGlobalDefinition {
                /// The raw storage for a global's value.
                storage: [u8; 16],
            }

            /// A WebAssembly tag defined within the instance.
            #[derive(Debug)]
            #[repr(C)]
            #[snake_name = vm_tag_definition]
            pub struct VMTagDefinition {
                /// Function signature's type id.
                pub type_index: VMSharedTypeIndex,
            }
        }
    };
}
