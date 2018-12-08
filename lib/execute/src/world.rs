use action::{ActionError, ActionOutcome, RuntimeValue};
use code::Code;
use cranelift_codegen::ir::InstBuilder;
use cranelift_codegen::Context;
use cranelift_codegen::{binemit, ir, isa};
use cranelift_entity::{BoxedSlice, EntityRef, PrimaryMap};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_wasm::{
    DefinedFuncIndex, DefinedGlobalIndex, DefinedMemoryIndex, DefinedTableIndex, FuncIndex,
    GlobalIndex, MemoryIndex, TableIndex,
};
use export::Resolver;
use link::link_module;
use std::cmp::max;
use std::collections::HashMap;
use std::slice;
use std::string::String;
use std::vec::Vec;
use std::{mem, ptr};
use wasmtime_environ::{
    compile_module, Compilation, CompileError, Export, Module, ModuleEnvironment, RelocSink,
    Tunables,
};
use wasmtime_runtime::{
    wasmtime_call_trampoline, wasmtime_init_eager, wasmtime_init_finish, Instance, VMContext,
    VMFunctionBody, VMGlobalDefinition, VMGlobalImport, VMMemoryDefinition, VMMemoryImport,
    VMTableDefinition, VMTableImport,
};

/// A module, an instance of that module, and accompanying compilation artifacts.
///
/// TODO: Rename and reorganize this.
pub struct InstanceWorld {
    module: Module,
    instance: Instance,

    /// Pointers to functions in executable memory.
    finished_functions: BoxedSlice<DefinedFuncIndex, *const VMFunctionBody>,

    /// Trampolines for calling into JIT code.
    trampolines: TrampolinePark,
}

impl InstanceWorld {
    /// Create a new `InstanceWorld` by compiling the wasm module in `data` and instatiating it.
    ///
    /// `finished_functions` holds the function bodies
    /// which have been placed in executable memory and linked.
    pub fn new(
        code: &mut Code,
        isa: &isa::TargetIsa,
        data: &[u8],
        resolver: &mut Resolver,
    ) -> Result<Self, ActionError> {
        let mut module = Module::new();
        // TODO: Allow the tunables to be overridden.
        let tunables = Tunables::default();
        let (lazy_function_body_inputs, lazy_data_initializers) = {
            let environ = ModuleEnvironment::new(isa, &mut module, tunables);

            let translation = environ
                .translate(&data)
                .map_err(|error| ActionError::Compile(CompileError::Wasm(error)))?;

            (
                translation.lazy.function_body_inputs,
                translation.lazy.data_initializers,
            )
        };

        let (compilation, relocations) = compile_module(&module, &lazy_function_body_inputs, isa)
            .map_err(ActionError::Compile)?;

        let allocated_functions =
            allocate_functions(code, compilation).map_err(ActionError::Resource)?;

        let imports = link_module(&module, &allocated_functions, relocations, resolver)
            .map_err(ActionError::Link)?;

        let finished_functions: BoxedSlice<DefinedFuncIndex, *const VMFunctionBody> =
            allocated_functions
                .into_iter()
                .map(|(_index, allocated)| {
                    let fatptr: *const [VMFunctionBody] = *allocated;
                    fatptr as *const VMFunctionBody
                })
                .collect::<PrimaryMap<_, _>>()
                .into_boxed_slice();

        let instance = Instance::new(
            &module,
            &finished_functions,
            imports,
            &lazy_data_initializers,
        )
        .map_err(ActionError::Resource)?;

        let fn_builder_ctx = FunctionBuilderContext::new();

        let mut result = Self {
            module,
            instance,
            finished_functions,
            trampolines: TrampolinePark {
                memo: HashMap::new(),
                fn_builder_ctx,
            },
        };

        // The WebAssembly spec specifies that the start function is
        // invoked automatically at instantiation time.
        match result.invoke_start_function(code, isa)? {
            ActionOutcome::Returned { .. } => {}
            ActionOutcome::Trapped { message } => {
                // Instantiation fails if the start function traps.
                return Err(ActionError::Start(message));
            }
        }

        Ok(result)
    }

    fn get_imported_function(&self, index: FuncIndex) -> Option<*const VMFunctionBody> {
        if index.index() < self.module.imported_funcs.len() {
            Some(unsafe { self.instance.vmctx().imported_function(index) })
        } else {
            None
        }
    }

    // TODO: Add an accessor for table elements.
    #[allow(dead_code)]
    fn get_imported_table(&self, index: TableIndex) -> Option<&VMTableImport> {
        if index.index() < self.module.imported_tables.len() {
            Some(unsafe { self.instance.vmctx().imported_table(index) })
        } else {
            None
        }
    }

    fn get_imported_memory(&self, index: MemoryIndex) -> Option<&VMMemoryImport> {
        if index.index() < self.module.imported_memories.len() {
            Some(unsafe { self.instance.vmctx().imported_memory(index) })
        } else {
            None
        }
    }

    fn get_imported_global(&self, index: GlobalIndex) -> Option<&VMGlobalImport> {
        if index.index() < self.module.imported_globals.len() {
            Some(unsafe { self.instance.vmctx().imported_global(index) })
        } else {
            None
        }
    }

    fn get_finished_function(&self, index: DefinedFuncIndex) -> Option<*const VMFunctionBody> {
        self.finished_functions.get(index).cloned()
    }

    // TODO: Add an accessor for table elements.
    #[allow(dead_code)]
    fn get_defined_table(&self, index: DefinedTableIndex) -> Option<&VMTableDefinition> {
        if self.module.table_index(index).index() < self.module.table_plans.len() {
            Some(unsafe { self.instance.vmctx().table(index) })
        } else {
            None
        }
    }

    fn get_defined_memory(&self, index: DefinedMemoryIndex) -> Option<&VMMemoryDefinition> {
        if self.module.memory_index(index).index() < self.module.memory_plans.len() {
            Some(unsafe { self.instance.vmctx().memory(index) })
        } else {
            None
        }
    }

    fn get_defined_global(&self, index: DefinedGlobalIndex) -> Option<&VMGlobalDefinition> {
        if self.module.global_index(index).index() < self.module.globals.len() {
            Some(unsafe { self.instance.vmctx().global(index) })
        } else {
            None
        }
    }

    /// Invoke a function in this `InstanceWorld` by name.
    pub fn invoke(
        &mut self,
        code: &mut Code,
        isa: &isa::TargetIsa,
        function_name: &str,
        args: &[RuntimeValue],
    ) -> Result<ActionOutcome, ActionError> {
        let fn_index = match self.module.exports.get(function_name) {
            Some(Export::Function(index)) => *index,
            Some(_) => {
                return Err(ActionError::Kind(format!(
                    "exported item \"{}\" is not a function",
                    function_name
                )))
            }
            None => {
                return Err(ActionError::Field(format!(
                    "no export named \"{}\"",
                    function_name
                )))
            }
        };

        self.invoke_by_index(code, isa, fn_index, args)
    }

    /// Invoke the WebAssembly start function of the instance, if one is present.
    fn invoke_start_function(
        &mut self,
        code: &mut Code,
        isa: &isa::TargetIsa,
    ) -> Result<ActionOutcome, ActionError> {
        if let Some(start_index) = self.module.start_func {
            self.invoke_by_index(code, isa, start_index, &[])
        } else {
            // No start function, just return nothing.
            Ok(ActionOutcome::Returned { values: vec![] })
        }
    }

    /// Calls the given indexed function, passing its return values and returning
    /// its results.
    fn invoke_by_index(
        &mut self,
        code: &mut Code,
        isa: &isa::TargetIsa,
        fn_index: FuncIndex,
        args: &[RuntimeValue],
    ) -> Result<ActionOutcome, ActionError> {
        let callee_address = match self.module.defined_func_index(fn_index) {
            Some(def_fn_index) => self
                .get_finished_function(def_fn_index)
                .ok_or_else(|| ActionError::Index(def_fn_index.index() as u64))?,
            None => self
                .get_imported_function(fn_index)
                .ok_or_else(|| ActionError::Index(fn_index.index() as u64))?,
        };

        // Rather than writing inline assembly to jump to the code region, we use the fact that
        // the Rust ABI for calling a function with no arguments and no return values matches the one
        // of the generated code. Thanks to this, we can transmute the code region into a first-class
        // Rust function and call it.
        // Ensure that our signal handlers are ready for action.
        wasmtime_init_eager();
        wasmtime_init_finish(self.instance.vmctx_mut());

        let signature = &self.module.signatures[self.module.functions[fn_index]];
        let vmctx: *mut VMContext = self.instance.vmctx_mut();

        for (index, value) in args.iter().enumerate() {
            assert_eq!(value.value_type(), signature.params[index].value_type);
        }

        // TODO: Support values larger than u64.
        let mut values_vec: Vec<u64> = Vec::new();
        let value_size = mem::size_of::<u64>();
        values_vec.resize(max(signature.params.len(), signature.returns.len()), 0u64);

        // Store the argument values into `values_vec`.
        for (index, arg) in args.iter().enumerate() {
            unsafe {
                let ptr = values_vec.as_mut_ptr().add(index);

                match arg {
                    RuntimeValue::I32(x) => ptr::write(ptr as *mut i32, *x),
                    RuntimeValue::I64(x) => ptr::write(ptr as *mut i64, *x),
                    RuntimeValue::F32(x) => ptr::write(ptr as *mut u32, *x),
                    RuntimeValue::F64(x) => ptr::write(ptr as *mut u64, *x),
                }
            }
        }

        // Store the vmctx value into `values_vec`.
        unsafe {
            let ptr = values_vec.as_mut_ptr().add(args.len());
            ptr::write(ptr as *mut usize, vmctx as usize)
        }

        // Get the trampoline to call for this function.
        let exec_code_buf =
            self.trampolines
                .get(code, isa, callee_address, &signature, value_size)?;

        // Make all JIT code produced thus far executable.
        code.publish();

        // Call the trampoline.
        if let Err(message) = unsafe {
            wasmtime_call_trampoline(
                exec_code_buf,
                values_vec.as_mut_ptr() as *mut u8,
                self.instance.vmctx_mut(),
            )
        } {
            return Ok(ActionOutcome::Trapped { message });
        }

        // Load the return values out of `values_vec`.
        let values = signature
            .returns
            .iter()
            .enumerate()
            .map(|(index, abi_param)| unsafe {
                let ptr = values_vec.as_ptr().add(index);

                match abi_param.value_type {
                    ir::types::I32 => RuntimeValue::I32(ptr::read(ptr as *const i32)),
                    ir::types::I64 => RuntimeValue::I64(ptr::read(ptr as *const i64)),
                    ir::types::F32 => RuntimeValue::F32(ptr::read(ptr as *const u32)),
                    ir::types::F64 => RuntimeValue::F64(ptr::read(ptr as *const u64)),
                    other => panic!("unsupported value type {:?}", other),
                }
            })
            .collect();

        Ok(ActionOutcome::Returned { values })
    }

    /// Read a global in this `InstanceWorld` by name.
    pub fn get(&self, global_name: &str) -> Result<RuntimeValue, ActionError> {
        let global_index = match self.module.exports.get(global_name) {
            Some(Export::Global(index)) => *index,
            Some(_) => {
                return Err(ActionError::Kind(format!(
                    "exported item \"{}\" is not a global",
                    global_name
                )))
            }
            None => {
                return Err(ActionError::Field(format!(
                    "no export named \"{}\"",
                    global_name
                )))
            }
        };

        self.get_by_index(global_index)
    }

    /// Reads the value of the indexed global variable in `module`.
    pub fn get_by_index(&self, global_index: GlobalIndex) -> Result<RuntimeValue, ActionError> {
        let global_address = match self.module.defined_global_index(global_index) {
            Some(def_global_index) => self
                .get_defined_global(def_global_index)
                .ok_or_else(|| ActionError::Index(def_global_index.index() as u64))?,
            None => {
                let from: *const VMGlobalDefinition = self
                    .get_imported_global(global_index)
                    .ok_or_else(|| ActionError::Index(global_index.index() as u64))?
                    .from;
                from
            }
        };
        let global_def = unsafe { &*global_address };

        unsafe {
            Ok(
                match self
                    .module
                    .globals
                    .get(global_index)
                    .ok_or_else(|| ActionError::Index(global_index.index() as u64))?
                    .ty
                {
                    ir::types::I32 => RuntimeValue::I32(*global_def.as_i32()),
                    ir::types::I64 => RuntimeValue::I64(*global_def.as_i64()),
                    ir::types::F32 => RuntimeValue::F32(*global_def.as_f32_bits()),
                    ir::types::F64 => RuntimeValue::F64(*global_def.as_f64_bits()),
                    other => {
                        return Err(ActionError::Type(format!(
                            "global with type {} not supported",
                            other
                        )))
                    }
                },
            )
        }
    }

    /// Returns a slice of the contents of allocated linear memory.
    pub fn inspect_memory(
        &self,
        memory_index: MemoryIndex,
        address: usize,
        len: usize,
    ) -> Result<&[u8], ActionError> {
        let memory_address = match self.module.defined_memory_index(memory_index) {
            Some(def_memory_index) => self
                .get_defined_memory(def_memory_index)
                .ok_or_else(|| ActionError::Index(def_memory_index.index() as u64))?,
            None => {
                let from: *const VMMemoryDefinition = self
                    .get_imported_memory(memory_index)
                    .ok_or_else(|| ActionError::Index(memory_index.index() as u64))?
                    .from;
                from
            }
        };
        let memory_def = unsafe { &*memory_address };

        Ok(unsafe {
            &slice::from_raw_parts(memory_def.base, memory_def.current_length)
                [address..address + len]
        })
    }
}

fn allocate_functions(
    code: &mut Code,
    compilation: Compilation,
) -> Result<PrimaryMap<DefinedFuncIndex, *mut [VMFunctionBody]>, String> {
    let mut result = PrimaryMap::with_capacity(compilation.functions.len());
    for (_, body) in compilation.functions.into_iter() {
        let fatptr: *mut [VMFunctionBody] = code.allocate_copy_of_byte_slice(body)?;
        result.push(fatptr);
    }
    Ok(result)
}

struct TrampolinePark {
    /// Memorized per-function trampolines.
    memo: HashMap<*const VMFunctionBody, *const VMFunctionBody>,

    /// The `FunctionBuilderContext`, shared between function compilations.
    fn_builder_ctx: FunctionBuilderContext,
}

impl TrampolinePark {
    fn get(
        &mut self,
        code: &mut Code,
        isa: &isa::TargetIsa,
        callee_address: *const VMFunctionBody,
        signature: &ir::Signature,
        value_size: usize,
    ) -> Result<*const VMFunctionBody, ActionError> {
        use std::collections::hash_map::Entry::{Occupied, Vacant};
        Ok(match self.memo.entry(callee_address) {
            Occupied(entry) => *entry.get(),
            Vacant(entry) => {
                let body = make_trampoline(
                    &mut self.fn_builder_ctx,
                    code,
                    isa,
                    callee_address,
                    signature,
                    value_size,
                )?;
                entry.insert(body);
                body
            }
        })
    }
}

fn make_trampoline(
    fn_builder_ctx: &mut FunctionBuilderContext,
    code: &mut Code,
    isa: &isa::TargetIsa,
    callee_address: *const VMFunctionBody,
    signature: &ir::Signature,
    value_size: usize,
) -> Result<*const VMFunctionBody, ActionError> {
    let pointer_type = isa.pointer_type();
    let mut wrapper_sig = ir::Signature::new(isa.frontend_config().default_call_conv);

    // Add the `values_vec` parameter.
    wrapper_sig.params.push(ir::AbiParam::new(pointer_type));
    // Add the `vmctx` parameter.
    wrapper_sig.params.push(ir::AbiParam::special(
        pointer_type,
        ir::ArgumentPurpose::VMContext,
    ));

    let mut context = Context::new();
    context.func = ir::Function::with_name_signature(ir::ExternalName::user(0, 0), wrapper_sig);

    {
        let mut builder = FunctionBuilder::new(&mut context.func, fn_builder_ctx);
        let block0 = builder.create_ebb();

        builder.append_ebb_params_for_function_params(block0);
        builder.switch_to_block(block0);
        builder.seal_block(block0);

        let mut callee_args = Vec::new();
        let pointer_type = isa.pointer_type();

        let (values_vec_ptr_val, vmctx_ptr_val) = {
            let params = builder.func.dfg.ebb_params(block0);
            (params[0], params[1])
        };

        // Load the argument values out of `values_vec`.
        let mflags = ir::MemFlags::trusted();
        for (i, r) in signature.params.iter().enumerate() {
            let value = match r.purpose {
                ir::ArgumentPurpose::Normal => builder.ins().load(
                    r.value_type,
                    mflags,
                    values_vec_ptr_val,
                    (i * value_size) as i32,
                ),
                ir::ArgumentPurpose::VMContext => vmctx_ptr_val,
                other => panic!("unsupported argument purpose {}", other),
            };
            callee_args.push(value);
        }

        let new_sig = builder.import_signature(signature.clone());

        // TODO: It's possible to make this a direct call. We just need Cranelift
        // to support functions declared with an immediate integer address.
        // ExternalName::Absolute(u64). Let's do it.
        let callee_value = builder.ins().iconst(pointer_type, callee_address as i64);
        let call = builder
            .ins()
            .call_indirect(new_sig, callee_value, &callee_args);

        let results = builder.func.dfg.inst_results(call).to_vec();

        // Store the return values into `values_vec`.
        let mflags = ir::MemFlags::trusted();
        for (i, r) in results.iter().enumerate() {
            builder
                .ins()
                .store(mflags, *r, values_vec_ptr_val, (i * value_size) as i32);
        }

        builder.ins().return_(&[]);
        builder.finalize()
    }

    let mut code_buf: Vec<u8> = Vec::new();
    let mut reloc_sink = RelocSink::new();
    let mut trap_sink = binemit::NullTrapSink {};
    context
        .compile_and_emit(isa, &mut code_buf, &mut reloc_sink, &mut trap_sink)
        .map_err(|error| ActionError::Compile(CompileError::Codegen(error)))?;
    assert!(reloc_sink.func_relocs.is_empty());

    Ok(code
        .allocate_copy_of_byte_slice(&code_buf)
        .map_err(ActionError::Resource)?
        .as_ptr())
}
