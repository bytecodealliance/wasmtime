use std::fmt;

use crate::{store::StoreOpaque, FrameInfo, Global, Instance, Memory, WasmBacktrace};

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
#[derive(Debug)]
pub struct WasmCoreDump {
    name: String,
    modules: Vec<String>,
    instances: Vec<Instance>,
    memories: Vec<Memory>,
    globals: Vec<Global>,
    backtrace: WasmBacktrace,
}

impl WasmCoreDump {
    pub(crate) fn new(store: &StoreOpaque, backtrace: WasmBacktrace) -> WasmCoreDump {
        let modules: Vec<_> = store
            .modules()
            .all_modules()
            .cloned()
            .map(|m| String::from(m.name().unwrap_or_default()))
            .collect();
        let instances: Vec<Instance> = store.all_instances().collect();
        let memories: Vec<Memory> = store.all_memories().collect();
        let globals: Vec<Global> = store.all_globals().collect();
        WasmCoreDump {
            name: String::from("store_name"),
            modules,
            instances,
            memories,
            globals,
            backtrace,
        }
    }

    /// The stack frames for the CoreDump
    pub fn frames(&self) -> &[FrameInfo] {
        self.backtrace.frames()
    }

    /// The names of the modules involved in the CoreDump
    pub fn modules(&self) -> &[String] {
        self.modules.as_ref()
    }

    /// The instances involved in the CoreDump
    pub fn instances(&self) -> &[Instance] {
        self.instances.as_ref()
    }

    /// The globals involved in the CoreDump
    pub fn globals(&self) -> &[Global] {
        self.globals.as_ref()
    }

    /// The memories involve din the CoreDump
    pub fn memories(&self) -> &[Memory] {
        self.memories.as_ref()
    }
}

impl fmt::Display for WasmCoreDump {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "wasm coredump generated while executing {}:", self.name)?;
        writeln!(f, "modules:")?;
        for module in self.modules.iter() {
            writeln!(f, "  {}", module)?;
        }

        writeln!(f, "instances:")?;
        for instance in self.instances.iter() {
            writeln!(f, "  {:?}", instance)?;
        }

        writeln!(f, "memories:")?;
        for memory in self.memories.iter() {
            writeln!(f, "  {:?}", memory)?;
        }

        writeln!(f, "globals:")?;
        for global in self.globals.iter() {
            writeln!(f, "  {:?}", global)?;
        }

        writeln!(f, "backtrace:")?;
        write!(f, "{}", self.backtrace)?;

        Ok(())
    }
}
