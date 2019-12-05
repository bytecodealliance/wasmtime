//! Oracles.
//!
//! Oracles take a test case and determine whether we have a bug. For example,
//! one of the simplest oracles is to take a Wasm binary as our input test case,
//! validate and instantiate it, and (implicitly) check that no assertions
//! failed or segfaults happened. A more complicated oracle might compare the
//! result of executing a Wasm file with and without optimizations enabled, and
//! make sure that the two executions are observably identical.
//!
//! When an oracle finds a bug, it should report it to the fuzzing engine by
//! panicking.

pub mod dummy;

use dummy::dummy_imports;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasmtime::{Config, Engine, HostRef, Instance, Module, Store};
use wasmtime_environ::{isa, settings};
use wasmtime_jit::{native, CompilationStrategy, CompiledModule, Compiler, NullResolver};

fn host_isa() -> Box<dyn isa::TargetIsa> {
    let flag_builder = settings::builder();
    let isa_builder = native::builder();
    isa_builder.finish(settings::Flags::new(flag_builder))
}

/// Instantiate the Wasm buffer, and implicitly fail if we have an unexpected
/// panic or segfault or anything else that can be detected "passively".
///
/// Performs initial validation, and returns early if the Wasm is invalid.
///
/// You can control which compiler is used via passing a `CompilationStrategy`.
pub fn instantiate(wasm: &[u8], compilation_strategy: CompilationStrategy) {
    if wasmparser::validate(wasm, None).is_err() {
        return;
    }

    let mut config = Config::new();
    config.strategy(compilation_strategy);

    let engine = HostRef::new(Engine::new(&config));
    let store = HostRef::new(Store::new(&engine));

    let module =
        HostRef::new(Module::new(&store, wasm).expect("Failed to compile a valid Wasm module!"));

    let imports = {
        let module = module.borrow();
        match dummy_imports(&store, module.imports()) {
            Ok(imps) => imps,
            Err(_) => {
                // There are some value types that we can't synthesize a
                // dummy value for (e.g. anyrefs) and for modules that
                // import things of these types we skip instantiation.
                return;
            }
        }
    };

    // Don't unwrap this: there can be instantiation-/link-time errors that
    // aren't caught during validation or compilation. For example, an imported
    // table might not have room for an element segment that we want to
    // initialize into it.
    let _result = Instance::new(&store, &module, &imports);
}

/// Compile the Wasm buffer, and implicitly fail if we have an unexpected
/// panic or segfault or anything else that can be detected "passively".
///
/// Performs initial validation, and returns early if the Wasm is invalid.
///
/// You can control which compiler is used via passing a `CompilationStrategy`.
pub fn compile(wasm: &[u8], compilation_strategy: CompilationStrategy) {
    if wasmparser::validate(wasm, None).is_err() {
        return;
    }

    let isa = host_isa();
    let mut compiler = Compiler::new(isa, compilation_strategy);
    let mut resolver = NullResolver {};
    let global_exports = Rc::new(RefCell::new(HashMap::new()));
    let _ = CompiledModule::new(&mut compiler, wasm, &mut resolver, global_exports, false);
}
