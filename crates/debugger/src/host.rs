//! Host implementation for the debugger world.

use wasmtime::{
    Result,
    component::{Resource, ResourceTable},
};

mod api;
mod bindings;
mod opaque;

pub use api::Debuggee;
pub use bindings::DebugMain as DebuggerComponent;
pub use bindings::bytecodealliance::wasmtime::debuggee as wit;
use opaque::OpaqueDebugger;

/// Register a debuggee in a resource table.
pub fn add_debuggee<T: Send + 'static>(
    table: &mut ResourceTable,
    debuggee: crate::Debuggee<T>,
) -> Result<Resource<Debuggee>> {
    let engine = debuggee.engine().clone();
    let interrupt_pending = debuggee.interrupt_pending().clone();
    let inner: Option<Box<dyn OpaqueDebugger + Send + 'static>> = Some(Box::new(debuggee));
    Ok(table.push(Debuggee {
        inner,
        engine,
        interrupt_pending,
    })?)
}

impl bindings::DebugMainImports for ResourceTable {
    async fn print_debugger_info(&mut self, message: String) -> wasmtime::Result<()> {
        eprintln!("Debugger: {message}");
        Ok(())
    }
}

/// Add the debugger world's host functions to a [`wasmtime::component::Linker`].
pub fn add_to_linker<T: Send + 'static>(
    linker: &mut wasmtime::component::Linker<T>,
    f: fn(&mut T) -> &mut ResourceTable,
) -> wasmtime::Result<()> {
    wit::add_to_linker::<_, wasmtime::component::HasSelf<ResourceTable>>(linker, f)?;
    bindings::DebugMain::add_to_linker_imports::<_, wasmtime::component::HasSelf<ResourceTable>>(
        linker, f,
    )?;
    Ok(())
}
