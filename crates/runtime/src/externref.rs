#[cfg(feature = "gc")]
mod gc;
#[cfg(feature = "gc")]
pub use gc::*;

#[cfg(not(feature = "gc"))]
mod no_gc;
#[cfg(not(feature = "gc"))]
pub use no_gc::*;

use wasmtime_environ::StackMap;

/// Used by the runtime to lookup information about a module given a
/// program counter value.
pub trait ModuleInfoLookup {
    /// Lookup the module information from a program counter value.
    fn lookup(&self, pc: usize) -> Option<&dyn ModuleInfo>;
}

/// Used by the runtime to query module information.
pub trait ModuleInfo {
    /// Lookup the stack map at a program counter value.
    fn lookup_stack_map(&self, pc: usize) -> Option<&StackMap>;
}
