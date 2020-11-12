use std::cmp;
use std::collections::BTreeMap;
use std::sync::Arc;
use wasmtime_environ::entity::EntityRef;
use wasmtime_environ::ir;
use wasmtime_environ::wasm::FuncIndex;
use wasmtime_environ::{FunctionAddressMap, Module, TrapInformation};
use wasmtime_jit::CompiledModule;

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

struct ModuleFrameInfo {
    start: usize,
    functions: BTreeMap<usize, FunctionInfo>,
    module: Arc<Module>,
}

struct FunctionInfo {
    start: usize,
    index: FuncIndex,
    traps: Vec<TrapInformation>,
    instr_map: FunctionAddressMap,
}

impl StoreFrameInfo {
    /// Fetches frame information about a program counter in a backtrace.
    ///
    /// Returns an object if this `pc` is known to some previously registered
    /// module, or returns `None` if no information can be found.
    pub fn lookup_frame_info(&self, pc: usize) -> Option<FrameInfo> {
        let (module, func) = self.func(pc)?;

        // Use our relative position from the start of the function to find the
        // machine instruction that corresponds to `pc`, which then allows us to
        // map that to a wasm original source location.
        let rel_pos = (pc - func.start) as u32;
        let pos = match func
            .instr_map
            .instructions
            .binary_search_by_key(&rel_pos, |map| map.code_offset)
        {
            // Exact hit!
            Ok(pos) => Some(pos),

            // This *would* be at the first slot in the array, so no
            // instructions cover `pc`.
            Err(0) => None,

            // This would be at the `nth` slot, so we're at the `n-1`th slot.
            Err(n) => Some(n - 1),
        };

        // In debug mode for now assert that we found a mapping for `pc` within
        // the function, because otherwise something is buggy along the way and
        // not accounting for all the instructions. This isn't super critical
        // though so we can omit this check in release mode.
        debug_assert!(pos.is_some(), "failed to find instruction for {:x}", pc);

        let instr = match pos {
            Some(pos) => func.instr_map.instructions[pos].srcloc,
            None => func.instr_map.start_srcloc,
        };
        Some(FrameInfo {
            module_name: module.module.name.clone(),
            func_index: func.index.index() as u32,
            func_name: module.module.func_names.get(&func.index).cloned(),
            instr,
            func_start: func.instr_map.start_srcloc,
        })
    }

    /// Returns whether the `pc` specified is contaained within some module's
    /// function.
    pub fn contains_pc(&self, pc: usize) -> bool {
        self.func(pc).is_some()
    }

    /// Fetches trap information about a program counter in a backtrace.
    pub fn lookup_trap_info(&self, pc: usize) -> Option<&TrapInformation> {
        let (_module, func) = self.func(pc)?;
        let idx = func
            .traps
            .binary_search_by_key(&((pc - func.start) as u32), |info| info.code_offset)
            .ok()?;
        Some(&func.traps[idx])
    }

    fn func(&self, pc: usize) -> Option<(&ModuleFrameInfo, &FunctionInfo)> {
        let (end, info) = self.ranges.range(pc..).next()?;
        if pc < info.start || *end < pc {
            return None;
        }
        let (end, func) = info.functions.range(pc..).next()?;
        if pc < func.start || *end < pc {
            return None;
        }
        Some((info, func))
    }

    /// Registers a new compiled module's frame information.
    ///
    /// This function will register the `names` information for all of the
    /// compiled functions within `module`. If the `module` has no functions
    /// then `None` will be returned. Otherwise the returned object, when
    /// dropped, will be used to unregister all name information from this map.
    pub fn register(&mut self, module: &CompiledModule) {
        let mut min = usize::max_value();
        let mut max = 0;
        let mut functions = BTreeMap::new();
        for (i, allocated, traps, address_map) in module.trap_information() {
            let (start, end) = unsafe {
                let ptr = (*allocated).as_ptr();
                let len = (*allocated).len();
                // First and last byte of the function text.
                (ptr as usize, ptr as usize + len - 1)
            };
            // Skip empty functions.
            if end < start {
                continue;
            }
            min = cmp::min(min, start);
            max = cmp::max(max, end);
            let func = FunctionInfo {
                start,
                index: module.module().func_index(i),
                traps: traps.to_vec(),
                instr_map: address_map.clone(),
            };
            assert!(functions.insert(end, func).is_none());
        }
        if functions.len() == 0 {
            return;
        }

        // First up assert that our chunk of jit functions doesn't collide with
        // any other known chunks of jit functions...
        if let Some((_, prev)) = self.ranges.range(max..).next() {
            assert!(prev.start > max);
        }
        if let Some((prev_end, _)) = self.ranges.range(..=min).next_back() {
            assert!(*prev_end < min);
        }

        // ... then insert our range and assert nothing was there previously
        let prev = self.ranges.insert(
            max,
            ModuleFrameInfo {
                start: min,
                functions,
                module: module.module().clone(),
            },
        );
        assert!(prev.is_none());
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
            let frame = info.lookup_frame_info(pc).unwrap();
            assert!(frame.func_index() == i.as_u32());
        }
    }
    Ok(())
}
