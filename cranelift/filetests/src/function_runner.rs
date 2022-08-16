//! Provides functionality for compiling and running CLIF IR for `run` tests.
use anyhow::Result;
use core::mem;
use cranelift_codegen::data_value::DataValue;
use cranelift_codegen::ir::{condcodes::IntCC, Function, InstBuilder, InstImmBuilder, Signature};
use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::{ir, settings, CodegenError};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module, ModuleError};
use cranelift_native::builder_with_options;
use std::cmp::max;
use thiserror::Error;

/// Compile a single function.
///
/// Several Cranelift functions need the ability to run Cranelift IR (e.g. `test_run`); this
/// [SingleFunctionCompiler] provides a way for compiling Cranelift [Function]s to
/// `CompiledFunction`s and subsequently calling them through the use of a `Trampoline`. As its
/// name indicates, this compiler is limited: any functionality that requires knowledge of things
/// outside the [Function] will likely not work (e.g. global values, calls). For an example of this
/// "outside-of-function" functionality, see `cranelift_jit::backend::JITBackend`.
///
/// ```
/// use cranelift_filetests::SingleFunctionCompiler;
/// use cranelift_reader::parse_functions;
/// use cranelift_codegen::data_value::DataValue;
///
/// let code = "test run \n function %add(i32, i32) -> i32 {  block0(v0:i32, v1:i32):  v2 = iadd v0, v1  return v2 }".into();
/// let func = parse_functions(code).unwrap().into_iter().nth(0).unwrap();
/// let compiler = SingleFunctionCompiler::with_default_host_isa().unwrap();
/// let compiled_func = compiler.compile(func).unwrap();
///
/// let returned = compiled_func.call(&vec![DataValue::I32(2), DataValue::I32(40)]);
/// assert_eq!(vec![DataValue::I32(42)], returned);
/// ```
pub struct SingleFunctionCompiler {
    isa: Box<dyn TargetIsa>,
}

impl SingleFunctionCompiler {
    /// Build a [SingleFunctionCompiler] from a [TargetIsa]. For functions to be runnable on the
    /// host machine, this [TargetIsa] must match the host machine's ISA (see
    /// [SingleFunctionCompiler::with_host_isa]).
    pub fn new(isa: Box<dyn TargetIsa>) -> Self {
        Self { isa }
    }

    /// Build a [SingleFunctionCompiler] using the host machine's ISA and the passed flags.
    pub fn with_host_isa(flags: settings::Flags) -> Result<Self> {
        let builder =
            builder_with_options(true).expect("Unable to build a TargetIsa for the current host");
        let isa = builder.finish(flags)?;
        Ok(Self::new(isa))
    }

    /// Build a [SingleFunctionCompiler] using the host machine's ISA and the default flags for this
    /// ISA.
    pub fn with_default_host_isa() -> Result<Self> {
        let flags = settings::Flags::new(settings::builder());
        Self::with_host_isa(flags)
    }

    /// Compile the passed [Function] to a `CompiledFunction`. This function will:
    ///  - check that the default ISA calling convention is used (to ensure it can be called)
    ///  - compile the [Function]
    ///  - compile a `Trampoline` for the [Function]'s signature (or used a cached `Trampoline`;
    ///    this makes it possible to call functions when the signature is not known until runtime.
    pub fn compile(self, function: Function) -> Result<CompiledFunction, CompilationError> {
        let signature = function.signature.clone();
        if signature.call_conv != self.isa.default_call_conv() {
            return Err(CompilationError::InvalidTargetIsa);
        }

        let trampoline = make_trampoline(&signature, self.isa.as_ref());

        let builder = JITBuilder::with_isa(self.isa, cranelift_module::default_libcall_names());
        let mut module = JITModule::new(builder);
        let mut ctx = module.make_context();

        let name = function.name.to_string();
        let func_id = module.declare_function(&name, Linkage::Local, &function.signature)?;

        // Build and declare the trampoline in the module
        let trampoline_name = trampoline.name.to_string();
        let trampoline_id =
            module.declare_function(&trampoline_name, Linkage::Local, &trampoline.signature)?;

        // Define both functions
        let func_signature = function.signature.clone();
        ctx.func = function;
        module.define_function(func_id, &mut ctx)?;
        module.clear_context(&mut ctx);

        ctx.func = trampoline;
        module.define_function(trampoline_id, &mut ctx)?;
        module.clear_context(&mut ctx);

        // Finalize the functions which we just defined, which resolves any
        // outstanding relocations (patching in addresses, now that they're
        // available).
        module.finalize_definitions();

        Ok(CompiledFunction::new(
            module,
            func_signature,
            func_id,
            trampoline_id,
        ))
    }
}

/// Compilation Error when compiling a function.
#[derive(Error, Debug)]
pub enum CompilationError {
    /// This Target ISA is invalid for the current host.
    #[error("Cross-compilation not currently supported; use the host's default calling convention \
    or remove the specified calling convention in the function signature to use the host's default.")]
    InvalidTargetIsa,
    /// Cranelift codegen error.
    #[error("Cranelift codegen error")]
    CodegenError(#[from] CodegenError),
    /// Module Error
    #[error("Module error")]
    ModuleError(#[from] ModuleError),
    /// Memory mapping error.
    #[error("Memory mapping error")]
    IoError(#[from] std::io::Error),
}

/// Container for the compiled code of a [Function]. This wrapper allows users to call the compiled
/// function through the use of a trampoline.
///
/// ```
/// use cranelift_filetests::SingleFunctionCompiler;
/// use cranelift_reader::parse_functions;
/// use cranelift_codegen::data_value::DataValue;
///
/// let code = "test run \n function %add(i32, i32) -> i32 {  block0(v0:i32, v1:i32):  v2 = iadd v0, v1  return v2 }".into();
/// let func = parse_functions(code).unwrap().into_iter().nth(0).unwrap();
/// let compiler = SingleFunctionCompiler::with_default_host_isa().unwrap();
/// let compiled_func = compiler.compile(func).unwrap();
///
/// let returned = compiled_func.call(&vec![DataValue::I32(2), DataValue::I32(40)]);
/// assert_eq!(vec![DataValue::I32(42)], returned);
/// ```
pub struct CompiledFunction {
    /// We need to store this since it contains the underlying memory for the functions
    /// Store it in an [Option] so that we can later drop it.
    module: Option<JITModule>,
    signature: Signature,
    func_id: FuncId,
    trampoline_id: FuncId,
}

impl CompiledFunction {
    /// Build a new [CompiledFunction].
    pub fn new(
        module: JITModule,
        signature: Signature,
        func_id: FuncId,
        trampoline_id: FuncId,
    ) -> Self {
        Self {
            module: Some(module),
            signature,
            func_id,
            trampoline_id,
        }
    }

    /// Call the [CompiledFunction], passing in [DataValue]s using a compiled trampoline.
    pub fn call(&self, arguments: &[DataValue]) -> Vec<DataValue> {
        let mut values = UnboxedValues::make_arguments(arguments, &self.signature);
        let arguments_address = values.as_mut_ptr();

        let module = self.module.as_ref().unwrap();
        let function_ptr = module.get_finalized_function(self.func_id);
        let trampoline_ptr = module.get_finalized_function(self.trampoline_id);

        let callable_trampoline: fn(*const u8, *mut u128) -> () =
            unsafe { mem::transmute(trampoline_ptr) };
        callable_trampoline(function_ptr, arguments_address);

        values.collect_returns(&self.signature)
    }
}

impl Drop for CompiledFunction {
    fn drop(&mut self) {
        // Freeing the module's memory erases the compiled functions.
        // This should be safe since their pointers never leave this struct.
        unsafe { self.module.take().unwrap().free_memory() }
    }
}

/// A container for laying out the [ValueData]s in memory in a way that the [Trampoline] can
/// understand.
struct UnboxedValues(Vec<u128>);

impl UnboxedValues {
    /// The size in bytes of each slot location in the allocated [DataValue]s. Though [DataValue]s
    /// could be smaller than 16 bytes (e.g. `I16`), this simplifies the creation of the [DataValue]
    /// array and could be used to align the slots to the largest used [DataValue] (i.e. 128-bit
    /// vectors).
    const SLOT_SIZE: usize = 16;

    /// Build the arguments vector for passing the [DataValue]s into the [Trampoline]. The size of
    /// `u128` used here must match [Trampoline::SLOT_SIZE].
    pub fn make_arguments(arguments: &[DataValue], signature: &ir::Signature) -> Self {
        assert_eq!(arguments.len(), signature.params.len());
        let mut values_vec = vec![0; max(signature.params.len(), signature.returns.len())];

        // Store the argument values into `values_vec`.
        for ((arg, slot), param) in arguments.iter().zip(&mut values_vec).zip(&signature.params) {
            assert!(
                arg.ty() == param.value_type || arg.is_vector() || arg.is_bool(),
                "argument type mismatch: {} != {}",
                arg.ty(),
                param.value_type
            );
            unsafe {
                arg.write_value_to(slot);
            }
        }

        Self(values_vec)
    }

    /// Return a pointer to the underlying memory for passing to the trampoline.
    pub fn as_mut_ptr(&mut self) -> *mut u128 {
        self.0.as_mut_ptr()
    }

    /// Collect the returned [DataValue]s into a [Vec]. The size of `u128` used here must match
    /// [Trampoline::SLOT_SIZE].
    pub fn collect_returns(&self, signature: &ir::Signature) -> Vec<DataValue> {
        assert!(self.0.len() >= signature.returns.len());
        let mut returns = Vec::with_capacity(signature.returns.len());

        // Extract the returned values from this vector.
        for (slot, param) in self.0.iter().zip(&signature.returns) {
            let value = unsafe { DataValue::read_value_from(slot, param.value_type) };
            returns.push(value);
        }

        returns
    }
}

/// Build the Cranelift IR for moving the memory-allocated [DataValue]s to their correct location
/// (e.g. register, stack) prior to calling a [CompiledFunction]. The [Function] returned by
/// [make_trampoline] is compiled to a [Trampoline]. Note that this uses the [TargetIsa]'s default
/// calling convention so we must also check that the [CompiledFunction] has the same calling
/// convention (see [SingleFunctionCompiler::compile]).
fn make_trampoline(signature: &ir::Signature, isa: &dyn TargetIsa) -> Function {
    // Create the trampoline signature: (callee_address: pointer, values_vec: pointer) -> ()
    let pointer_type = isa.pointer_type();
    let mut wrapper_sig = ir::Signature::new(isa.frontend_config().default_call_conv);
    wrapper_sig.params.push(ir::AbiParam::new(pointer_type)); // Add the `callee_address` parameter.
    wrapper_sig.params.push(ir::AbiParam::new(pointer_type)); // Add the `values_vec` parameter.

    let mut func = ir::Function::with_name_signature(ir::UserFuncName::default(), wrapper_sig);

    // The trampoline has a single block filled with loads, one call to callee_address, and some loads.
    let mut builder_context = FunctionBuilderContext::new();
    let mut builder = FunctionBuilder::new(&mut func, &mut builder_context);
    let block0 = builder.create_block();
    builder.append_block_params_for_function_params(block0);
    builder.switch_to_block(block0);
    builder.seal_block(block0);

    // Extract the incoming SSA values.
    let (callee_value, values_vec_ptr_val) = {
        let params = builder.func.dfg.block_params(block0);
        (params[0], params[1])
    };

    // Load the argument values out of `values_vec`.
    let callee_args = signature
        .params
        .iter()
        .enumerate()
        .map(|(i, param)| {
            // Calculate the type to load from memory, using integers for booleans (no encodings).
            let ty = param.value_type.coerce_bools_to_ints();

            // We always store vector types in little-endian byte order as DataValue.
            let mut flags = ir::MemFlags::trusted();
            if param.value_type.is_vector() {
                flags.set_endianness(ir::Endianness::Little);
            }

            // Load the value.
            let loaded = builder.ins().load(
                ty,
                flags,
                values_vec_ptr_val,
                (i * UnboxedValues::SLOT_SIZE) as i32,
            );

            // For booleans, we want to type-convert the loaded integer into a boolean and ensure
            // that we are using the architecture's canonical boolean representation (presumably
            // comparison will emit this).
            if param.value_type.is_bool() {
                let b = builder.ins().icmp_imm(IntCC::NotEqual, loaded, 0);

                // icmp_imm always produces a `b1`, `bextend` it if we need a larger bool
                if param.value_type.bits() > 1 {
                    builder.ins().bextend(param.value_type, b)
                } else {
                    b
                }
            } else if param.value_type.is_bool_vector() {
                let zero_constant = builder.func.dfg.constants.insert(vec![0; 16].into());
                let zero_vec = builder.ins().vconst(ty, zero_constant);
                builder.ins().icmp(IntCC::NotEqual, loaded, zero_vec)
            } else {
                loaded
            }
        })
        .collect::<Vec<_>>();

    // Call the passed function.
    let new_sig = builder.import_signature(signature.clone());
    let call = builder
        .ins()
        .call_indirect(new_sig, callee_value, &callee_args);

    // Store the return values into `values_vec`.
    let results = builder.func.dfg.inst_results(call).to_vec();
    for ((i, value), param) in results.iter().enumerate().zip(&signature.returns) {
        // Before storing return values, we convert booleans to their integer representation.
        let value = if param.value_type.lane_type().is_bool() {
            let ty = param.value_type.lane_type().as_int();
            builder.ins().bint(ty, *value)
        } else {
            *value
        };
        // We always store vector types in little-endian byte order as DataValue.
        let mut flags = ir::MemFlags::trusted();
        if param.value_type.is_vector() {
            flags.set_endianness(ir::Endianness::Little);
        }
        // Store the value.
        builder.ins().store(
            flags,
            value,
            values_vec_ptr_val,
            (i * UnboxedValues::SLOT_SIZE) as i32,
        );
    }

    builder.ins().return_(&[]);
    builder.finalize();

    func
}

#[cfg(test)]
mod test {
    use super::*;
    use cranelift_reader::{parse_functions, parse_test, ParseOptions};

    fn parse(code: &str) -> Function {
        parse_functions(code).unwrap().into_iter().nth(0).unwrap()
    }

    #[test]
    fn nop() {
        let code = String::from(
            "
            test run
            function %test() -> b8 {
            block0:
                nop
                v1 = bconst.b8 true
                return v1
            }",
        );

        // extract function
        let test_file = parse_test(code.as_str(), ParseOptions::default()).unwrap();
        assert_eq!(1, test_file.functions.len());
        let function = test_file.functions[0].0.clone();

        // execute function
        let compiler = SingleFunctionCompiler::with_default_host_isa().unwrap();
        let compiled_function = compiler.compile(function).unwrap();
        let returned = compiled_function.call(&[]);
        assert_eq!(returned, vec![DataValue::B(true)])
    }

    #[test]
    fn trampolines() {
        let function = parse(
            "
            function %test(f32, i8, i64x2, b1) -> f32x4, b64 {
            block0(v0: f32, v1: i8, v2: i64x2, v3: b1):
                v4 = vconst.f32x4 [0x0.1 0x0.2 0x0.3 0x0.4]
                v5 = bconst.b64 true
                return v4, v5
            }",
        );

        let compiler = SingleFunctionCompiler::with_default_host_isa().unwrap();
        let trampoline = make_trampoline(&function.signature, compiler.isa.as_ref());
        assert!(
            format!("{}", trampoline).ends_with(
                "sig0 = (f32, i8, i64x2, b1) -> f32x4, b64 fast

block0(v0: i64, v1: i64):
    v2 = load.f32 notrap aligned v1
    v3 = load.i8 notrap aligned v1+16
    v4 = load.i64x2 notrap aligned little v1+32
    v5 = load.i8 notrap aligned v1+48
    v6 = iconst.i8 0
    v7 = icmp ne v5, v6
    v8, v9 = call_indirect sig0, v0(v2, v3, v4, v7)
    store notrap aligned little v8, v1
    v10 = bint.i64 v9
    store notrap aligned v10, v1+16
    return
}
"
            ),
            "got:\n{}",
            trampoline
        );
    }
}
