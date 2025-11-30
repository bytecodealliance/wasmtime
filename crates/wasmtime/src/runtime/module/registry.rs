//! Implements a registry of modules for a store.

use crate::code::{EngineCode, EngineCodePC, ModuleWithCode, StoreCode, StoreCodePC};
#[cfg(feature = "component-model")]
use crate::component::Component;
use crate::runtime::vm::VMWasmCallFunction;
use crate::sync::{OnceLock, RwLock};
use crate::vm::CompiledModuleId;
use crate::{Engine, prelude::*};
use crate::{FrameInfo, Module, code_memory::CodeMemory};
use alloc::collections::btree_map::{BTreeMap, Entry};
use alloc::sync::Arc;
use core::ops::Range;
use core::ptr::NonNull;
use wasmtime_environ::VMSharedTypeIndex;

/// Used for registering modules with a store.
///
/// There are two basic purposes that this registry serves:
///
/// - It keeps all modules and their metadata alive as long as the
///   store exists.
/// - It owns the [`StoreCode`], i.e. possibly-private-copy of machine
///   code, for all modules that execute in this store.
///
/// The registry allows for translation of EngineCode to StoreCode,
/// deduplicating by the start address of the EngineCode; and allows
/// for looking up modules by "registered module ID", and looking up
/// StoreCode and Modules by PC.
///
/// Note that multiple modules may be backed by a single
/// `StoreCode`. This is specifically the case for components in
/// general. When a component is first instantiated, the component
/// itself is registered (which loads the StoreCode into the
/// registry), then each individual module within that component is
/// registered and added to the data structures.
///
/// A brief overview of the kinds of compiled object and their
/// relationships:
///
/// - `Module` is a Wasm module. It owns a `CompiledModule`.
/// - `CompiledModule` contains metadata about the module (e.g., a map
///   from Wasm function indices to locations in the machine code),
///   and also owns an `EngineCode`.
/// - `EngineCode` holds an `Arc` to a `CodeMemory` with the canonical
///   copy of machine code, as well as some lower-level metadata
///   (signatures and types). It is instantiated by this registry into
///   `StoreCode`.
/// - `StoreCode` owns either another `Arc` to the same `CodeMemory`
///   as `EngineCode`, or if guest debugging is enabled and causes us
///   to clone private copies of code for patching per store, owns its
///   own private `CodeMemory` at a different address.
/// - Instances hold a `RegisteredModuleId` to be able to look up their modules.
#[derive(Default)]
pub struct ModuleRegistry {
    /// StoreCode and Modules associated with it.
    ///
    /// Keyed by the start address of the `StoreCode`. We maintain the
    /// invariant of no overlaps on insertion. We use a range query to
    /// find the StoreCode for a given PC: take the range `0..=pc`,
    /// then take the last element of the range. That picks the
    /// highest start address <= the query, and we can check whether
    /// it contains the address.
    loaded_code: BTreeMap<StoreCodePC, LoadedCode>,

    /// Map from EngineCodePC start to StoreCodePC start. We use this
    /// to memoize the store-code creation process: each EngineCode is
    /// instantiated to a StoreCode only once per store.
    store_code: BTreeMap<EngineCodePC, StoreCodePC>,

    /// Modules instantiated in this registry.
    ///
    /// Every module is placed in this map, but not every module will
    /// be in a LoadedCode entry, because the module may have no text.
    modules: BTreeMap<RegisteredModuleId, Module>,
}

struct LoadedCode {
    /// The StoreCode in this range.
    code: StoreCode,

    /// Map by starting text offset of Modules in this code region.
    modules: BTreeMap<usize, RegisteredModuleId>,
}

/// An identifier of a module that has previously been inserted into a
/// `ModuleRegistry`.
///
/// This is just a newtype around `CompiledModuleId`, which is unique
/// within the Engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RegisteredModuleId(CompiledModuleId);

fn assert_no_overlap(loaded_code: &BTreeMap<StoreCodePC, LoadedCode>, range: Range<StoreCodePC>) {
    if let Some((start, _)) = loaded_code.range(range.start..).next() {
        assert!(*start >= range.end);
    }
    if let Some((_, code)) = loaded_code.range(..range.end).next_back() {
        assert!(code.code.text_range().end <= range.start);
    }
}

impl ModuleRegistry {
    /// Get a previously-registered module by id.
    pub fn module_by_id(&self, id: RegisteredModuleId) -> Option<&Module> {
        self.modules.get(&id)
    }

    /// Fetches a registered StoreCode and module and an offset within
    /// it given a program counter value.
    pub fn module_and_code_by_pc<'a>(&'a self, pc: usize) -> Option<(ModuleWithCode<'a>, usize)> {
        let (_, code) = self
            .loaded_code
            .range(..=StoreCodePC::from_raw(pc))
            .next_back()?;
        let offset = StoreCodePC::offset_of(code.code.text_range(), pc)?;
        let (_, module_id) = code.modules.range(..=offset).next_back()?;
        let module = self.modules.get(&module_id)?;
        Some((ModuleWithCode::from_raw(module, &code.code), offset))
    }

    /// Fetches the `StoreCode` for a given `EngineCode`.
    pub fn store_code(&self, engine_code: &EngineCode) -> Option<&StoreCode> {
        let store_code_pc = *self.store_code.get(&engine_code.text_range().start)?;
        let (_, code) = self.loaded_code.range(store_code_pc..).next()?;
        Some(&code.code)
    }

    /// Gets an iterator over all modules in the registry.
    #[cfg(feature = "coredump")]
    pub fn all_modules(&self) -> impl Iterator<Item = &'_ Module> + '_ {
        self.modules.values()
    }

    /// Registers a new module with the registry.
    pub fn register_module(
        &mut self,
        module: &Module,
        engine: &Engine,
    ) -> Result<RegisteredModuleId> {
        self.register(module.id(), module.engine_code(), Some(module), engine)
            .map(|id| id.unwrap())
    }

    #[cfg(feature = "component-model")]
    pub fn register_component(&mut self, component: &Component, engine: &Engine) -> Result<()> {
        self.register(component.id(), component.engine_code(), None, engine)?;
        Ok(())
    }

    /// Registers a new module with the registry.
    fn register(
        &mut self,
        compiled_id: CompiledModuleId,
        code: &Arc<EngineCode>,
        module: Option<&Module>,
        engine: &Engine,
    ) -> Result<Option<RegisteredModuleId>> {
        // Register the module, if any.
        let id = module.map(|module| {
            let id = RegisteredModuleId(compiled_id);
            self.modules.entry(id).or_insert_with(|| module.clone());
            id
        });

        // Create a StoreCode if one does not already exist.
        let store_code_pc = match self.store_code.entry(code.text_range().start) {
            Entry::Vacant(v) => {
                let store_code = StoreCode::new(engine, code)?;
                let store_code_pc = store_code.text_range().start;
                assert_no_overlap(&self.loaded_code, store_code.text_range());
                self.loaded_code.insert(
                    store_code_pc,
                    LoadedCode {
                        code: store_code,
                        modules: BTreeMap::default(),
                    },
                );
                *v.insert(store_code_pc)
            }
            Entry::Occupied(o) => *o.get(),
        };

        // Add this module to the LoadedCode if not present.
        if let (Some(module), Some(id)) = (module, id) {
            if let Some((_, range)) = module.compiled_module().finished_function_ranges().next() {
                let loaded_code = self
                    .loaded_code
                    .get_mut(&store_code_pc)
                    .expect("loaded_code must have entry for StoreCodePC");
                loaded_code.modules.insert(range.start, id);
            }
        }

        Ok(id)
    }

    /// Fetches frame information about a program counter in a backtrace.
    ///
    /// Returns an object if this `pc` is known to some previously registered
    /// module, or returns `None` if no information can be found. The first
    /// boolean returned indicates whether the original module has unparsed
    /// debug information due to the compiler's configuration. The second
    /// boolean indicates whether the engine used to compile this module is
    /// using environment variables to control debuginfo parsing.
    pub(crate) fn lookup_frame_info<'a>(
        &'a self,
        pc: usize,
    ) -> Option<(FrameInfo, ModuleWithCode<'a>)> {
        let (_, code) = self
            .loaded_code
            .range(..=StoreCodePC::from_raw(pc))
            .next_back()?;
        let text_offset = StoreCodePC::offset_of(code.code.text_range(), pc)?;
        let (_, module_id) = code.modules.range(..=text_offset).next_back()?;
        let module = self
            .modules
            .get(&module_id)
            .expect("referenced module ID not found");
        let info = FrameInfo::new(module.clone(), text_offset)?;
        let module_with_code = ModuleWithCode::from_raw(module, &code.code);
        Some((info, module_with_code))
    }

    pub fn wasm_to_array_trampoline(
        &self,
        sig: VMSharedTypeIndex,
    ) -> Option<NonNull<VMWasmCallFunction>> {
        // TODO: We are doing a linear search over each module. This is fine for
        // now because we typically have very few modules per store (almost
        // always one, in fact). If this linear search ever becomes a
        // bottleneck, we could avoid it by incrementally and lazily building a
        // `VMSharedSignatureIndex` to `SignatureIndex` map.
        //
        // See also the comment in `ModuleInner::wasm_to_native_trampoline`.
        for module in self.modules.values() {
            if let Some(trampoline) = module.wasm_to_array_trampoline(sig) {
                return Some(trampoline);
            }
        }
        None
    }
}

// This is the global code registry that stores information for all loaded code
// objects that are currently in use by any `Store` in the current process.
//
// The purpose of this map is to be called from signal handlers to determine
// whether a program counter is a wasm trap or not. Specifically macOS has
// no contextual information about the thread available, hence the necessity
// for global state rather than using thread local state.
//
// This is similar to `ModuleRegistry` except that it has less information and
// supports removal. Any time anything is registered with a `ModuleRegistry`
// it is also automatically registered with the singleton global module
// registry. When a `ModuleRegistry` is destroyed then all of its entries
// are removed from the global registry.
fn global_code() -> &'static RwLock<GlobalRegistry> {
    static GLOBAL_CODE: OnceLock<RwLock<GlobalRegistry>> = OnceLock::new();
    GLOBAL_CODE.get_or_init(Default::default)
}

type GlobalRegistry = BTreeMap<usize, (usize, Arc<CodeMemory>)>;

/// Find which registered region of code contains the given program counter, and
/// what offset that PC is within that module's code.
pub fn lookup_code(pc: usize) -> Option<(Arc<CodeMemory>, usize)> {
    let all_modules = global_code().read();
    let (_end, (start, module)) = all_modules.range(pc..).next()?;
    let text_offset = pc.checked_sub(*start)?;
    Some((module.clone(), text_offset))
}

/// Registers a new region of code.
///
/// Must not have been previously registered and must be `unregister`'d to
/// prevent leaking memory.
///
/// This is required to enable traps to work correctly since the signal handler
/// will lookup in the `GLOBAL_CODE` list to determine which a particular pc
/// is a trap or not.
pub fn register_code(image: &Arc<CodeMemory>, address: Range<usize>) {
    if address.is_empty() {
        return;
    }
    let start = address.start;
    let end = address.end - 1;
    let prev = global_code().write().insert(end, (start, image.clone()));
    assert!(prev.is_none());
}

/// Unregisters a code mmap from the global map.
///
/// Must have been previously registered with `register`.
pub fn unregister_code(address: Range<usize>) {
    if address.is_empty() {
        return;
    }
    let end = address.end - 1;
    let code = global_code().write().remove(&end);
    assert!(code.is_some());
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_frame_info() -> Result<(), anyhow::Error> {
    use crate::*;

    let mut store = Store::<()>::default();
    let module = Module::new(
        store.engine(),
        r#"
            (module
                (func (export "add") (param $x i32) (param $y i32) (result i32) (i32.add (local.get $x) (local.get $y)))
                (func (export "sub") (param $x i32) (param $y i32) (result i32) (i32.sub (local.get $x) (local.get $y)))
                (func (export "mul") (param $x i32) (param $y i32) (result i32) (i32.mul (local.get $x) (local.get $y)))
                (func (export "div_s") (param $x i32) (param $y i32) (result i32) (i32.div_s (local.get $x) (local.get $y)))
                (func (export "div_u") (param $x i32) (param $y i32) (result i32) (i32.div_u (local.get $x) (local.get $y)))
                (func (export "rem_s") (param $x i32) (param $y i32) (result i32) (i32.rem_s (local.get $x) (local.get $y)))
                (func (export "rem_u") (param $x i32) (param $y i32) (result i32) (i32.rem_u (local.get $x) (local.get $y)))
            )
         "#,
    )?;
    // Create an instance to ensure the frame information is registered.
    Instance::new(&mut store, &module, &[])?;

    // Look for frame info for each function. Assume that StoreCode
    // does not actually clone in the default configuration.
    for (i, range) in module.compiled_module().finished_function_ranges() {
        let base = module.engine_code().text_range().start.raw();
        let start = base + range.start;
        let end = base + range.end;
        for pc in start..end {
            let (frame, _) = store
                .as_context()
                .0
                .modules()
                .lookup_frame_info(pc)
                .unwrap();
            assert!(
                frame.func_index() == i.as_u32(),
                "lookup of {:#x} returned {}, expected {}",
                pc,
                frame.func_index(),
                i.as_u32()
            );
        }
    }
    Ok(())
}
