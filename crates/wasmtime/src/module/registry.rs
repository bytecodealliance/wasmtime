//! Implements a registry of modules for a store.

#[cfg(feature = "component-model")]
use crate::component::Component;
use crate::{Engine, Module};
use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};
use wasmtime_environ::{EntityRef, FilePos, TrapCode};
use wasmtime_jit::CompiledModule;
use wasmtime_runtime::{ModuleInfo, VMCallerCheckedAnyfunc, VMTrampoline};

lazy_static::lazy_static! {
    static ref GLOBAL_MODULES: RwLock<GlobalModuleRegistry> = Default::default();
}

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
        self.module(pc).map(|m| m.module_info())
    }

    fn module(&self, pc: usize) -> Option<&Module> {
        match self.module_or_component(pc)? {
            ModuleOrComponent::Module(m) => Some(m),
            #[cfg(feature = "component-model")]
            ModuleOrComponent::Component(_) => None,
        }
    }

    fn module_or_component(&self, pc: usize) -> Option<&ModuleOrComponent> {
        let (end, (start, module)) = self.modules_with_code.range(pc..).next()?;
        if pc < *start || *end < pc {
            return None;
        }
        Some(module)
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
        let signatures = match self.module_or_component(anyfunc.func_ptr.as_ptr() as usize)? {
            ModuleOrComponent::Module(m) => m.signatures(),
            #[cfg(feature = "component-model")]
            ModuleOrComponent::Component(c) => c.signatures(),
        };
        signatures.trampoline(anyfunc.type_index)
    }
}

// Counterpart to `RegisteredModule`, but stored in the global registry.
struct GlobalRegisteredModule {
    start: usize,
    module: Arc<CompiledModule>,
    wasm_backtrace_details_env_used: bool,
}

/// This is the global module registry that stores information for all modules
/// that are currently in use by any `Store`.
///
/// The purpose of this map is to be called from signal handlers to determine
/// whether a program counter is a wasm trap or not. Specifically macOS has
/// no contextual information about the thread available, hence the necessity
/// for global state rather than using thread local state.
///
/// This is similar to `ModuleRegistry` except that it has less information and
/// supports removal. Any time anything is registered with a `ModuleRegistry`
/// it is also automatically registered with the singleton global module
/// registry. When a `ModuleRegistry` is destroyed then all of its entries
/// are removed from the global module registry.
#[derive(Default)]
pub struct GlobalModuleRegistry(BTreeMap<usize, GlobalRegisteredModule>);

impl GlobalModuleRegistry {
    /// Returns whether the `pc`, according to globally registered information,
    /// is a wasm trap or not.
    pub(crate) fn is_wasm_trap_pc(pc: usize) -> bool {
        let (module, text_offset) = match GLOBAL_MODULES.read().unwrap().module(pc) {
            Some((module, offset)) => (module.module.clone(), offset),
            None => return false,
        };

        wasmtime_environ::lookup_trap_code(module.trap_data(), text_offset).is_some()
    }

    /// Returns, if found, the corresponding module for the `pc` as well as the
    /// pc transformed to a relative offset within the text section.
    fn module(&self, pc: usize) -> Option<(&GlobalRegisteredModule, usize)> {
        let (end, info) = self.0.range(pc..).next()?;
        if pc < info.start || *end < pc {
            return None;
        }
        Some((info, pc - info.start))
    }

    // Work with the global instance of `GlobalModuleRegistry`. Note that only
    // shared access is allowed, this isn't intended to mutate the contents.
    pub(crate) fn with<R>(f: impl FnOnce(&GlobalModuleRegistry) -> R) -> R {
        f(&GLOBAL_MODULES.read().unwrap())
    }

    /// Fetches frame information about a program counter in a backtrace.
    ///
    /// Returns an object if this `pc` is known to some previously registered
    /// module, or returns `None` if no information can be found. The first
    /// boolean returned indicates whether the original module has unparsed
    /// debug information due to the compiler's configuration. The second
    /// boolean indicates whether the engine used to compile this module is
    /// using environment variables to control debuginfo parsing.
    pub(crate) fn lookup_frame_info(&self, pc: usize) -> Option<(FrameInfo, bool, bool)> {
        let (module, offset) = self.module(pc)?;
        module.lookup_frame_info(offset).map(|info| {
            (
                info,
                module.has_unparsed_debuginfo(),
                module.wasm_backtrace_details_env_used,
            )
        })
    }

    /// Fetches trap information about a program counter in a backtrace.
    pub(crate) fn lookup_trap_code(&self, pc: usize) -> Option<TrapCode> {
        let (module, offset) = self.module(pc)?;
        wasmtime_environ::lookup_trap_code(module.module.trap_data(), offset)
    }
}

/// Registers a new region of code.
///
/// Must not have been previously registered and must be `unregister`'d to
/// prevent leaking memory.
///
/// This is required to enable traps to work correctly since the signal handler
/// will lookup in the `GLOBAL_MODULES` list to determine which a particular pc
/// is a trap or not.
pub fn register(engine: &Engine, module: &Arc<CompiledModule>) {
    let code = module.code();
    if code.is_empty() {
        return;
    }
    let start = code.as_ptr() as usize;
    let end = start + code.len() - 1;
    let module = GlobalRegisteredModule {
        start,
        wasm_backtrace_details_env_used: engine.config().wasm_backtrace_details_env_used,
        module: module.clone(),
    };
    let prev = GLOBAL_MODULES.write().unwrap().0.insert(end, module);
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
    let module = GLOBAL_MODULES.write().unwrap().0.remove(&end);
    assert!(module.is_some());
}

impl GlobalRegisteredModule {
    /// Determines if the related module has unparsed debug information.
    pub fn has_unparsed_debuginfo(&self) -> bool {
        self.module.has_unparsed_debuginfo()
    }

    /// Fetches frame information about a program counter in a backtrace.
    ///
    /// Returns an object if this `pc` is known to this module, or returns `None`
    /// if no information can be found.
    pub fn lookup_frame_info(&self, text_offset: usize) -> Option<FrameInfo> {
        let (index, _func_offset) = self.module.func_by_text_offset(text_offset)?;
        let info = self.module.func_info(index);
        let instr = wasmtime_environ::lookup_file_pos(self.module.address_map_data(), text_offset);

        // In debug mode for now assert that we found a mapping for `pc` within
        // the function, because otherwise something is buggy along the way and
        // not accounting for all the instructions. This isn't super critical
        // though so we can omit this check in release mode.
        //
        // Note that if the module doesn't even have an address map due to
        // compilation settings then it's expected that `instr` is `None`.
        debug_assert!(
            instr.is_some() || !self.module.has_address_map(),
            "failed to find instruction for {:#x}",
            text_offset
        );

        // Use our wasm-relative pc to symbolize this frame. If there's a
        // symbolication context (dwarf debug info) available then we can try to
        // look this up there.
        //
        // Note that dwarf pcs are code-section-relative, hence the subtraction
        // from the location of `instr`. Also note that all errors are ignored
        // here for now since technically wasm modules can always have any
        // custom section contents.
        let mut symbols = Vec::new();

        if let Some(s) = &self.module.symbolize_context().ok().and_then(|c| c) {
            if let Some(offset) = instr.and_then(|i| i.file_offset()) {
                let to_lookup = u64::from(offset) - s.code_section_offset();
                if let Ok(mut frames) = s.addr2line().find_frames(to_lookup) {
                    while let Ok(Some(frame)) = frames.next() {
                        symbols.push(FrameSymbol {
                            name: frame
                                .function
                                .as_ref()
                                .and_then(|l| l.raw_name().ok())
                                .map(|s| s.to_string()),
                            file: frame
                                .location
                                .as_ref()
                                .and_then(|l| l.file)
                                .map(|s| s.to_string()),
                            line: frame.location.as_ref().and_then(|l| l.line),
                            column: frame.location.as_ref().and_then(|l| l.column),
                        });
                    }
                }
            }
        }

        let module = self.module.module();
        let index = module.func_index(index);

        Some(FrameInfo {
            module_name: module.name.clone(),
            func_index: index.index() as u32,
            func_name: self.module.func_name(index).map(|s| s.to_string()),
            instr,
            func_start: info.start_srcloc,
            symbols,
        })
    }
}

/// Description of a frame in a backtrace for a [`Trap`].
///
/// Whenever a WebAssembly trap occurs an instance of [`Trap`] is created. Each
/// [`Trap`] has a backtrace of the WebAssembly frames that led to the trap, and
/// each frame is described by this structure.
///
/// [`Trap`]: crate::Trap
#[derive(Debug)]
pub struct FrameInfo {
    module_name: Option<String>,
    func_index: u32,
    func_name: Option<String>,
    func_start: FilePos,
    instr: Option<FilePos>,
    symbols: Vec<FrameSymbol>,
}

impl FrameInfo {
    /// Returns the WebAssembly function index for this frame.
    ///
    /// This function index is the index in the function index space of the
    /// WebAssembly module that this frame comes from.
    pub fn func_index(&self) -> u32 {
        self.func_index
    }

    /// Returns the identifer of the module that this frame is for.
    ///
    /// Module identifiers are present in the `name` section of a WebAssembly
    /// binary, but this may not return the exact item in the `name` section.
    /// Module names can be overwritten at construction time or perhaps inferred
    /// from file names. The primary purpose of this function is to assist in
    /// debugging and therefore may be tweaked over time.
    ///
    /// This function returns `None` when no name can be found or inferred.
    pub fn module_name(&self) -> Option<&str> {
        self.module_name.as_deref()
    }

    /// Returns a descriptive name of the function for this frame, if one is
    /// available.
    ///
    /// The name of this function may come from the `name` section of the
    /// WebAssembly binary, or wasmtime may try to infer a better name for it if
    /// not available, for example the name of the export if it's exported.
    ///
    /// This return value is primarily used for debugging and human-readable
    /// purposes for things like traps. Note that the exact return value may be
    /// tweaked over time here and isn't guaranteed to be something in
    /// particular about a wasm module due to its primary purpose of assisting
    /// in debugging.
    ///
    /// This function returns `None` when no name could be inferred.
    pub fn func_name(&self) -> Option<&str> {
        self.func_name.as_deref()
    }

    /// Returns the offset within the original wasm module this frame's program
    /// counter was at.
    ///
    /// The offset here is the offset from the beginning of the original wasm
    /// module to the instruction that this frame points to.
    ///
    /// Note that `None` may be returned if the original module was not
    /// compiled with mapping information to yield this information. This is
    /// controlled by the
    /// [`Config::generate_address_map`](crate::Config::generate_address_map)
    /// configuration option.
    pub fn module_offset(&self) -> Option<usize> {
        Some(self.instr?.file_offset()? as usize)
    }

    /// Returns the offset from the original wasm module's function to this
    /// frame's program counter.
    ///
    /// The offset here is the offset from the beginning of the defining
    /// function of this frame (within the wasm module) to the instruction this
    /// frame points to.
    ///
    /// Note that `None` may be returned if the original module was not
    /// compiled with mapping information to yield this information. This is
    /// controlled by the
    /// [`Config::generate_address_map`](crate::Config::generate_address_map)
    /// configuration option.
    pub fn func_offset(&self) -> Option<usize> {
        let instr_offset = self.instr?.file_offset()?;
        Some((instr_offset - self.func_start.file_offset()?) as usize)
    }

    /// Returns the debug symbols found, if any, for this function frame.
    ///
    /// When a wasm program is compiled with DWARF debug information then this
    /// function may be populated to return symbols which contain extra debug
    /// information about a frame including the filename and line number. If no
    /// debug information was found or if it was malformed then this will return
    /// an empty array.
    pub fn symbols(&self) -> &[FrameSymbol] {
        &self.symbols
    }
}

/// Debug information for a symbol that is attached to a [`FrameInfo`].
///
/// When DWARF debug information is present in a wasm file then this structure
/// can be found on a [`FrameInfo`] and can be used to learn about filenames,
/// line numbers, etc, which are the origin of a function in a stack trace.
#[derive(Debug)]
pub struct FrameSymbol {
    name: Option<String>,
    file: Option<String>,
    line: Option<u32>,
    column: Option<u32>,
}

impl FrameSymbol {
    /// Returns the function name associated with this symbol.
    ///
    /// Note that this may not be present with malformed debug information, or
    /// the debug information may not include it. Also note that the symbol is
    /// frequently mangled, so you might need to run some form of demangling
    /// over it.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Returns the source code filename this symbol was defined in.
    ///
    /// Note that this may not be present with malformed debug information, or
    /// the debug information may not include it.
    pub fn file(&self) -> Option<&str> {
        self.file.as_deref()
    }

    /// Returns the 1-indexed source code line number this symbol was defined
    /// on.
    ///
    /// Note that this may not be present with malformed debug information, or
    /// the debug information may not include it.
    pub fn line(&self) -> Option<u32> {
        self.line
    }

    /// Returns the 1-indexed source code column number this symbol was defined
    /// on.
    ///
    /// Note that this may not be present with malformed debug information, or
    /// the debug information may not include it.
    pub fn column(&self) -> Option<u32> {
        self.column
    }
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

    GlobalModuleRegistry::with(|modules| {
        for (i, alloc) in module.compiled_module().finished_functions() {
            let (start, end) = unsafe {
                let ptr = (*alloc).as_ptr();
                let len = (*alloc).len();
                (ptr as usize, ptr as usize + len)
            };
            for pc in start..end {
                let (frame, _, _) = modules.lookup_frame_info(pc).unwrap();
                assert!(
                    frame.func_index() == i.as_u32(),
                    "lookup of {:#x} returned {}, expected {}",
                    pc,
                    frame.func_index(),
                    i.as_u32()
                );
            }
        }
    });
    Ok(())
}
