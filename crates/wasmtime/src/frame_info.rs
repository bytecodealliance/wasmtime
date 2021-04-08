use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Mutex;
use wasmtime_environ::entity::EntityRef;
use wasmtime_environ::ir;
use wasmtime_environ::wasm::DefinedFuncIndex;
use wasmtime_environ::{FunctionAddressMap, TrapInformation};
use wasmtime_jit::CompiledModule;

/// This is a structure that lives within a `Store` and retains information
/// about all modules registered with the `Store` via instantiation.
///
/// "frame information" here refers to things like determining whether a
/// program counter is a wasm program counter, and additionally mapping program
/// counters to wasm filenames, modules, line numbers, etc. This store of
/// information lives as long as a `Store` lives since modules are never
/// unloaded today.
#[derive(Default)]
pub struct StoreFrameInfo {
    /// An internal map that keeps track of backtrace frame information for
    /// each module.
    ///
    /// This map is morally a map of ranges to a map of information for that
    /// module. Each module is expected to reside in a disjoint section of
    /// contiguous memory. No modules can overlap.
    ///
    /// The key of this map is the highest address in the module and the value
    /// is the module's information, which also contains the start address.
    ranges: BTreeMap<usize, ModuleFrameInfo>,
}

impl StoreFrameInfo {
    /// Fetches frame information about a program counter in a backtrace.
    ///
    /// Returns an object if this `pc` is known to some previously registered
    /// module, or returns `None` if no information can be found. The boolean
    /// returned indicates whether the original module has unparsed debug
    /// information due to the compiler's configuration.
    pub fn lookup_frame_info(&self, pc: usize) -> Option<(FrameInfo, bool)> {
        let module = self.module(pc)?;
        module
            .lookup_frame_info(pc)
            .map(|info| (info, module.has_unparsed_debuginfo()))
    }

    /// Fetches trap information about a program counter in a backtrace.
    pub fn lookup_trap_info(&self, pc: usize) -> Option<&TrapInformation> {
        self.module(pc)?.lookup_trap_info(pc)
    }

    fn module(&self, pc: usize) -> Option<&ModuleFrameInfo> {
        let (end, info) = self.ranges.range(pc..).next()?;
        if pc < info.start || *end < pc {
            return None;
        }

        Some(info)
    }

    /// Registers a new compiled module's frame information.
    pub fn register(&mut self, module: &Arc<CompiledModule>) {
        let (start, end) = module.code().range();

        // Ignore modules with no code or finished functions
        if start == end || module.finished_functions().is_empty() {
            return;
        }

        // The module code range is exclusive for end, so make it inclusive as it
        // may be a valid PC value
        let end = end - 1;

        // Assert that this module's code doesn't collide with any other registered modules
        if let Some((_, prev)) = self.ranges.range(end..).next() {
            assert!(prev.start > end);
        }
        if let Some((prev_end, _)) = self.ranges.range(..=start).next_back() {
            assert!(*prev_end < start);
        }

        let prev = self.ranges.insert(
            end,
            ModuleFrameInfo {
                start,
                module: module.clone(),
            },
        );
        assert!(prev.is_none());

        GLOBAL_INFO.lock().unwrap().register(start, end, module);
    }
}

impl Drop for StoreFrameInfo {
    fn drop(&mut self) {
        let mut info = GLOBAL_INFO.lock().unwrap();
        for end in self.ranges.keys() {
            info.unregister(*end);
        }
    }
}

/// Represents a module's frame information.
#[derive(Clone)]
pub struct ModuleFrameInfo {
    start: usize,
    module: Arc<CompiledModule>,
}

impl ModuleFrameInfo {
    /// Determines if the related module has unparsed debug information.
    pub fn has_unparsed_debuginfo(&self) -> bool {
        self.module.has_unparsed_debuginfo()
    }

    /// Fetches frame information about a program counter in a backtrace.
    ///
    /// Returns an object if this `pc` is known to this module, or returns `None`
    /// if no information can be found.
    pub fn lookup_frame_info(&self, pc: usize) -> Option<FrameInfo> {
        let (index, offset) = self.func(pc)?;
        let (addr_map, _) = self.module.func_info(index);
        let pos = Self::instr_pos(offset, addr_map);

        // In debug mode for now assert that we found a mapping for `pc` within
        // the function, because otherwise something is buggy along the way and
        // not accounting for all the instructions. This isn't super critical
        // though so we can omit this check in release mode.
        debug_assert!(pos.is_some(), "failed to find instruction for {:x}", pc);

        let instr = match pos {
            Some(pos) => addr_map.instructions[pos].srcloc,
            None => addr_map.start_srcloc,
        };

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
            let to_lookup = (instr.bits() as u64) - s.code_section_offset();
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

        let module = self.module.module();
        let index = module.func_index(index);

        Some(FrameInfo {
            module_name: module.name.clone(),
            func_index: index.index() as u32,
            func_name: module.func_names.get(&index).cloned(),
            instr,
            func_start: addr_map.start_srcloc,
            symbols,
        })
    }

    /// Fetches trap information about a program counter in a backtrace.
    pub fn lookup_trap_info(&self, pc: usize) -> Option<&TrapInformation> {
        let (index, offset) = self.func(pc)?;
        let (_, traps) = self.module.func_info(index);
        let idx = traps
            .binary_search_by_key(&offset, |info| info.code_offset)
            .ok()?;
        Some(&traps[idx])
    }

    fn func(&self, pc: usize) -> Option<(DefinedFuncIndex, u32)> {
        let (index, start, _) = self.module.func_by_pc(pc)?;
        Some((index, (pc - start) as u32))
    }

    fn instr_pos(offset: u32, addr_map: &FunctionAddressMap) -> Option<usize> {
        // Use our relative position from the start of the function to find the
        // machine instruction that corresponds to `pc`, which then allows us to
        // map that to a wasm original source location.
        match addr_map
            .instructions
            .binary_search_by_key(&offset, |map| map.code_offset)
        {
            // Exact hit!
            Ok(pos) => Some(pos),

            // This *would* be at the first slot in the array, so no
            // instructions cover `pc`.
            Err(0) => None,

            // This would be at the `nth` slot, so we're at the `n-1`th slot.
            Err(n) => Some(n - 1),
        }
    }
}

/// This is the dual of `StoreFrameInfo` and is stored globally (as the name
/// implies) rather than simply in one `Store`.
///
/// The purpose of this map is to be called from signal handlers to determine
/// whether a program counter is a wasm trap or not. Specifically macOS has
/// no contextual information about the thread available, hence the necessity
/// for global state rather than using thread local state.
///
/// This is similar to `StoreFrameInfo` except that it has less information and
/// supports removal. Any time anything is registered with a `StoreFrameInfo`
/// it is also automatically registered with the singleton global frame
/// information. When a `StoreFrameInfo` is destroyed then all of its entries
/// are removed from the global frame information.
#[derive(Default)]
pub struct GlobalFrameInfo {
    // The map here behaves the same way as `StoreFrameInfo`.
    ranges: BTreeMap<usize, GlobalModuleFrameInfo>,
}

/// This is the equivalent of `ModuleFrameInfo` except it keeps a reference count.
struct GlobalModuleFrameInfo {
    module: ModuleFrameInfo,

    /// Note that modules can be instantiated in many stores, so the purpose of
    /// this field is to keep track of how many stores have registered a
    /// module. Information is only removed from the global store when this
    /// reference count reaches 0.
    references: usize,
}

lazy_static::lazy_static! {
    static ref GLOBAL_INFO: Mutex<GlobalFrameInfo> = Default::default();
}

impl GlobalFrameInfo {
    /// Returns whether the `pc`, according to globally registered information,
    /// is a wasm trap or not.
    pub(crate) fn is_wasm_pc(pc: usize) -> bool {
        let info = GLOBAL_INFO.lock().unwrap();

        match info.ranges.range(pc..).next() {
            Some((end, info)) => {
                if pc < info.module.start || *end < pc {
                    return false;
                }

                match info.module.func(pc) {
                    Some((index, offset)) => {
                        let (addr_map, _) = info.module.module.func_info(index);
                        ModuleFrameInfo::instr_pos(offset, addr_map).is_some()
                    }
                    None => false,
                }
            }
            None => false,
        }
    }

    /// Registers a new region of code, described by `(start, end)` and with
    /// the given function information, with the global information.
    fn register(&mut self, start: usize, end: usize, module: &Arc<CompiledModule>) {
        let info = self
            .ranges
            .entry(end)
            .or_insert_with(|| GlobalModuleFrameInfo {
                module: ModuleFrameInfo {
                    start,
                    module: module.clone(),
                },
                references: 0,
            });

        // Note that ideally we'd debug_assert that the information previously
        // stored, if any, matches the `functions` we were given, but for now we
        // just do some simple checks to hope it's the same.
        assert_eq!(info.module.start, start);
        info.references += 1;
    }

    /// Unregisters a region of code (keyed by the `end` address) from this
    /// global information.
    fn unregister(&mut self, end: usize) {
        let info = self.ranges.get_mut(&end).unwrap();
        info.references -= 1;
        if info.references == 0 {
            self.ranges.remove(&end);
        }
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
    func_start: ir::SourceLoc,
    instr: ir::SourceLoc,
    symbols: Vec<FrameSymbol>,
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
        self.instr.bits() as usize
    }

    /// Returns the offset from the original wasm module's function to this
    /// frame's program counter.
    ///
    /// The offset here is the offset from the beginning of the defining
    /// function of this frame (within the wasm module) to the instruction this
    /// frame points to.
    pub fn func_offset(&self) -> usize {
        (self.instr.bits() - self.func_start.bits()) as usize
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
    let store = Store::default();
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
    Instance::new(&store, &module, &[])?;
    let info = store.frame_info().borrow();
    for (i, alloc) in module.compiled_module().finished_functions() {
        let (start, end) = unsafe {
            let ptr = (**alloc).as_ptr();
            let len = (**alloc).len();
            (ptr as usize, ptr as usize + len)
        };
        for pc in start..end {
            let (frame, _) = info.lookup_frame_info(pc).unwrap();
            assert!(frame.func_index() == i.as_u32());
        }
    }
    Ok(())
}
