//! Implements a registry of modules for a store.

use crate::Engine;
use crate::code::CodeObject;
#[cfg(feature = "component-model")]
use crate::component::Component;
use crate::prelude::*;
use crate::runtime::vm::VMWasmCallFunction;
use crate::sync::{OnceLock, RwLock};
use crate::vm::CompiledModuleId;
use crate::{FrameInfo, Module, code_memory::CodeMemory};
use alloc::collections::btree_map::{BTreeMap, Entry};
use alloc::sync::Arc;
use core::ptr::NonNull;
use wasmtime_environ::VMSharedTypeIndex;

/// Used for registering modules with a store.
///
/// Note that the primary reason for this registry is to ensure that everything
/// in `Module` is kept alive for the duration of a `Store`. At this time we
/// need "basically everything" within a `Module` to stay alive once it's
/// instantiated within a store. While there's some smaller portions that could
/// theoretically be omitted as they're not needed by the store they're
/// currently small enough to not worry much about.
#[derive(Default)]
pub struct ModuleRegistry {
    // Keyed by the end address of a `CodeObject`.
    //
    // The value here is the start address and the information about what's
    // loaded at that address.
    loaded_code: BTreeMap<usize, (usize, LoadedCode)>,

    // Preserved for keeping data segments alive or similar
    modules_without_code: Vec<Module>,

    // Private clones of code memory, when the store requires private
    // copies of code for patching (e.g. debugging support).
    private_code: BTreeMap<CompiledModuleId, Arc<CodeObject>>,
}

struct LoadedCode {
    /// Kept alive here in the store to have a strong reference to keep the
    /// relevant code mapped while the store is alive.
    code_object: Arc<CodeObject>,

    /// Modules found within `self.code`, keyed by start address here of the
    /// address of the first function in the module.
    modules: BTreeMap<usize, Module>,

    /// These modules have no functions inside of them but they can still be
    /// used for trampoline lookup.
    modules_with_only_trampolines: Vec<Module>,
}

/// An identifier of a module that has previously been inserted into a
/// `ModuleRegistry`.
#[derive(Clone, Copy, Debug)]
pub enum RegisteredModuleId {
    /// Index into `ModuleRegistry::modules_without_code`.
    WithoutCode(usize),
    /// Start address of the module's code so that we can get it again via
    /// `ModuleRegistry::lookup_module`.
    LoadedCode(usize),
}

impl ModuleRegistry {
    /// Get a previously-registered module by id.
    pub fn lookup_module_by_id(&self, id: RegisteredModuleId) -> Option<&Module> {
        match id {
            RegisteredModuleId::WithoutCode(idx) => self.modules_without_code.get(idx),
            RegisteredModuleId::LoadedCode(pc) => {
                let (module, _, _) = self.module_and_offset(pc)?;
                Some(module)
            }
        }
    }

    /// Fetches a registered module given a program counter value.
    #[cfg(any(feature = "gc", feature = "debug"))]
    pub fn lookup_module_by_pc(&self, pc: usize) -> Option<(&Module, &CodeMemory)> {
        let (module, code_memory, _) = self.module_and_offset(pc)?;
        Some((module, code_memory))
    }

    fn code(&self, pc: usize) -> Option<(&LoadedCode, usize)> {
        let (end, (start, code)) = self.loaded_code.range(pc..).next()?;
        if pc < *start || *end < pc {
            return None;
        }
        Some((code, pc - *start))
    }

    fn module_and_offset(&self, pc: usize) -> Option<(&Module, &CodeMemory, usize)> {
        let (code, offset) = self.code(pc)?;
        Some((code.module(pc)?, code.code_object.code_memory(), offset))
    }

    /// Gets an iterator over all modules in the registry.
    #[cfg(feature = "coredump")]
    pub fn all_modules(&self) -> impl Iterator<Item = &'_ Module> + '_ {
        self.loaded_code
            .values()
            .flat_map(|(_, code)| code.modules.values())
            .chain(self.modules_without_code.iter())
    }

    /// Registers a new module with the registry.
    ///
    /// Ensures that the module's code object is duplicated, if
    /// required by our configuration. Returns the CodeMemory specific
    /// to this store for this module.
    pub fn register_module(
        &mut self,
        module: &Module,
        engine: &Engine,
    ) -> Result<(RegisteredModuleId, Arc<CodeMemory>)> {
        Ok(self
            .register(module.id(), module.code_object(), engine, Some(module))?
            .unwrap())
    }

    #[cfg(feature = "component-model")]
    pub fn register_component(&mut self, component: &Component, engine: &Engine) -> Result<()> {
        self.register(component.id(), component.code_object(), engine, None)?;
        Ok(())
    }

    /// Registers a new module with the registry.
    fn register(
        &mut self,
        compiled_module_id: CompiledModuleId,
        code_object: &Arc<CodeObject>,
        engine: &Engine,
        module: Option<&Module>,
    ) -> Result<Option<(RegisteredModuleId, Arc<CodeMemory>)>> {
        // If there's not actually any functions in this module then we may
        // still need to preserve it for its data segments. Instances of this
        // module will hold a pointer to the data stored in the module itself,
        // and for schemes that perform lazy initialization which could use the
        // module in the future. For that reason we continue to register empty
        // modules and retain them.
        if code_object.code_memory().text().is_empty() {
            return Ok(module.map(|module| {
                let id = RegisteredModuleId::WithoutCode(self.modules_without_code.len());
                self.modules_without_code.push(module.clone());
                (id, code_object.code_memory().clone())
            }));
        }

        // Get the code memory object, cloning if needed.
        let code_object = if engine.tunables().debug_guest {
            match self.private_code.entry(compiled_module_id) {
                Entry::Occupied(o) => o.into_mut(),
                Entry::Vacant(v) => {
                    let mut memory = code_object.code_memory().deep_clone(engine)?;
                    memory.publish()?;
                    let object = code_object.with_private_code(Arc::new(memory));
                    v.insert(Arc::new(object))
                }
            }
        } else {
            code_object
        };

        // The module code range is exclusive for end, so make it inclusive as
        // it may be a valid PC value
        let text = code_object.code_memory().text();
        let start_addr = text.as_ptr() as usize;
        let end_addr = start_addr + text.len() - 1;
        let id = module.map(|_| {
            (
                RegisteredModuleId::LoadedCode(start_addr),
                code_object.code_memory().clone(),
            )
        });

        // If this module is already present in the registry then that means
        // it's either an overlapping image, for example for two modules
        // found within a component, or it's a second instantiation of the same
        // module. Delegate to `push_module` to find out.
        if let Some((other_start, prev)) = self.loaded_code.get_mut(&end_addr) {
            assert_eq!(*other_start, start_addr);
            if let Some(module) = module {
                prev.push_module(module, code_object.code_memory());
            }
            return Ok(id);
        }

        // Assert that this module's code doesn't collide with any other
        // registered modules
        if let Some((_, (prev_start, _))) = self.loaded_code.range(start_addr..).next() {
            assert!(*prev_start > end_addr);
        }
        if let Some((prev_end, _)) = self.loaded_code.range(..=start_addr).next_back() {
            assert!(*prev_end < start_addr);
        }

        let mut item = LoadedCode {
            code_object: code_object.clone(),
            modules: Default::default(),
            modules_with_only_trampolines: Vec::new(),
        };
        if let Some(module) = module {
            item.push_module(module, code_object.code_memory());
        }
        let prev = self.loaded_code.insert(end_addr, (start_addr, item));
        assert!(prev.is_none());
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
    pub(crate) fn lookup_frame_info(&self, pc: usize) -> Option<(FrameInfo, &Module)> {
        let (module, _code_memory, offset) = self.module_and_offset(pc)?;
        let info = FrameInfo::new(module.clone(), offset)?;
        Some((info, module))
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
        for (_, code) in self.loaded_code.values() {
            for module in code
                .modules
                .values()
                .chain(&code.modules_with_only_trampolines)
            {
                if let Some(trampoline) =
                    module.wasm_to_array_trampoline(sig, code.code_object.code_memory())
                {
                    return Some(trampoline);
                }
            }
        }
        None
    }
}

impl LoadedCode {
    fn push_module(&mut self, module: &Module, code_memory: &CodeMemory) {
        let func = match module
            .compiled_module()
            .finished_functions(code_memory)
            .next()
        {
            Some((_, func)) => func,
            // There are no compiled functions in this module so there's no
            // need to push onto `self.modules` which is only used for frame
            // information lookup for a trap which only symbolicates defined
            // functions.
            None => {
                self.modules_with_only_trampolines.push(module.clone());
                return;
            }
        };
        let start = func.as_ptr() as usize;

        match self.modules.entry(start) {
            // This module is already present, and it should be the same as
            // `module`.
            Entry::Occupied(m) => {
                debug_assert!(Arc::ptr_eq(&module.inner, &m.get().inner));
            }
            // This module was not already present, so now it's time to insert.
            Entry::Vacant(v) => {
                v.insert(module.clone());
            }
        }
    }

    fn module(&self, pc: usize) -> Option<&Module> {
        // The `modules` map is keyed on the start address of the first
        // function in the module, so find the first module whose start address
        // is less than the `pc`. That may be the wrong module but lookup
        // within the module should fail in that case.
        let (_start, module) = self.modules.range(..=pc).next_back()?;
        Some(module)
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
pub fn register_code(code: &Arc<CodeMemory>) {
    let text = code.text();
    if text.is_empty() {
        return;
    }
    let start = text.as_ptr() as usize;
    let end = start + text.len() - 1;
    let prev = global_code().write().insert(end, (start, code.clone()));
    assert!(prev.is_none());
}

/// Unregisters a code mmap from the global map.
///
/// Must have been previously registered with `register`.
pub fn unregister_code(code: &Arc<CodeMemory>) {
    let text = code.text();
    if text.is_empty() {
        return;
    }
    let end = (text.as_ptr() as usize) + text.len() - 1;
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

    for (i, alloc) in module
        .compiled_module()
        .finished_functions(module.compiled_module().default_code_memory())
    {
        let (start, end) = {
            let ptr = alloc.as_ptr();
            let len = alloc.len();
            (ptr as usize, ptr as usize + len)
        };
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
