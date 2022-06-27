//! Implements a registry of modules for a store.

#[cfg(feature = "component-model")]
use crate::component::Component;
use crate::{FrameInfo, Module};
use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};
use wasmtime_environ::TrapCode;
use wasmtime_jit::CompiledModule;
use wasmtime_runtime::{ModuleInfo, VMCallerCheckedAnyfunc, VMTrampoline};

/// Used for registering modules with a store.
///
/// Note that the primary reason for this registry is to ensure that everything
/// in `Module` is kept alive for the duration of a `Store`. At this time we
/// need "basically everything" within a `Moudle` to stay alive once it's
/// instantiated within a store. While there's some smaller portions that could
/// theoretically be omitted as they're not needed by the store they're
/// currently small enough to not worry much about.
#[derive(Default)]
pub struct ModuleRegistry {
    // Keyed by the end address of the module's code in memory.
    //
    // The value here is the start address and the module/component it
    // corresponds to.
    modules_with_code: BTreeMap<usize, (usize, ModuleOrComponent)>,

    // Preserved for keeping data segments alive or similar
    modules_without_code: Vec<Module>,
}

enum ModuleOrComponent {
    Module(Module),
    #[cfg(feature = "component-model")]
    Component(Component),
}

fn start(module: &Module) -> usize {
    assert!(!module.compiled_module().code().is_empty());
    module.compiled_module().code().as_ptr() as usize
}

impl ModuleRegistry {
    /// Fetches information about a registered module given a program counter value.
    pub fn lookup_module(&self, pc: usize) -> Option<&dyn ModuleInfo> {
        self.module(pc).map(|(m, _)| m.module_info())
    }

    fn module(&self, pc: usize) -> Option<(&Module, usize)> {
        match self.module_or_component(pc)? {
            (ModuleOrComponent::Module(m), offset) => Some((m, offset)),
            #[cfg(feature = "component-model")]
            (ModuleOrComponent::Component(_), _) => None,
        }
    }

    fn module_or_component(&self, pc: usize) -> Option<(&ModuleOrComponent, usize)> {
        let (end, (start, module)) = self.modules_with_code.range(pc..).next()?;
        if pc < *start || *end < pc {
            return None;
        }
        Some((module, pc - *start))
    }

    /// Registers a new module with the registry.
    pub fn register_module(&mut self, module: &Module) {
        let compiled_module = module.compiled_module();

        // If there's not actually any functions in this module then we may
        // still need to preserve it for its data segments. Instances of this
        // module will hold a pointer to the data stored in the module itself,
        // and for schemes that perform lazy initialization which could use the
        // module in the future. For that reason we continue to register empty
        // modules and retain them.
        if compiled_module.finished_functions().len() == 0 {
            self.modules_without_code.push(module.clone());
        } else {
            // The module code range is exclusive for end, so make it inclusive as it
            // may be a valid PC value
            let start_addr = start(module);
            let end_addr = start_addr + compiled_module.code().len() - 1;
            self.register(
                start_addr,
                end_addr,
                ModuleOrComponent::Module(module.clone()),
            );
        }
    }

    #[cfg(feature = "component-model")]
    pub fn register_component(&mut self, component: &Component) {
        // If there's no text section associated with this component (e.g. no
        // lowered functions) then there's nothing to register, otherwise it's
        // registered along the same lines as modules above.
        //
        // Note that empty components don't need retaining here since it doesn't
        // have data segments like empty modules.
        let text = component.text();
        if text.is_empty() {
            return;
        }
        let start = text.as_ptr() as usize;
        self.register(
            start,
            start + text.len() - 1,
            ModuleOrComponent::Component(component.clone()),
        );
    }

    /// Registers a new module with the registry.
    fn register(&mut self, start_addr: usize, end_addr: usize, item: ModuleOrComponent) {
        // Ensure the module isn't already present in the registry
        // This is expected when a module is instantiated multiple times in the
        // same store
        if let Some((other_start, _)) = self.modules_with_code.get(&end_addr) {
            assert_eq!(*other_start, start_addr);
            return;
        }

        // Assert that this module's code doesn't collide with any other
        // registered modules
        if let Some((_, (prev_start, _))) = self.modules_with_code.range(start_addr..).next() {
            assert!(*prev_start > end_addr);
        }
        if let Some((prev_end, _)) = self.modules_with_code.range(..=start_addr).next_back() {
            assert!(*prev_end < start_addr);
        }

        let prev = self.modules_with_code.insert(end_addr, (start_addr, item));
        assert!(prev.is_none());
    }

    /// Looks up a trampoline from an anyfunc.
    pub fn lookup_trampoline(&self, anyfunc: &VMCallerCheckedAnyfunc) -> Option<VMTrampoline> {
        let signatures = match self
            .module_or_component(anyfunc.func_ptr.as_ptr() as usize)?
            .0
        {
            ModuleOrComponent::Module(m) => m.signatures(),
            #[cfg(feature = "component-model")]
            ModuleOrComponent::Component(c) => c.signatures(),
        };
        signatures.trampoline(anyfunc.type_index)
    }

    /// Fetches trap information about a program counter in a backtrace.
    pub fn lookup_trap_code(&self, pc: usize) -> Option<TrapCode> {
        let (module, offset) = self.module(pc)?;
        wasmtime_environ::lookup_trap_code(module.compiled_module().trap_data(), offset)
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
        let (module, offset) = self.module(pc)?;
        let info = FrameInfo::new(module, offset)?;
        Some((info, module))
    }
}

// This is the global module registry that stores information for all modules
// that are currently in use by any `Store`.
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
// are removed from the global module registry.
lazy_static::lazy_static! {
    static ref GLOBAL_MODULES: RwLock<GlobalModuleRegistry> = Default::default();
}

type GlobalModuleRegistry = BTreeMap<usize, (usize, Arc<CompiledModule>)>;

/// Returns whether the `pc`, according to globally registered information,
/// is a wasm trap or not.
pub fn is_wasm_trap_pc(pc: usize) -> bool {
    let (module, text_offset) = {
        let all_modules = GLOBAL_MODULES.read().unwrap();

        let (end, (start, module)) = match all_modules.range(pc..).next() {
            Some(info) => info,
            None => return false,
        };
        if pc < *start || *end < pc {
            return false;
        }
        (module.clone(), pc - *start)
    };

    wasmtime_environ::lookup_trap_code(module.trap_data(), text_offset).is_some()
}

/// Registers a new region of code.
///
/// Must not have been previously registered and must be `unregister`'d to
/// prevent leaking memory.
///
/// This is required to enable traps to work correctly since the signal handler
/// will lookup in the `GLOBAL_MODULES` list to determine which a particular pc
/// is a trap or not.
pub fn register(module: &Arc<CompiledModule>) {
    let code = module.code();
    if code.is_empty() {
        return;
    }
    let start = code.as_ptr() as usize;
    let end = start + code.len() - 1;
    let prev = GLOBAL_MODULES
        .write()
        .unwrap()
        .insert(end, (start, module.clone()));
    assert!(prev.is_none());
}

/// Unregisters a module from the global map.
///
/// Must hae been previously registered with `register`.
pub fn unregister(module: &Arc<CompiledModule>) {
    let code = module.code();
    if code.is_empty() {
        return;
    }
    let end = (code.as_ptr() as usize) + code.len() - 1;
    let module = GLOBAL_MODULES.write().unwrap().remove(&end);
    assert!(module.is_some());
}

#[test]
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
        let (start, end) = unsafe {
            let ptr = (*alloc).as_ptr();
            let len = (*alloc).len();
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
