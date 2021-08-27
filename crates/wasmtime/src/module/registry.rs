//! Implements a registry of modules for a store.

use crate::{signatures::SignatureCollection, Module};
use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};
use wasmtime_environ::{EntityRef, FilePos, StackMap, TrapCode};
use wasmtime_jit::CompiledModule;
use wasmtime_runtime::{ModuleInfo, VMCallerCheckedAnyfunc, VMTrampoline};

lazy_static::lazy_static! {
    static ref GLOBAL_MODULES: RwLock<GlobalModuleRegistry> = Default::default();
}

/// Used for registering modules with a store.
///
/// The map is from the ending (exclusive) address for the module code to
/// the registered module.
///
/// The `BTreeMap` is used to quickly locate a module based on a program counter value.
#[derive(Default)]
pub struct ModuleRegistry {
    modules_with_code: BTreeMap<usize, Arc<RegisteredModule>>,
    modules_without_code: Vec<Arc<CompiledModule>>,
}

impl ModuleRegistry {
    /// Fetches information about a registered module given a program counter value.
    pub fn lookup_module(&self, pc: usize) -> Option<Arc<dyn ModuleInfo>> {
        self.module(pc)
            .map(|m| -> Arc<dyn ModuleInfo> { m.clone() })
    }

    fn module(&self, pc: usize) -> Option<&Arc<RegisteredModule>> {
        let (end, info) = self.modules_with_code.range(pc..).next()?;
        if pc < info.start || *end < pc {
            return None;
        }

        Some(info)
    }

    /// Registers a new module with the registry.
    pub fn register(&mut self, module: &Module) {
        let compiled_module = module.compiled_module();

        // If there's not actually any functions in this module then we may
        // still need to preserve it for its data segments. Instances of this
        // module will hold a pointer to the data stored in the module itself,
        // and for schemes like uffd this performs lazy initialization which
        // could use the module in the future. For that reason we continue to
        // register empty modules and retain them.
        if compiled_module.finished_functions().len() == 0 {
            self.modules_without_code.push(compiled_module.clone());
            return;
        }

        // The module code range is exclusive for end, so make it inclusive as it
        // may be a valid PC value
        let code = compiled_module.code();
        assert!(!code.is_empty());
        let start = code.as_ptr() as usize;
        let end = start + code.len() - 1;

        // Ensure the module isn't already present in the registry
        // This is expected when a module is instantiated multiple times in the
        // same store
        if let Some(m) = self.modules_with_code.get(&end) {
            assert_eq!(m.start, start);
            return;
        }

        // Assert that this module's code doesn't collide with any other registered modules
        if let Some((_, prev)) = self.modules_with_code.range(end..).next() {
            assert!(prev.start > end);
        }

        if let Some((prev_end, _)) = self.modules_with_code.range(..=start).next_back() {
            assert!(*prev_end < start);
        }

        let prev = self.modules_with_code.insert(
            end,
            Arc::new(RegisteredModule {
                start,
                module: compiled_module.clone(),
                signatures: module.signatures().clone(),
            }),
        );
        assert!(prev.is_none());

        GLOBAL_MODULES.write().unwrap().register(start, end, module);
    }

    /// Looks up a trampoline from an anyfunc.
    pub fn lookup_trampoline(&self, anyfunc: &VMCallerCheckedAnyfunc) -> Option<VMTrampoline> {
        let module = self.module(anyfunc.func_ptr.as_ptr() as usize)?;
        module.signatures.trampoline(anyfunc.type_index)
    }
}

impl Drop for ModuleRegistry {
    fn drop(&mut self) {
        let mut info = GLOBAL_MODULES.write().unwrap();
        for end in self.modules_with_code.keys() {
            info.unregister(*end);
        }
    }
}

struct RegisteredModule {
    start: usize,
    module: Arc<CompiledModule>,
    signatures: Arc<SignatureCollection>,
}

impl ModuleInfo for RegisteredModule {
    fn lookup_stack_map(&self, pc: usize) -> Option<&StackMap> {
        let text_offset = pc - self.start;
        let (index, func_offset) = self.module.func_by_text_offset(text_offset)?;
        let info = self.module.func_info(index);

        // Do a binary search to find the stack map for the given offset.
        //
        // Because GC safepoints are technically only associated with a single
        // PC, we should ideally only care about `Ok(index)` values returned
        // from the binary search. However, safepoints are inserted right before
        // calls, and there are two things that can disturb the PC/offset
        // associated with the safepoint versus the PC we actually use to query
        // for the stack map:
        //
        // 1. The `backtrace` crate gives us the PC in a frame that will be
        //    *returned to*, and where execution will continue from, rather than
        //    the PC of the call we are currently at. So we would need to
        //    disassemble one instruction backwards to query the actual PC for
        //    the stack map.
        //
        //    TODO: One thing we *could* do to make this a little less error
        //    prone, would be to assert/check that the nearest GC safepoint
        //    found is within `max_encoded_size(any kind of call instruction)`
        //    our queried PC for the target architecture.
        //
        // 2. Cranelift's stack maps only handle the stack, not
        //    registers. However, some references that are arguments to a call
        //    may need to be in registers. In these cases, what Cranelift will
        //    do is:
        //
        //      a. spill all the live references,
        //      b. insert a GC safepoint for those references,
        //      c. reload the references into registers, and finally
        //      d. make the call.
        //
        //    Step (c) adds drift between the GC safepoint and the location of
        //    the call, which is where we actually walk the stack frame and
        //    collect its live references.
        //
        //    Luckily, the spill stack slots for the live references are still
        //    up to date, so we can still find all the on-stack roots.
        //    Furthermore, we do not have a moving GC, so we don't need to worry
        //    whether the following code will reuse the references in registers
        //    (which would not have been updated to point to the moved objects)
        //    or reload from the stack slots (which would have been updated to
        //    point to the moved objects).

        let index = match info
            .stack_maps
            .binary_search_by_key(&func_offset, |i| i.code_offset)
        {
            // Exact hit.
            Ok(i) => i,

            // `Err(0)` means that the associated stack map would have been the
            // first element in the array if this pc had an associated stack
            // map, but this pc does not have an associated stack map. This can
            // only happen inside a Wasm frame if there are no live refs at this
            // pc.
            Err(0) => return None,

            Err(i) => i - 1,
        };

        Some(&info.stack_maps[index].stack_map)
    }
}

// Counterpart to `RegisteredModule`, but stored in the global registry.
struct GlobalRegisteredModule {
    start: usize,
    module: Arc<CompiledModule>,
    wasm_backtrace_details_env_used: bool,
    /// Note that modules can be instantiated in many stores, so the purpose of
    /// this field is to keep track of how many stores have registered a
    /// module. Information is only removed from the global registry when this
    /// reference count reaches 0.
    references: usize,
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
    pub(crate) fn is_wasm_pc(pc: usize) -> bool {
        let modules = GLOBAL_MODULES.read().unwrap();

        match modules.module(pc) {
            Some((entry, text_offset)) => {
                wasmtime_environ::lookup_file_pos(entry.module.address_map_data(), text_offset)
                    .is_some()
            }
            None => false,
        }
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

    /// Registers a new region of code, described by `(start, end)` and with
    /// the given function information, with the global information.
    fn register(&mut self, start: usize, end: usize, module: &Module) {
        let info = self.0.entry(end).or_insert_with(|| GlobalRegisteredModule {
            start,
            module: module.compiled_module().clone(),
            wasm_backtrace_details_env_used: module
                .engine()
                .config()
                .wasm_backtrace_details_env_used,
            references: 0,
        });

        // Note that ideally we'd debug_assert that the information previously
        // stored, if any, matches the `functions` we were given, but for now we
        // just do some simple checks to hope it's the same.
        assert_eq!(info.start, start);
        info.references += 1;
    }

    /// Unregisters a region of code (keyed by the `end` address) from the
    /// global information.
    fn unregister(&mut self, end: usize) {
        let info = self.0.get_mut(&end).unwrap();
        info.references -= 1;
        if info.references == 0 {
            self.0.remove(&end);
        }
    }
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
        debug_assert!(
            instr.is_some(),
            "failed to find instruction for {:#x}",
            text_offset
        );

        let instr = instr.unwrap_or(info.start_srcloc);

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
            if let Some(offset) = instr.file_offset() {
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
            func_name: module.func_names.get(&index).cloned(),
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
    instr: FilePos,
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
    pub fn module_offset(&self) -> usize {
        self.instr.file_offset().unwrap_or(u32::MAX) as usize
    }

    /// Returns the offset from the original wasm module's function to this
    /// frame's program counter.
    ///
    /// The offset here is the offset from the beginning of the defining
    /// function of this frame (within the wasm module) to the instruction this
    /// frame points to.
    pub fn func_offset(&self) -> usize {
        match self.instr.file_offset() {
            Some(i) => (i - self.func_start.file_offset().unwrap()) as usize,
            None => u32::MAX as usize,
        }
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
