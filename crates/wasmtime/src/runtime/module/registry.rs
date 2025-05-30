//! Implements a registry of modules for a store.

use crate::code::CodeObject;
#[cfg(feature = "component-model")]
use crate::component::Component;
use crate::prelude::*;
use crate::runtime::vm::VMWasmCallFunction;
use crate::sync::{OnceLock, RwLock};
use crate::{FrameInfo, Module, code_memory::CodeMemory};
use alloc::collections::btree_map::{BTreeMap, Entry};
use alloc::sync::Arc;
use core::ops::Range;
use core::ptr::NonNull;
use wasmtime_environ::{Trap, VMSharedTypeIndex, lookup_trap_code};

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
}

struct LoadedCode {
    /// Kept alive here in the store to have a strong reference to keep the
    /// relevant code mapped while the store is alive.
    _code: Arc<CodeObject>,

    /// Modules found within `self.code`, keyed by start address here of the
    /// address of the first function in the module.
    modules: BTreeMap<usize, Module>,
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
                let (module, _) = self.module_and_offset(pc)?;
                Some(module)
            }
        }
    }

    /// Fetches a registered module given a program counter value.
    #[cfg(feature = "gc")]
    pub fn lookup_module_by_pc(&self, pc: usize) -> Option<&Module> {
        let (module, _) = self.module_and_offset(pc)?;
        Some(module)
    }

    fn code(&self, pc: usize) -> Option<(&LoadedCode, usize)> {
        let (end, (start, code)) = self.loaded_code.range(pc..).next()?;
        if pc < *start || *end < pc {
            return None;
        }
        Some((code, pc - *start))
    }

    fn module_and_offset(&self, pc: usize) -> Option<(&Module, usize)> {
        let (code, offset) = self.code(pc)?;
        Some((code.module(pc)?, offset))
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
    pub fn register_module(&mut self, module: &Module) -> RegisteredModuleId {
        self.register(module.code_object(), Some(module)).unwrap()
    }

    #[cfg(feature = "component-model")]
    pub fn register_component(&mut self, component: &Component) {
        self.register(component.code_object(), None);
    }

    /// Registers a new module with the registry.
    fn register(
        &mut self,
        code: &Arc<CodeObject>,
        module: Option<&Module>,
    ) -> Option<RegisteredModuleId> {
        let text = code.code_memory().text();

        // If there's not actually any functions in this module then we may
        // still need to preserve it for its data segments. Instances of this
        // module will hold a pointer to the data stored in the module itself,
        // and for schemes that perform lazy initialization which could use the
        // module in the future. For that reason we continue to register empty
        // modules and retain them.
        if text.is_empty() {
            return module.map(|module| {
                let id = RegisteredModuleId::WithoutCode(self.modules_without_code.len());
                self.modules_without_code.push(module.clone());
                id
            });
        }

        // The module code range is exclusive for end, so make it inclusive as
        // it may be a valid PC value
        let start_addr = text.as_ptr() as usize;
        let end_addr = start_addr + text.len() - 1;
        let id = module.map(|_| RegisteredModuleId::LoadedCode(start_addr));

        // If this module is already present in the registry then that means
        // it's either an overlapping image, for example for two modules
        // found within a component, or it's a second instantiation of the same
        // module. Delegate to `push_module` to find out.
        if let Some((other_start, prev)) = self.loaded_code.get_mut(&end_addr) {
            assert_eq!(*other_start, start_addr);
            if let Some(module) = module {
                prev.push_module(module);
            }
            return id;
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
            _code: code.clone(),
            modules: Default::default(),
        };
        if let Some(module) = module {
            item.push_module(module);
        }
        let prev = self.loaded_code.insert(end_addr, (start_addr, item));
        assert!(prev.is_none());
        id
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
        let (module, offset) = self.module_and_offset(pc)?;
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
            for module in code.modules.values() {
                if let Some(trampoline) = module.wasm_to_array_trampoline(sig) {
                    return Some(trampoline);
                }
            }
        }
        None
    }
}

impl LoadedCode {
    fn push_module(&mut self, module: &Module) {
        let func = match module.compiled_module().finished_functions().next() {
            Some((_, func)) => func,
            // There are no compiled functions in this module so there's no
            // need to push onto `self.modules` which is only used for frame
            // information lookup for a trap which only symbolicates defined
            // functions.
            None => return,
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

// This is the global trap data registry that stores trap section pointer
// for all loaded code objects that are currently in use by any
// `Store` in the current process.
//
// The purpose of this map is to be called from signal handlers to determine
// whether a program counter is a wasm trap or not. Specifically macOS has
// no contextual information about the thread available, hence the necessity
// for global state rather than using thread local state.
//
// This is similar to `ModuleRegistry` except that it stores only trap data and
// supports removal. Any time anything is registered with a `ModuleRegistry`
// it is also automatically registered with the singleton global module
// registry. When a `ModuleRegistry` is destroyed then all of its entries
// are removed from the global registry.
fn global_trap_registry() -> &'static RwLock<GlobalTrapRegistry> {
    static GLOBAL_TRAP_REGISTRY: OnceLock<RwLock<GlobalTrapRegistry>> = OnceLock::new();
    GLOBAL_TRAP_REGISTRY.get_or_init(Default::default)
}

type GlobalTrapRegistry = BTreeMap<usize, (usize, i32, Range<usize>)>;

/// Find which registered region of code contains the given program counter, and
/// lookup trap for given PC offset within that module code.
pub fn lookup_trap_for_pc(pc: usize) -> Option<Trap> {
    let all_modules = global_trap_registry().read();
    let (_end, (start, _count, trap_range)) = all_modules.range(pc..).next()?;
    let trap_data: &[u8];
    unsafe {
        // GlobalTrapRegistryHandle ensures that CodeMemory is not dropped
        // before unregistration and we're holding registry RwLock here.
        trap_data = std::slice::from_raw_parts(
            trap_range.start as *const u8,
            trap_range.end - trap_range.start,
        );
    }
    let text_offset = pc.checked_sub(*start)?;
    lookup_trap_code(trap_data, text_offset)
}

pub struct GlobalTrapRegistryHandle {
    code: Arc<CodeMemory>,
}

impl GlobalTrapRegistryHandle {
    /// Registers a new region of code.
    ///
    /// Multiple `CodeMemory` pointing to the same code might be registered
    /// if trap_data pointers are also equal.
    ///
    /// Returns handle that automatically unregisters this region when dropped.
    ///
    /// This is required to enable traps to work correctly since the signal handler
    /// will lookup in the `GLOBAL_CODE` list to determine which a particular pc
    /// is a trap or not.
    pub fn register_code(code: Arc<CodeMemory>) -> Self {
        let text = code.text();
        if !text.is_empty() {
            let start = text.as_ptr() as usize;
            let end = start + text.len() - 1;
            let trap_data = code.trap_data().as_ptr_range();
            let trap_range = Range {
                start: trap_data.start as usize,
                end: trap_data.end as usize,
            };

            let mut locked = global_trap_registry().write();
            let prev = locked.get_mut(&end);
            if let Some(prev) = prev {
                // Assert that trap_range is equal to previously added entry.
                assert_eq!(trap_range, prev.2);

                // Increment usage count.
                prev.1 += 1;
            } else {
                locked.insert(end, (start, 1, trap_range));
            }
        }

        Self { code }
    }
}

impl Drop for GlobalTrapRegistryHandle {
    fn drop(&mut self) {
        let text = self.code.text();
        if !text.is_empty() {
            let end = (text.as_ptr() as usize) + text.len() - 1;

            let mut locked = global_trap_registry().write();
            let prev = locked.get_mut(&end).unwrap();

            // Decrement usage count and remove if needed.
            prev.1 -= 1;
            if prev.1 == 0 {
                locked.remove(&end);
            }
        }
    }
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

    for (i, alloc) in module.compiled_module().finished_functions() {
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
