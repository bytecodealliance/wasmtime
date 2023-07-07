use std::fmt;

use crate::{store::StoreOpaque, FrameInfo, Global, Instance, Memory, Module, WasmBacktrace};

/// Representation of a core dump of a WebAssembly module
///
/// When the Config::coredump_on_trap option is enabled this structure is
/// attached to the [`anyhow::Error`] returned from many Wasmtime functions that
/// execute WebAssembly such as [`Instance::new`] or [`Func::call`]. This can be
/// acquired with the [`anyhow::Error::downcast`] family of methods to
/// programmatically inspect the coredump. Otherwise since it's part of the
/// error returned this will get printed along with the rest of the error when
/// the error is logged.
///
/// Note that some state, such as Wasm locals or values on the operand stack,
/// may be optimized away by the compiler or otherwise not recovered in the
/// coredump.
///
/// Capturing of wasm coredumps can be configured through the
/// [`Config::coredump_on_trap`][crate::Config::coredump_on_trap] method.
///
/// For more information about errors in wasmtime see the documentation of the
/// [`Trap`][crate::Trap] type.
///
/// [`Func::call`]: crate::Func::call
/// [`Instance::new`]: crate::Instance::new
pub struct WasmCoreDump {
    name: String,
    modules: Vec<Module>,
    instances: Vec<Instance>,
    store_memories: Vec<Memory>,
    store_globals: Vec<Global>,
    backtrace: WasmBacktrace,
}

impl WasmCoreDump {
    pub(crate) fn new(store: &StoreOpaque, backtrace: WasmBacktrace) -> WasmCoreDump {
        let modules: Vec<_> = store.modules().all_modules().cloned().collect();
        let instances: Vec<Instance> = store.all_instances().collect();
        let store_memories: Vec<Memory> = store.all_memories().collect();
        let store_globals: Vec<Global> = store.all_globals().collect();

        WasmCoreDump {
            name: String::from("store_name"),
            modules,
            instances,
            store_memories,
            store_globals,
            backtrace,
        }
    }

    /// The stack frames for the CoreDump
    pub fn frames(&self) -> &[FrameInfo] {
        self.backtrace.frames()
    }

    /// The names of the modules involved in the CoreDump
    pub fn modules(&self) -> &[Module] {
        self.modules.as_ref()
    }

    /// The instances involved in the CoreDump
    pub fn instances(&self) -> &[Instance] {
        self.instances.as_ref()
    }

    /// The imported globals that belong to the store, rather than a specific
    /// instance
    pub fn store_globals(&self) -> &[Global] {
        self.store_globals.as_ref()
    }

    /// The imported memories that belong to the store, rather than a specific
    /// instance.
    pub fn store_memories(&self) -> &[Memory] {
        self.store_memories.as_ref()
    }
}

impl fmt::Display for WasmCoreDump {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "wasm coredump generated while executing {}:", self.name)?;
        writeln!(f, "modules:")?;
        for module in self.modules.iter() {
            writeln!(f, "  {}", module.name().unwrap_or("<module>"))?;
        }

        writeln!(f, "instances:")?;
        for instance in self.instances.iter() {
            writeln!(f, "  {:?}", instance)?;
        }

        writeln!(f, "memories:")?;
        for memory in self.store_memories.iter() {
            writeln!(f, "  {:?}", memory)?;
        }

        writeln!(f, "globals:")?;
        for global in self.store_globals.iter() {
            writeln!(f, "  {:?}", global)?;
        }

        writeln!(f, "backtrace:")?;
        write!(f, "{}", self.backtrace)?;

        Ok(())
    }
}

impl fmt::Debug for WasmCoreDump {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<wasm core dump>")
    }
}
