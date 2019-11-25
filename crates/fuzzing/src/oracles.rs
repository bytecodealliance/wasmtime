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

use cranelift_codegen::settings;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasmtime_jit::{CompilationStrategy, CompiledModule, Compiler, NullResolver};

fn host_isa() -> Box<dyn cranelift_codegen::isa::TargetIsa> {
    let flag_builder = settings::builder();
    let isa_builder = cranelift_native::builder().expect("host machine is not a supported target");
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

    let isa = host_isa();
    let mut compiler = Compiler::new(isa, compilation_strategy);
    let mut imports_resolver = NullResolver {};

    wasmtime_jit::instantiate(
        &mut compiler,
        wasm,
        &mut imports_resolver,
        Default::default(),
        true,
    )
    .expect("failed to instantiate valid Wasm!");
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
