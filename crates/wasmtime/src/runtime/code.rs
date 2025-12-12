use crate::Engine;
use crate::Module;
use crate::module::ModuleRegistry;
use crate::vm::ModuleMemoryImageSource;
use crate::{code_memory::CodeMemory, type_registry::TypeCollection};
#[cfg(feature = "debug")]
use alloc::boxed::Box;
use alloc::sync::Arc;
use anyhow::Result;
use core::ops::{Add, Range, Sub};
use wasmtime_environ::DefinedFuncIndex;
use wasmtime_environ::ModuleTypes;
#[cfg(feature = "component-model")]
use wasmtime_environ::component::ComponentTypes;

macro_rules! define_pc_kind {
    ($ty:ident) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $ty(usize);

        impl Add<usize> for $ty {
            type Output = $ty;
            fn add(self, other: usize) -> $ty {
                $ty(self.0.wrapping_add(other))
            }
        }
        impl Sub<usize> for $ty {
            type Output = $ty;
            fn sub(self, other: usize) -> $ty {
                $ty(self.0.wrapping_sub(other))
            }
        }

        impl $ty {
            /// Is the given PC within this range? Give the relative
            /// offset if so.
            pub fn offset_of(range: Range<$ty>, pc: usize) -> Option<usize> {
                if pc >= range.start.0 && pc < range.end.0 {
                    Some(pc.wrapping_sub(range.start.0))
                } else {
                    None
                }
            }

            /// Get the raw PC value.
            pub fn raw(&self) -> usize {
                self.0
            }
        }
    };
}

// An address in "engine code": the original (canonical) copy of code
// for a compiled module.
//
// See [`EngineCode`] for more details.
define_pc_kind!(EngineCodePC);

// An address in "store code": the copy of code used when executing
// instances of a module in a given store.
//
// May or may not be the same as the engine code address -- a store
// is allowed to make a private copy of engine code as its store
// code.
//
// See [`StoreCode`] for more details.
define_pc_kind!(StoreCodePC);

impl StoreCodePC {
    /// Construct a StoreCodePC for search purposes from a raw PC
    /// observed during execution.
    pub fn from_raw(pc: usize) -> StoreCodePC {
        StoreCodePC(pc)
    }
}

/// Metadata about, and original machine code for, a loaded compiled
/// artifact in memory which is ready for Wasmtime to execute.
///
/// This structure is used in both `Module` and `Component`. For components it's
/// notably shared amongst the core wasm modules within a component and the
/// component itself. For core wasm modules this is uniquely owned within a
/// `Module`.
///
/// The `EngineCode` is "static", like a module: there is one copy per
/// unit of code. Instantiation of a module containing an `EngineCode`
/// into a `Store` produces a `StoreCode`. The latter is the owner of
/// the actual machine code that runs. This machine code *may* be a
/// read-only-shared copy of the original, or may be a private copy
/// that we have patched. The latter is useful for some kinds of
/// debugging/instrumentation, which is always scoped per-Store.
///
/// The `EngineCode` does *not* expose its underlying `CodeMemory`, to
/// guard against the original-copy being used in an inappropriate
/// (store-specific) context. Instead, it has accessors that provide
/// the instance-agnostic metadata from the `CodeMemory`, and
/// instance-specific data and code pointers can be obtained from the
/// `StoreCode`.
pub struct EngineCode {
    /// Actual underlying code which is executable and contains other
    /// compiled information.
    ///
    /// Note the `Arc` here is used to share this with `CompiledModule` and the
    /// global module registry of traps. While probably not strictly necessary
    /// and could be avoided with some refactorings is a hopefully a relatively
    /// minor `Arc` for now.
    ///
    /// As noted above, this is the *original* copy of the code, and
    /// may be used directly, but may also be deep-cloned to a private
    /// copy in a `StoreCode`.
    original_code: Arc<CodeMemory>,

    /// Registered shared signature for the loaded object.
    ///
    /// Note that this type has a significant destructor which unregisters
    /// signatures within the `Engine` it was originally tied to, and this ends
    /// up corresponding to the lifetime of a `Component` or `Module`.
    signatures: TypeCollection,

    /// Type information for the loaded object.
    ///
    /// This is either a `ModuleTypes` or a `ComponentTypes` depending on the
    /// top-level creator of this code.
    types: Types,
}

impl EngineCode {
    pub fn new(mmap: Arc<CodeMemory>, signatures: TypeCollection, types: Types) -> EngineCode {
        // The corresponding unregister for this is below in `Drop for
        // EngineCode`.
        crate::module::register_code(&mmap, mmap.raw_addr_range());

        EngineCode {
            original_code: mmap,
            signatures,
            types,
        }
    }

    #[cfg(feature = "component-model")]
    pub fn types(&self) -> &Types {
        &self.types
    }

    pub fn module_types(&self) -> &ModuleTypes {
        self.types.module_types()
    }

    pub fn signatures(&self) -> &TypeCollection {
        &self.signatures
    }

    pub fn text_size(&self) -> usize {
        self.original_code.text().len()
    }

    /// Give the range of engine-code PCs in this code.
    pub fn text_range(&self) -> Range<EngineCodePC> {
        let raw = self.original_code.raw_addr_range();
        EngineCodePC(raw.start)..EngineCodePC(raw.end)
    }

    // For all accessors to code object sections below: we can use the
    // original (uncloned) code image for these: they are just slices
    // of bytes that we interpret independent of location.
    //
    // We hide the raw `CodeMemory` in `original_code` (*not* a public
    // field) because we do not want the EngineCode copy used
    // inadvertently for execution of Wasm functions.
    //
    // We thus *cannot* add an accessor below for anything that
    // actually interprets the location of the bytes as a code
    // pointer to a Wasm function.
    //
    // Note that Wasm-to-array trampolines, in contrast, are fair game
    // to execute directly from the EngineCode: these are not specific
    // to Wasm functions, but instead call builtins, and we never
    // patch trampolines in StoreCode, so we can freely mix in
    // EngineCode variants.

    /// Get the Wasm-to-array trampoline for the given raw range in
    /// the text segment.
    pub(crate) fn raw_wasm_to_array_trampoline_data(&self, range: Range<usize>) -> &[u8] {
        &self.original_code.text()[range]
    }

    /// Provide the address-map data for this EngineCode.
    pub fn address_map_data(&self) -> &[u8] {
        // We can use the original (uncloned) code image for this: it
        // is just a slice of bytes that we interpret independent of
        // location.
        self.original_code.address_map_data()
    }

    /// Provide the stack-map data for this EngineCode.
    pub fn stack_map_data(&self) -> &[u8] {
        // We can use the original (uncloned) code image for this: it
        // is just a slice of bytes that we interpret independent of
        // location.
        self.original_code.stack_map_data()
    }

    /// Returns the encoded exception-tables section to pass to
    /// `wasmtime_unwinder::ExceptionTable::parse`.
    pub fn exception_tables(&self) -> &[u8] {
        // We can use the original (uncloned) code image for this: it
        // is just a slice of bytes that we interpret independent of
        // location.
        self.original_code.exception_tables()
    }

    /// Returns the encoded frame-tables section to pass to
    /// `wasmtime_environ::FrameTable::parse`.
    pub fn frame_tables(&self) -> &[u8] {
        self.original_code.frame_tables()
    }

    /// Returns the data in the `ELF_NAME_DATA` section.
    #[inline]
    pub fn func_name_data(&self) -> &[u8] {
        self.original_code.func_name_data()
    }

    /// Returns the contents of the `ELF_WASMTIME_DWARF` section.
    #[inline]
    pub fn wasm_dwarf(&self) -> &[u8] {
        self.original_code.wasm_dwarf()
    }

    /// Returns the raw image as bytes (in our internal image format).
    pub fn image(&self) -> &[u8] {
        &self.original_code.mmap()[..]
    }

    pub fn text(&self) -> &[u8] {
        &self.original_code.text()
    }

    /// Returns the concatenated list of all data associated with this wasm
    /// module.
    ///
    /// This is used for initialization of memories and all data ranges stored
    /// in a `Module` are relative to the slice returned here.
    #[inline]
    pub fn wasm_data(&self) -> &[u8] {
        self.original_code.wasm_data()
    }

    pub(crate) fn module_memory_image_source(&self) -> &Arc<impl ModuleMemoryImageSource> {
        &self.original_code
    }
}

impl Drop for EngineCode {
    fn drop(&mut self) {
        crate::module::unregister_code(self.original_code.raw_addr_range());
    }
}

pub enum Types {
    Module(ModuleTypes),
    #[cfg(feature = "component-model")]
    Component(Arc<ComponentTypes>),
}

impl Types {
    fn module_types(&self) -> &ModuleTypes {
        match self {
            Types::Module(m) => m,
            #[cfg(feature = "component-model")]
            Types::Component(c) => c.module_types(),
        }
    }
}

impl From<ModuleTypes> for Types {
    fn from(types: ModuleTypes) -> Types {
        Types::Module(types)
    }
}

#[cfg(feature = "component-model")]
impl From<Arc<ComponentTypes>> for Types {
    fn from(types: Arc<ComponentTypes>) -> Types {
        Types::Component(types)
    }
}

/// A `Store`-local instance of code.
///
/// This type encapsulates executable code within the context of
/// instantiation in a single store. It may be an unmodified pointer
/// to the read-only original code, or it may be locally patched for
/// this store due to debugging or instrumentation settings.
///
/// Most things that the runtime will want to do with a module's image
/// will require the `EngineCode` -- all metadata lives there. The
/// `StoreCode` is solely responsible for the executable machine code.
///
/// This type is designed to be uniquely owned, unlike `EngineCode`
/// above. The runtime data structures will have scattered `Arc`s to
/// the `EngineCode` but only one
pub struct StoreCode(StoreCodeStorage);

enum StoreCodeStorage {
    Shared(Arc<CodeMemory>),
    /// Private copy of the given code memory.
    ///
    /// This is the only reference to this CodeMemory. The StoreCode
    /// is owned directly by the Store's ModuleRegistry.
    #[cfg(feature = "debug")]
    Private(Box<CodeMemory>),
}

impl StoreCode {
    /// Create a new StoreCode for a given Store from a given
    /// EngineCode, given the engine's settings (tunables).
    pub fn new(engine: &Engine, engine_code: &Arc<EngineCode>) -> Result<Self> {
        // Enabled guest-debugging causes us to allocate private
        // copies of code in every store, to allow individual enabling
        // of breakpoints (by code patching) independently in each
        // one.
        #[cfg(feature = "debug")]
        let code = if engine.tunables().debug_guest {
            // TODO(#12104): we should be able to clone only `.text`;
            // this clones the whole image.
            let mut private_copy = engine_code.original_code.deep_clone(engine)?;
            private_copy.publish()?;
            crate::module::register_code(&engine_code.original_code, private_copy.raw_addr_range());
            StoreCodeStorage::Private(Box::new(private_copy))
        } else {
            StoreCodeStorage::Shared(engine_code.original_code.clone())
        };

        #[cfg(not(feature = "debug"))]
        let code = StoreCodeStorage::Shared(engine_code.original_code.clone());
        // Avoid unused-variable warning in build without debugging
        // support.
        let _ = engine;

        Ok(StoreCode(code))
    }

    /// Provide the underlying CodeMemory.
    pub fn code_memory(&self) -> &CodeMemory {
        match &self.0 {
            StoreCodeStorage::Shared(m) => m,
            #[cfg(feature = "debug")]
            StoreCodeStorage::Private(m) => m,
        }
    }

    /// Provide a mutable reference to a CodeMemory that is privately
    /// owned only by this StoreCode.
    #[cfg(feature = "debug")]
    pub fn code_memory_mut(&mut self) -> Option<&mut CodeMemory> {
        match &mut self.0 {
            StoreCodeStorage::Shared(_) => None,
            StoreCodeStorage::Private(m) => Some(m),
        }
    }

    /// Provide the address range for this StoreCode.
    pub fn text_range(&self) -> Range<StoreCodePC> {
        let raw = self.code_memory().raw_addr_range();
        StoreCodePC(raw.start)..StoreCodePC(raw.end)
    }

    /// Provide the actual text segment for this StoreCode.
    pub fn text(&self) -> &[u8] {
        self.code_memory().text()
    }
}

impl Drop for StoreCode {
    fn drop(&mut self) {
        match &self.0 {
            StoreCodeStorage::Shared(_) => {
                // Drop impl for EngineCode will de-register (see
                // above).
            }
            #[cfg(feature = "debug")]
            StoreCodeStorage::Private(mem) => {
                crate::module::unregister_code(mem.raw_addr_range());
            }
        }
    }
}

/// A wrapper for a Module together with a StoreCode. Allows fetching
/// code pointers, ready to call.
pub struct ModuleWithCode<'a> {
    module: &'a Module,
    store_code: &'a StoreCode,
}

impl<'a> ModuleWithCode<'a> {
    /// Find the StoreCode in a given store for a module and wrap it
    /// up with that module, ready to compute code pointers.
    pub fn in_store(
        registry: &'a ModuleRegistry,
        module: &'a Module,
    ) -> Option<ModuleWithCode<'a>> {
        let store_code = registry.store_code(module.engine_code())?;
        Some(ModuleWithCode { module, store_code })
    }

    pub(crate) fn from_raw(module: &'a Module, store_code: &'a StoreCode) -> ModuleWithCode<'a> {
        ModuleWithCode { module, store_code }
    }

    /// Provide the Module wrapped in this tuple.
    pub fn module(&self) -> &'a Module {
        self.module
    }

    /// Provide the StoreCode wrapped in this tuple.
    pub fn store_code(&self) -> &'a StoreCode {
        self.store_code
    }

    /// Returns an iterator over all functions defined within this module with
    /// their index and their raw pointer.
    #[inline]
    pub fn finished_functions(
        &self,
    ) -> impl ExactSizeIterator<Item = (DefinedFuncIndex, &[u8])> + '_ {
        self.module
            .env_module()
            .defined_func_indices()
            .map(|i| (i, self.finished_function(i)))
    }

    /// Returns the slice in the text section of the function that
    /// `index` points to.
    #[inline]
    pub fn finished_function(&self, def_func_index: DefinedFuncIndex) -> &[u8] {
        let range = self
            .module
            .compiled_module()
            .finished_function_range(def_func_index);
        &self.store_code.text()[range]
    }

    /// Get the array-to-Wasm trampoline for the function `index`
    /// points to, as a slice of raw code that can be converted to a
    /// callable function pointer.
    ///
    /// If the function `index` points to does not escape, then `None` is
    /// returned.
    ///
    /// These trampolines are used for array callers (e.g. `Func::new`)
    /// calling Wasm callees.
    pub fn array_to_wasm_trampoline(&self, def_func_index: DefinedFuncIndex) -> Option<&[u8]> {
        let range = self
            .module
            .compiled_module()
            .array_to_wasm_trampoline_range(def_func_index)?;
        Some(&self.store_code.text()[range])
    }

    /// Get the text offset (relative PC) for a given absolute PC in
    /// this module.
    #[cfg(feature = "gc")]
    pub(crate) fn text_offset(&self, pc: usize) -> Option<u32> {
        StoreCodePC::offset_of(self.store_code.text_range(), pc)
            .map(|offset| u32::try_from(offset).expect("Module larger than 4GiB"))
    }

    /// Lookup the stack map at a program counter value.
    #[cfg(feature = "gc")]
    pub(crate) fn lookup_stack_map(&self, pc: usize) -> Option<wasmtime_environ::StackMap<'_>> {
        let text_offset = self.text_offset(pc)?;
        let info = self.module.engine_code().stack_map_data();
        wasmtime_environ::StackMap::lookup(text_offset, info)
    }
}
