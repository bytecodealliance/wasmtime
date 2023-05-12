use crate::code::CodeObject;
use crate::module::ModuleFunctionIndices;
use crate::signatures::SignatureCollection;
use crate::{Engine, Module};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::fs;
use std::mem;
use std::path::Path;
use std::ptr::NonNull;
use std::sync::Arc;
use wasmtime_environ::component::{
    AllCallFunc, ComponentTypes, GlobalInitializer, LoweredIndex, RuntimeAlwaysTrapIndex,
    RuntimeTranscoderIndex, StaticModuleIndex, Translator,
};
use wasmtime_environ::{FunctionLoc, ObjectKind, PrimaryMap, ScopeVec};
use wasmtime_jit::{CodeMemory, CompiledModuleInfo};
use wasmtime_runtime::{
    MmapVec, VMArrayCallFunction, VMFunctionBody, VMNativeCallFunction, VMWasmCallFunction,
};

/// A compiled WebAssembly Component.
//
// FIXME: need to write more docs here.
#[derive(Clone)]
pub struct Component {
    inner: Arc<ComponentInner>,
}

struct ComponentInner {
    /// Core wasm modules that the component defined internally, indexed by the
    /// compile-time-assigned `ModuleUpvarIndex`.
    static_modules: PrimaryMap<StaticModuleIndex, Module>,

    /// Code-related information such as the compiled artifact, type
    /// information, etc.
    ///
    /// Note that the `Arc` here is used to share this allocation with internal
    /// modules.
    code: Arc<CodeObject>,

    /// Metadata produced during compilation.
    info: CompiledComponentInfo,
}

#[derive(Serialize, Deserialize)]
struct CompiledComponentInfo {
    /// Type information calculated during translation about this component.
    component: wasmtime_environ::component::Component,

    /// Where lowered function trampolines are located within the `text`
    /// section of `code_memory`.
    ///
    /// These are the
    ///
    /// 1. Wasm-call,
    /// 2. array-call, and
    /// 3. native-call
    ///
    /// function pointers that end up in a `VMFuncRef` for each
    /// lowering.
    lowerings: PrimaryMap<LoweredIndex, AllCallFunc<FunctionLoc>>,

    /// Where the "always trap" functions are located within the `text` section
    /// of `code_memory`.
    ///
    /// These functions are "degenerate functions" here solely to implement
    /// functions that are `canon lift`'d then immediately `canon lower`'d. The
    /// `u32` value here is the offset of the trap instruction from the start fo
    /// the function.
    always_trap: PrimaryMap<RuntimeAlwaysTrapIndex, AllCallFunc<FunctionLoc>>,

    /// Where all the cranelift-generated transcode functions are located in the
    /// compiled image of this component.
    transcoders: PrimaryMap<RuntimeTranscoderIndex, AllCallFunc<FunctionLoc>>,
}

pub(crate) struct AllCallFuncPointers {
    pub wasm_call: NonNull<VMWasmCallFunction>,
    pub array_call: VMArrayCallFunction,
    pub native_call: NonNull<VMNativeCallFunction>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct ComponentArtifacts {
    info: CompiledComponentInfo,
    types: ComponentTypes,
    static_modules: PrimaryMap<StaticModuleIndex, CompiledModuleInfo>,
}

impl Component {
    /// Compiles a new WebAssembly component from the in-memory wasm image
    /// provided.
    //
    // FIXME: need to write more docs here.
    #[cfg(any(feature = "cranelift", feature = "winch"))]
    #[cfg_attr(nightlydoc, doc(cfg(any(feature = "cranelift", feature = "winch"))))]
    pub fn new(engine: &Engine, bytes: impl AsRef<[u8]>) -> Result<Component> {
        let bytes = bytes.as_ref();
        #[cfg(feature = "wat")]
        let bytes = wat::parse_bytes(bytes)?;
        Component::from_binary(engine, &bytes)
    }

    /// Compiles a new WebAssembly component from a wasm file on disk pointed to
    /// by `file`.
    //
    // FIXME: need to write more docs here.
    #[cfg(any(feature = "cranelift", feature = "winch"))]
    #[cfg_attr(nightlydoc, doc(cfg(any(feature = "cranelift", feature = "winch"))))]
    pub fn from_file(engine: &Engine, file: impl AsRef<Path>) -> Result<Component> {
        match Self::new(
            engine,
            &fs::read(&file).with_context(|| "failed to read input file")?,
        ) {
            Ok(m) => Ok(m),
            Err(e) => {
                cfg_if::cfg_if! {
                    if #[cfg(feature = "wat")] {
                        let mut e = e.downcast::<wat::Error>()?;
                        e.set_path(file);
                        bail!(e)
                    } else {
                        Err(e)
                    }
                }
            }
        }
    }

    /// Compiles a new WebAssembly component from the in-memory wasm image
    /// provided.
    //
    // FIXME: need to write more docs here.
    #[cfg(any(feature = "cranelift", feature = "winch"))]
    #[cfg_attr(nightlydoc, doc(cfg(any(feature = "cranelift", feature = "winch"))))]
    pub fn from_binary(engine: &Engine, binary: &[u8]) -> Result<Component> {
        engine
            .check_compatible_with_native_host()
            .context("compilation settings are not compatible with the native host")?;

        let (mmap, artifacts) = Component::build_artifacts(engine, binary)?;
        let mut code_memory = CodeMemory::new(mmap)?;
        code_memory.publish()?;
        Component::from_parts(engine, Arc::new(code_memory), Some(artifacts))
    }

    /// Same as [`Module::deserialize`], but for components.
    ///
    /// Note that the file referenced here must contain contents previously
    /// produced by [`Engine::precompile_component`] or
    /// [`Component::serialize`].
    ///
    /// For more information see the [`Module::deserialize`] method.
    ///
    /// [`Module::deserialize`]: crate::Module::deserialize
    pub unsafe fn deserialize(engine: &Engine, bytes: impl AsRef<[u8]>) -> Result<Component> {
        let code = engine.load_code_bytes(bytes.as_ref(), ObjectKind::Component)?;
        Component::from_parts(engine, code, None)
    }

    /// Same as [`Module::deserialize_file`], but for components.
    ///
    /// For more information see the [`Component::deserialize`] and
    /// [`Module::deserialize_file`] methods.
    ///
    /// [`Module::deserialize_file`]: crate::Module::deserialize_file
    pub unsafe fn deserialize_file(engine: &Engine, path: impl AsRef<Path>) -> Result<Component> {
        let code = engine.load_code_file(path.as_ref(), ObjectKind::Component)?;
        Component::from_parts(engine, code, None)
    }

    /// Performs the compilation phase for a component, translating and
    /// validating the provided wasm binary to machine code.
    ///
    /// This method will compile all nested core wasm binaries in addition to
    /// any necessary extra functions required for operation with components.
    /// The output artifact here is the serialized object file contained within
    /// an owned mmap along with metadata about the compilation itself.
    #[cfg(any(feature = "cranelift", feature = "winch"))]
    pub(crate) fn build_artifacts(
        engine: &Engine,
        binary: &[u8],
    ) -> Result<(MmapVec, ComponentArtifacts)> {
        let tunables = &engine.config().tunables;
        let compiler = engine.compiler();

        let scope = ScopeVec::new();
        let mut validator =
            wasmparser::Validator::new_with_features(engine.config().features.clone());
        let mut types = Default::default();
        let (component, mut modules) =
            Translator::new(tunables, &mut validator, &mut types, &scope)
                .translate(binary)
                .context("failed to parse WebAssembly module")?;
        let types = types.finish();

        // Compile all core wasm modules, in parallel, which will internally
        // compile all their functions in parallel as well.
        let compilations = engine.run_maybe_parallel(modules.values_mut().collect(), |module| {
            Module::compile_functions(engine, module, types.module_types())
        })?;

        let mut compiled_funcs = vec![];
        let wasm_to_native_trampoline_indices = Module::compile_wasm_to_native_trampolines(
            engine,
            modules.values().as_slice(),
            types.module_types(),
            &mut compiled_funcs,
        )?;

        let mut indices = vec![];
        for ((i, translation), compilation) in modules.into_iter().zip(compilations) {
            let prefix = format!("wasm_{}_", i.as_u32());
            indices.push((
                compiled_funcs.len(),
                ModuleFunctionIndices::new(translation, compilation, &prefix, &mut compiled_funcs),
            ));
        }

        // Compile all transcoders required which adapt from a
        // core-wasm-specific ABI (e.g. 32 or 64-bit) into the host transcoder
        // ABI through an indirect libcall.
        let transcoders = component
            .initializers
            .iter()
            .filter_map(|init| match init {
                GlobalInitializer::Transcoder(i) => Some(i),
                _ => None,
            })
            .collect();
        let transcoder_indices = flatten_all_calls(
            &mut compiled_funcs,
            engine.run_maybe_parallel(transcoders, |info| -> Result<_> {
                Ok((
                    info.symbol_name(),
                    compiler
                        .component_compiler()
                        .compile_transcoder(&component, info, &types)?,
                ))
            })?,
        );

        // Compile all "always trap" functions which are small typed shims that
        // exits to solely trap immediately for components.
        let always_trap = component
            .initializers
            .iter()
            .filter_map(|init| match init {
                GlobalInitializer::AlwaysTrap(i) => Some(i),
                _ => None,
            })
            .collect();
        let always_trap_indices = flatten_all_calls(
            &mut compiled_funcs,
            engine.run_maybe_parallel(always_trap, |info| -> Result<_> {
                Ok((
                    info.symbol_name(),
                    compiler
                        .component_compiler()
                        .compile_always_trap(&types[info.canonical_abi])?,
                ))
            })?,
        );

        // Compile all "lowerings" which are adapters that go from core wasm
        // into the host which will process the canonical ABI.
        let lowerings = component
            .initializers
            .iter()
            .filter_map(|init| match init {
                GlobalInitializer::LowerImport(i) => Some(i),
                _ => None,
            })
            .collect();
        let lowering_indices = flatten_all_calls(
            &mut compiled_funcs,
            engine.run_maybe_parallel(lowerings, |lowering| -> Result<_> {
                Ok((
                    lowering.symbol_name(),
                    compiler
                        .component_compiler()
                        .compile_lowered_trampoline(&component, lowering, &types)?,
                ))
            })?,
        );

        let mut object = compiler.object(ObjectKind::Component)?;
        let locs = compiler.append_code(
            &mut object,
            &compiled_funcs,
            tunables,
            &|caller_index, callee_index| {
                // Find the index of the module that contains the function we are calling.
                let module_index = indices.partition_point(|(i, _)| *i <= caller_index) - 1;
                indices[module_index].1.resolve_reloc(callee_index)
            },
        )?;
        engine.append_compiler_info(&mut object);
        engine.append_bti(&mut object);

        // Convert all `ModuleTranslation` instances into `CompiledModuleInfo`
        // through an `ObjectBuilder` here. This is then used to create the
        // final `mmap` which is the final compilation artifact.
        let mut builder = wasmtime_jit::ObjectBuilder::new(object, tunables);
        let mut static_modules = PrimaryMap::new();
        for (_, module_indices) in indices {
            let info = module_indices.append_to_object(
                &locs,
                &wasm_to_native_trampoline_indices,
                &mut builder,
            )?;
            static_modules.push(info);
        }

        let info = CompiledComponentInfo {
            component,
            always_trap: always_trap_indices
                .into_iter()
                .map(|x| x.map(|i| locs[i].1))
                .collect(),
            lowerings: lowering_indices
                .into_iter()
                .map(|x| x.map(|i| locs[i].1))
                .collect(),
            transcoders: transcoder_indices
                .into_iter()
                .map(|x| x.map(|i| locs[i].1))
                .collect(),
        };
        let artifacts = ComponentArtifacts {
            info,
            types,
            static_modules,
        };
        builder.serialize_info(&artifacts);

        let mmap = builder.finish()?;
        Ok((mmap, artifacts))
    }

    /// Final assembly step for a component from its in-memory representation.
    ///
    /// If the `artifacts` are specified as `None` here then they will be
    /// deserialized from `code_memory`.
    fn from_parts(
        engine: &Engine,
        code_memory: Arc<CodeMemory>,
        artifacts: Option<ComponentArtifacts>,
    ) -> Result<Component> {
        let ComponentArtifacts {
            info,
            types,
            static_modules,
        } = match artifacts {
            Some(artifacts) => artifacts,
            None => bincode::deserialize(code_memory.wasmtime_info())?,
        };

        // Create a signature registration with the `Engine` for all trampolines
        // and core wasm types found within this component, both for the
        // component and for all included core wasm modules.
        let signatures =
            SignatureCollection::new_for_module(engine.signatures(), types.module_types());

        // Assemble the `CodeObject` artifact which is shared by all core wasm
        // modules as well as the final component.
        let types = Arc::new(types);
        let code = Arc::new(CodeObject::new(code_memory, signatures, types.into()));

        // Convert all information about static core wasm modules into actual
        // `Module` instances by converting each `CompiledModuleInfo`, the
        // `types` type information, and the code memory to a runtime object.
        let static_modules = static_modules
            .into_iter()
            .map(|(_, info)| Module::from_parts_raw(engine, code.clone(), info, false))
            .collect::<Result<_>>()?;

        Ok(Component {
            inner: Arc::new(ComponentInner {
                static_modules,
                code,
                info,
            }),
        })
    }

    pub(crate) fn env_component(&self) -> &wasmtime_environ::component::Component {
        &self.inner.info.component
    }

    pub(crate) fn static_module(&self, idx: StaticModuleIndex) -> &Module {
        &self.inner.static_modules[idx]
    }

    pub(crate) fn types(&self) -> &Arc<ComponentTypes> {
        match self.inner.code.types() {
            crate::code::Types::Component(types) => types,
            // The only creator of a `Component` is itself which uses the other
            // variant, so this shouldn't be possible.
            crate::code::Types::Module(_) => unreachable!(),
        }
    }

    pub(crate) fn signatures(&self) -> &SignatureCollection {
        self.inner.code.signatures()
    }

    pub(crate) fn text(&self) -> &[u8] {
        self.inner.code.code_memory().text()
    }

    pub(crate) fn lowering_ptrs(&self, index: LoweredIndex) -> AllCallFuncPointers {
        let AllCallFunc {
            wasm_call,
            array_call,
            native_call,
        } = &self.inner.info.lowerings[index];
        AllCallFuncPointers {
            wasm_call: self.func(wasm_call).cast(),
            array_call: unsafe {
                mem::transmute::<NonNull<VMFunctionBody>, VMArrayCallFunction>(
                    self.func(array_call),
                )
            },
            native_call: self.func(native_call).cast(),
        }
    }

    pub(crate) fn always_trap_ptrs(&self, index: RuntimeAlwaysTrapIndex) -> AllCallFuncPointers {
        let AllCallFunc {
            wasm_call,
            array_call,
            native_call,
        } = &self.inner.info.always_trap[index];
        AllCallFuncPointers {
            wasm_call: self.func(wasm_call).cast(),
            array_call: unsafe {
                mem::transmute::<NonNull<VMFunctionBody>, VMArrayCallFunction>(
                    self.func(array_call),
                )
            },
            native_call: self.func(native_call).cast(),
        }
    }

    pub(crate) fn transcoder_ptrs(&self, index: RuntimeTranscoderIndex) -> AllCallFuncPointers {
        let AllCallFunc {
            wasm_call,
            array_call,
            native_call,
        } = &self.inner.info.transcoders[index];
        AllCallFuncPointers {
            wasm_call: self.func(wasm_call).cast(),
            array_call: unsafe {
                mem::transmute::<NonNull<VMFunctionBody>, VMArrayCallFunction>(
                    self.func(array_call),
                )
            },
            native_call: self.func(native_call).cast(),
        }
    }

    fn func(&self, loc: &FunctionLoc) -> NonNull<VMFunctionBody> {
        let text = self.text();
        let trampoline = &text[loc.start as usize..][..loc.length as usize];
        NonNull::new(trampoline.as_ptr() as *mut VMFunctionBody).unwrap()
    }

    pub(crate) fn code_object(&self) -> &Arc<CodeObject> {
        &self.inner.code
    }

    /// Same as [`Module::serialize`], except for a component.
    ///
    /// Note that the artifact produced here must be passed to
    /// [`Component::deserialize`] and is not compatible for use with
    /// [`Module`].
    ///
    /// [`Module::serialize`]: crate::Module::serialize
    /// [`Module`]: crate::Module
    pub fn serialize(&self) -> Result<Vec<u8>> {
        Ok(self.code_object().code_memory().mmap().to_vec())
    }
}

/// Flatten a list of grouped `AllCallFunc<Box<dyn Any + Send>>` into the flat
/// list of all compiled functions.
fn flatten_all_calls(
    compiled_funcs: &mut Vec<(String, Box<dyn Any + Send>)>,
    all_calls: Vec<(String, AllCallFunc<Box<dyn Any + Send>>)>,
) -> Vec<AllCallFunc<usize>> {
    compiled_funcs.reserve(3 * all_calls.len());

    all_calls
        .into_iter()
        .map(
            |(
                prefix,
                AllCallFunc {
                    wasm_call,
                    array_call,
                    native_call,
                },
            )| {
                let i = compiled_funcs.len();
                compiled_funcs.push((format!("{prefix}_wasm_call"), wasm_call));
                compiled_funcs.push((format!("{prefix}_array_call"), array_call));
                compiled_funcs.push((format!("{prefix}_native_call"), native_call));
                AllCallFunc {
                    wasm_call: i + 0,
                    array_call: i + 1,
                    native_call: i + 2,
                }
            },
        )
        .collect()
}
