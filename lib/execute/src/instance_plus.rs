use action::{ActionError, ActionOutcome, RuntimeValue};
use cranelift_codegen::{ir, isa};
use cranelift_entity::{BoxedSlice, PrimaryMap};
use cranelift_wasm::DefinedFuncIndex;
use jit_code::JITCode;
use link::link_module;
use resolver::Resolver;
use std::boxed::Box;
use std::cmp::max;
use std::rc::Rc;
use std::slice;
use std::string::String;
use std::vec::Vec;
use std::{mem, ptr};
use target_tunables::target_tunables;
use trampoline_park::TrampolinePark;
use wasmtime_environ::{
    compile_module, Compilation, CompileError, DataInitializer, Module, ModuleEnvironment,
};
use wasmtime_runtime::{
    wasmtime_call_trampoline, Export, Imports, Instance, InstantiationError, VMFunctionBody,
};

/// `InstancePlus` holds an `Instance` and adds support for performing actions
/// such as the "invoke" command in wast.
///
/// TODO: Think of a better name.
#[derive(Debug)]
pub struct InstancePlus {
    /// The contained instance.
    pub instance: Box<Instance>,

    /// Trampolines for calling into JIT code.
    trampolines: TrampolinePark,
}

impl InstancePlus {
    /// Create a new `InstancePlus` by compiling the wasm module in `data` and instatiating it.
    pub fn new(
        jit_code: &mut JITCode,
        isa: &isa::TargetIsa,
        data: &[u8],
        resolver: &mut Resolver,
    ) -> Result<Self, ActionError> {
        let mut module = Module::new();
        let tunables = target_tunables(isa.triple());

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

        let allocated_functions = allocate_functions(jit_code, compilation).map_err(|message| {
            ActionError::Instantiate(InstantiationError::Resource(format!(
                "failed to allocate memory for functions: {}",
                message
            )))
        })?;

        let imports = link_module(&module, &allocated_functions, relocations, resolver)
            .map_err(ActionError::Link)?;

        // Gather up the pointers to the compiled functions.
        let finished_functions: BoxedSlice<DefinedFuncIndex, *const VMFunctionBody> =
            allocated_functions
                .into_iter()
                .map(|(_index, allocated)| {
                    let fatptr: *const [VMFunctionBody] = *allocated;
                    fatptr as *const VMFunctionBody
                })
                .collect::<PrimaryMap<_, _>>()
                .into_boxed_slice();

        // Make all code compiled thus far executable.
        jit_code.publish();

        Self::with_parts(
            Rc::new(module),
            finished_functions,
            imports,
            lazy_data_initializers,
        )
    }

    /// Construct a new `InstancePlus` from the parts needed to produce an `Instance`.
    pub fn with_parts(
        module: Rc<Module>,
        finished_functions: BoxedSlice<DefinedFuncIndex, *const VMFunctionBody>,
        imports: Imports,
        data_initializers: Vec<DataInitializer>,
    ) -> Result<Self, ActionError> {
        let instance = Instance::new(module, finished_functions, imports, data_initializers)
            .map_err(ActionError::Instantiate)?;

        Ok(Self::with_instance(instance))
    }

    /// Construct a new `InstancePlus` from an existing instance.
    pub fn with_instance(instance: Box<Instance>) -> Self {
        Self {
            instance,
            trampolines: TrampolinePark::new(),
        }
    }

    /// Invoke a function in this `Instance` identified by an export name.
    pub fn invoke(
        &mut self,
        jit_code: &mut JITCode,
        isa: &isa::TargetIsa,
        function_name: &str,
        args: &[RuntimeValue],
    ) -> Result<ActionOutcome, ActionError> {
        let (address, signature, callee_vmctx) = match self.instance.lookup(function_name) {
            Some(Export::Function {
                address,
                signature,
                vmctx,
            }) => (address, signature, vmctx),
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

        for (index, value) in args.iter().enumerate() {
            assert_eq!(value.value_type(), signature.params[index].value_type);
        }

        // TODO: Support values larger than u64. And pack the values into memory
        // instead of just using fixed-sized slots.
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

        // Get the trampoline to call for this function.
        let exec_code_buf = self
            .trampolines
            .get(jit_code, isa, address, &signature, value_size)?;

        // Make all JIT code produced thus far executable.
        jit_code.publish();

        // Call the trampoline.
        if let Err(message) = unsafe {
            wasmtime_call_trampoline(
                exec_code_buf,
                values_vec.as_mut_ptr() as *mut u8,
                callee_vmctx,
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

    /// Returns a slice of the contents of allocated linear memory.
    pub fn inspect_memory(
        &self,
        memory_name: &str,
        start: usize,
        len: usize,
    ) -> Result<&[u8], ActionError> {
        let address = match unsafe { self.instance.lookup_immutable(memory_name) } {
            Some(Export::Memory {
                address,
                memory: _memory,
                vmctx: _vmctx,
            }) => address,
            Some(_) => {
                return Err(ActionError::Kind(format!(
                    "exported item \"{}\" is not a linear memory",
                    memory_name
                )))
            }
            None => {
                return Err(ActionError::Field(format!(
                    "no export named \"{}\"",
                    memory_name
                )))
            }
        };

        Ok(unsafe {
            let memory_def = &*address;
            &slice::from_raw_parts(memory_def.base, memory_def.current_length)[start..start + len]
        })
    }

    /// Read a global in this `Instance` identified by an export name.
    pub fn get(&self, global_name: &str) -> Result<RuntimeValue, ActionError> {
        let (address, global) = match unsafe { self.instance.lookup_immutable(global_name) } {
            Some(Export::Global { address, global }) => (address, global),
            Some(_) => {
                return Err(ActionError::Kind(format!(
                    "exported item \"{}\" is not a global variable",
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

        unsafe {
            let global_def = &*address;
            Ok(match global.ty {
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
            })
        }
    }
}

fn allocate_functions(
    jit_code: &mut JITCode,
    compilation: Compilation,
) -> Result<PrimaryMap<DefinedFuncIndex, *mut [VMFunctionBody]>, String> {
    let mut result = PrimaryMap::with_capacity(compilation.functions.len());
    for (_, body) in compilation.functions.into_iter() {
        let fatptr: *mut [VMFunctionBody] = jit_code.allocate_copy_of_byte_slice(body)?;
        result.push(fatptr);
    }
    Ok(result)
}
