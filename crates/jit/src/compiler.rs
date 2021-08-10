//! JIT compilation.

use crate::instantiate::SetupError;
use crate::object::{build_object, ObjectUnwindInfo};
use object::write::Object;
#[cfg(feature = "parallel-compilation")]
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::mem;
use wasmparser::WasmFeatures;
use wasmtime_environ::entity::EntityRef;
use wasmtime_environ::wasm::{DefinedMemoryIndex, MemoryIndex};
use wasmtime_environ::{
    CompiledFunctions, Compiler as EnvCompiler, CompilerBuilder, ModuleMemoryOffset,
    ModuleTranslation, Tunables, TypeTables, VMOffsets,
};

/// Select which kind of compilation to use.
#[derive(Copy, Clone, Debug, Hash, Serialize, Deserialize, Eq, PartialEq)]
pub enum CompilationStrategy {
    /// Let Wasmtime pick the strategy.
    Auto,

    /// Compile all functions with Cranelift.
    Cranelift,

    /// Compile all functions with Lightbeam.
    #[cfg(feature = "lightbeam")]
    Lightbeam,
}

/// A WebAssembly code JIT compiler.
pub struct Compiler {
    compiler: Box<dyn EnvCompiler>,
    tunables: Tunables,
    features: WasmFeatures,
    parallel_compilation: bool,
}

impl Compiler {
    /// Creates a new compiler builder for the provided compilation strategy.
    pub fn builder(strategy: CompilationStrategy) -> Box<dyn CompilerBuilder> {
        match strategy {
            CompilationStrategy::Auto | CompilationStrategy::Cranelift => {
                wasmtime_cranelift::builder()
            }
            #[cfg(feature = "lightbeam")]
            CompilationStrategy::Lightbeam => unimplemented!(),
        }
    }

    /// Creates a new instance of a `Compiler` from the provided compiler
    /// builder.
    pub fn new(
        builder: &dyn CompilerBuilder,
        tunables: Tunables,
        features: WasmFeatures,
        parallel_compilation: bool,
    ) -> Compiler {
        Compiler {
            compiler: builder.build(),
            tunables,
            features,
            parallel_compilation,
        }
    }
}

fn _assert_compiler_send_sync() {
    fn _assert<T: Send + Sync>() {}
    _assert::<Compiler>();
}

#[allow(missing_docs)]
pub struct Compilation {
    pub obj: Object,
    pub unwind_info: Vec<ObjectUnwindInfo>,
    pub funcs: CompiledFunctions,
}

impl Compiler {
    /// Return the tunables in use by this engine.
    pub fn tunables(&self) -> &Tunables {
        &self.tunables
    }

    /// Return the enabled wasm features.
    pub fn features(&self) -> &WasmFeatures {
        &self.features
    }

    /// Return the underlying compiler in use
    pub fn compiler(&self) -> &dyn EnvCompiler {
        &*self.compiler
    }

    /// Returns the target this compiler is compiling for.
    pub fn triple(&self) -> &target_lexicon::Triple {
        self.compiler.triple()
    }

    /// Compile the given function bodies.
    pub fn compile<'data>(
        &self,
        translation: &mut ModuleTranslation,
        types: &TypeTables,
    ) -> Result<Compilation, SetupError> {
        let functions = mem::take(&mut translation.function_body_inputs);
        let functions = functions.into_iter().collect::<Vec<_>>();

        let funcs = self
            .run_maybe_parallel(functions, |(index, func)| {
                self.compiler
                    .compile_function(translation, index, func, &self.tunables, types)
            })?
            .into_iter()
            .collect::<CompiledFunctions>();

        let dwarf_sections = if self.tunables.generate_native_debuginfo && !funcs.is_empty() {
            let ofs = VMOffsets::new(
                self.compiler
                    .triple()
                    .architecture
                    .pointer_width()
                    .unwrap()
                    .bytes(),
                &translation.module,
            );

            let memory_offset = if ofs.num_imported_memories > 0 {
                ModuleMemoryOffset::Imported(ofs.vmctx_vmmemory_import(MemoryIndex::new(0)))
            } else if ofs.num_defined_memories > 0 {
                ModuleMemoryOffset::Defined(
                    ofs.vmctx_vmmemory_definition_base(DefinedMemoryIndex::new(0)),
                )
            } else {
                ModuleMemoryOffset::None
            };
            self.compiler
                .emit_dwarf(&translation.debuginfo, &funcs, &memory_offset)
                .map_err(SetupError::DebugInfo)?
        } else {
            vec![]
        };

        let (obj, unwind_info) = build_object(self, &translation, types, &funcs, dwarf_sections)?;

        Ok(Compilation {
            obj,
            unwind_info,
            funcs,
        })
    }

    /// Run the given closure in parallel if the compiler is configured to do so.
    pub(crate) fn run_maybe_parallel<
        A: Send,
        B: Send,
        E: Send,
        F: Fn(A) -> Result<B, E> + Send + Sync,
    >(
        &self,
        input: Vec<A>,
        f: F,
    ) -> Result<Vec<B>, E> {
        if self.parallel_compilation {
            #[cfg(feature = "parallel-compilation")]
            return input
                .into_par_iter()
                .map(|a| f(a))
                .collect::<Result<Vec<B>, E>>();
        }

        // In case the parallel-compilation feature is disabled or the parallel_compilation config
        // was turned off dynamically fallback to the non-parallel version.
        input
            .into_iter()
            .map(|a| f(a))
            .collect::<Result<Vec<B>, E>>()
    }
}

impl Hash for Compiler {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        let Compiler {
            compiler,
            tunables,
            features,
            parallel_compilation: _,
        } = self;

        compiler.triple().hash(hasher);
        compiler
            .flags()
            .into_iter()
            .collect::<BTreeMap<_, _>>()
            .hash(hasher);
        compiler
            .isa_flags()
            .into_iter()
            .collect::<BTreeMap<_, _>>()
            .hash(hasher);
        tunables.hash(hasher);
        features.hash(hasher);

        // Catch accidental bugs of reusing across crate versions.
        env!("CARGO_PKG_VERSION").hash(hasher);
    }
}
