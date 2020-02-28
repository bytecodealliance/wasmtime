use crate::module::Names;
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};
use wasmtime_environ::entity::EntityRef;
use wasmtime_environ::wasm::FuncIndex;
use wasmtime_jit::CompiledModule;

lazy_static::lazy_static! {
    /// This is a global cache of backtrace frame information for all active
    ///
    /// This global cache is used during `Trap` creation to symbolicate frames.
    /// This is populated on module compilation, and it is cleared out whenever
    /// all references to a module are dropped.
    pub static ref FRAME_INFO: GlobalFrameInfo = GlobalFrameInfo::default();
}

#[derive(Default)]
pub struct GlobalFrameInfo {
    /// An internal map that keeps track of backtrace frame information for
    /// each module.
    ///
    /// This map is morally a map of ranges to a map of information for that
    /// module. Each module is expected to reside in a disjoint section of
    /// contiguous memory. No modules can overlap.
    ///
    /// The key of this map is the highest address in the module and the value
    /// is the module's information, which also contains the start address.
    ranges: RwLock<BTreeMap<usize, ModuleFrameInfo>>,
}

/// An RAII structure used to unregister a module's frame information when the
/// module is destroyed.
pub struct GlobalFrameInfoRegistration {
    /// The key that will be removed from the global `ranges` map when this is
    /// dropped.
    key: usize,
}

struct ModuleFrameInfo {
    start: usize,
    functions: BTreeMap<usize, (usize, FuncIndex)>,
    names: Arc<Names>,
}

impl GlobalFrameInfo {
    /// Registers a new compiled module's frame information.
    ///
    /// This function will register the `names` information for all of the
    /// compiled functions within `module`. If the `module` has no functions
    /// then `None` will be returned. Otherwise the returned object, when
    /// dropped, will be used to unregister all name information from this map.
    pub fn register(
        &self,
        names: &Arc<Names>,
        module: &CompiledModule,
    ) -> Option<GlobalFrameInfoRegistration> {
        let mut min = usize::max_value();
        let mut max = 0;
        let mut functions = BTreeMap::new();
        for (i, allocated) in module.finished_functions() {
            let (start, end) = unsafe {
                let ptr = (**allocated).as_ptr();
                let len = (**allocated).len();
                (ptr as usize, ptr as usize + len)
            };
            if start < min {
                min = start;
            }
            if end > max {
                max = end;
            }
            let func_index = module.module().local.func_index(i);
            assert!(functions.insert(end, (start, func_index)).is_none());
        }
        if functions.len() == 0 {
            return None;
        }

        let mut ranges = self.ranges.write().unwrap();
        // First up assert that our chunk of jit functions doesn't collide with
        // any other known chunks of jit functions...
        if let Some((_, prev)) = ranges.range(max..).next() {
            assert!(prev.start > max);
        }
        if let Some((prev_end, _)) = ranges.range(..=min).next_back() {
            assert!(*prev_end < min);
        }

        // ... then insert our range and assert nothing was there previously
        let prev = ranges.insert(
            max,
            ModuleFrameInfo {
                start: min,
                functions,
                names: names.clone(),
            },
        );
        assert!(prev.is_none());
        Some(GlobalFrameInfoRegistration { key: max })
    }

    /// Fetches information about a program counter in a backtrace.
    ///
    /// Returns an object if this `pc` is known to some previously registered
    /// module, or returns `None` if no information can be found.
    pub fn lookup(&self, pc: usize) -> Option<FrameInfo> {
        let ranges = self.ranges.read().ok()?;
        let (end, info) = ranges.range(pc..).next()?;
        if pc < info.start || *end < pc {
            return None;
        }
        let (end, (start, func_index)) = info.functions.range(pc..).next()?;
        if pc < *start || *end < pc {
            return None;
        }
        Some(FrameInfo {
            module_name: info.names.module_name.clone(),
            func_index: func_index.index() as u32,
            func_name: info.names.module.func_names.get(func_index).cloned(),
        })
    }
}

impl Drop for GlobalFrameInfoRegistration {
    fn drop(&mut self) {
        if let Ok(mut map) = FRAME_INFO.ranges.write() {
            map.remove(&self.key);
        }
    }
}

/// Description of a frame in a backtrace for a [`Trap`].
///
/// Whenever a WebAssembly trap occurs an instance of [`Trap`] is created. Each
/// [`Trap`] has a backtrace of the WebAssembly frames that led to the trap, and
/// each frame is described by this structure.
#[derive(Debug)]
pub struct FrameInfo {
    module_name: Option<String>,
    func_index: u32,
    func_name: Option<String>,
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
}
