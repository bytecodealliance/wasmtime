use crate::code::CodeObject;
use crate::signatures::SignatureCollection;
use crate::{Engine, Module};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::mem;
use std::path::Path;
use std::ptr::NonNull;
use std::sync::Arc;
use wasmtime_environ::component::{
    AllCallFunc, ComponentTypes, LoweredIndex, RuntimeAlwaysTrapIndex, RuntimeResourceDropIndex,
    RuntimeResourceNewIndex, RuntimeResourceRepIndex, RuntimeTranscoderIndex, StaticModuleIndex,
    Translator,
};
use wasmtime_environ::{FunctionLoc, ObjectKind, PrimaryMap, ScopeVec};
use wasmtime_jit::{CodeMemory, CompiledModuleInfo};
use wasmtime_runtime::component::ComponentRuntimeInfo;
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

    /// TODO
    resource_new: PrimaryMap<RuntimeResourceNewIndex, AllCallFunc<FunctionLoc>>,
    /// TODO
    resource_rep: PrimaryMap<RuntimeResourceRepIndex, AllCallFunc<FunctionLoc>>,
    /// TODO
    resource_drop: PrimaryMap<RuntimeResourceDropIndex, AllCallFunc<FunctionLoc>>,
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
        use crate::compiler::CompileInputs;

        let tunables = &engine.config().tunables;
        let compiler = engine.compiler();

        let scope = ScopeVec::new();
        let mut validator =
            wasmparser::Validator::new_with_features(engine.config().features.clone());
        let mut types = Default::default();
        let (component, mut module_translations) =
            Translator::new(tunables, &mut validator, &mut types, &scope)
                .translate(binary)
                .context("failed to parse WebAssembly module")?;
        let types = types.finish();

        let compile_inputs = CompileInputs::for_component(
            &types,
            &component,
            module_translations.iter_mut().map(|(i, translation)| {
                let functions = mem::take(&mut translation.function_body_inputs);
                (i, &*translation, functions)
            }),
        );
        let unlinked_compile_outputs = compile_inputs.compile(&engine)?;
        let (compiled_funcs, function_indices) = unlinked_compile_outputs.pre_link();

        let mut object = compiler.object(ObjectKind::Component)?;
        engine.append_compiler_info(&mut object);
        engine.append_bti(&mut object);

        let (mut object, compilation_artifacts) = function_indices.link_and_append_code(
            object,
            tunables,
            compiler,
            compiled_funcs,
            module_translations,
        )?;

        let info = CompiledComponentInfo {
            component,
            always_trap: compilation_artifacts.always_traps,
            lowerings: compilation_artifacts.lowerings,
            transcoders: compilation_artifacts.transcoders,
            resource_new: compilation_artifacts.resource_new,
            resource_rep: compilation_artifacts.resource_rep,
            resource_drop: compilation_artifacts.resource_drop,
        };
        let artifacts = ComponentArtifacts {
            info,
            types,
            static_modules: compilation_artifacts.modules,
        };
        object.serialize_info(&artifacts);

        let mmap = object.finish()?;
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
        self.inner.component_types()
    }

    pub(crate) fn signatures(&self) -> &SignatureCollection {
        self.inner.code.signatures()
    }

    pub(crate) fn text(&self) -> &[u8] {
        self.inner.code.code_memory().text()
    }

    fn all_call_func_ptrs(&self, func: &AllCallFunc<FunctionLoc>) -> AllCallFuncPointers {
        let AllCallFunc {
            wasm_call,
            array_call,
            native_call,
        } = func;
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

    pub(crate) fn lowering_ptrs(&self, index: LoweredIndex) -> AllCallFuncPointers {
        self.all_call_func_ptrs(&self.inner.info.lowerings[index])
    }

    pub(crate) fn always_trap_ptrs(&self, index: RuntimeAlwaysTrapIndex) -> AllCallFuncPointers {
        self.all_call_func_ptrs(&self.inner.info.always_trap[index])
    }

    pub(crate) fn transcoder_ptrs(&self, index: RuntimeTranscoderIndex) -> AllCallFuncPointers {
        self.all_call_func_ptrs(&self.inner.info.transcoders[index])
    }

    pub(crate) fn resource_new_ptrs(&self, index: RuntimeResourceNewIndex) -> AllCallFuncPointers {
        self.all_call_func_ptrs(&self.inner.info.resource_new[index])
    }

    pub(crate) fn resource_rep_ptrs(&self, index: RuntimeResourceRepIndex) -> AllCallFuncPointers {
        self.all_call_func_ptrs(&self.inner.info.resource_rep[index])
    }

    pub(crate) fn resource_drop_ptrs(
        &self,
        index: RuntimeResourceDropIndex,
    ) -> AllCallFuncPointers {
        self.all_call_func_ptrs(&self.inner.info.resource_drop[index])
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

    pub(crate) fn runtime_info(&self) -> Arc<dyn ComponentRuntimeInfo> {
        self.inner.clone()
    }
}

impl ComponentRuntimeInfo for ComponentInner {
    fn component(&self) -> &wasmtime_environ::component::Component {
        &self.info.component
    }

    fn component_types(&self) -> &Arc<ComponentTypes> {
        match self.code.types() {
            crate::code::Types::Component(types) => types,
            // The only creator of a `Component` is itself which uses the other
            // variant, so this shouldn't be possible.
            crate::code::Types::Module(_) => unreachable!(),
        }
    }
}
