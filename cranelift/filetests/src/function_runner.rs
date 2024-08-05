//! Provides functionality for compiling and running CLIF IR for `run` tests.
use anyhow::{anyhow, Result};
use core::mem;
use cranelift_codegen::data_value::DataValue;
use cranelift_codegen::ir::{
    ExternalName, Function, InstBuilder, Signature, UserExternalName, UserFuncName,
};
use cranelift_codegen::isa::{OwnedTargetIsa, TargetIsa};
use cranelift_codegen::{ir, settings, CodegenError, Context};
use cranelift_control::ControlPlane;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module, ModuleError};
use cranelift_native::builder_with_options;
use cranelift_reader::TestFile;
use std::cmp::max;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use thiserror::Error;

const TESTFILE_NAMESPACE: u32 = 0;

/// Holds information about a previously defined function.
#[derive(Debug)]
struct DefinedFunction {
    /// This is the name that the function is internally known as.
    ///
    /// The JIT module does not support linking / calling [TestcaseName]'s, so
    /// we rename every function into a [UserExternalName].
    ///
    /// By doing this we also have to rename functions that previously were using a
    /// [UserFuncName], since they may now be in conflict after the renaming that
    /// occurred.
    new_name: UserExternalName,

    /// The function signature
    signature: ir::Signature,

    /// JIT [FuncId]
    func_id: FuncId,
}

/// Compile a test case.
///
/// Several Cranelift functions need the ability to run Cranelift IR (e.g. `test_run`); this
/// [TestFileCompiler] provides a way for compiling Cranelift [Function]s to
/// `CompiledFunction`s and subsequently calling them through the use of a `Trampoline`. As its
/// name indicates, this compiler is limited: any functionality that requires knowledge of things
/// outside the [Function] will likely not work (e.g. global values, calls). For an example of this
/// "outside-of-function" functionality, see `cranelift_jit::backend::JITBackend`.
///
/// ```
/// # let ctrl_plane = &mut Default::default();
/// use cranelift_filetests::TestFileCompiler;
/// use cranelift_reader::parse_functions;
/// use cranelift_codegen::data_value::DataValue;
///
/// let code = "test run \n function %add(i32, i32) -> i32 {  block0(v0:i32, v1:i32):  v2 = iadd v0, v1  return v2 }".into();
/// let func = parse_functions(code).unwrap().into_iter().nth(0).unwrap();
/// let mut compiler = TestFileCompiler::with_default_host_isa().unwrap();
/// compiler.declare_function(&func).unwrap();
/// compiler.define_function(func.clone(), ctrl_plane).unwrap();
/// compiler.create_trampoline_for_function(&func, ctrl_plane).unwrap();
/// let compiled = compiler.compile().unwrap();
/// let trampoline = compiled.get_trampoline(&func).unwrap();
///
/// let returned = trampoline.call(&vec![DataValue::I32(2), DataValue::I32(40)]);
/// assert_eq!(vec![DataValue::I32(42)], returned);
/// ```
pub struct TestFileCompiler {
    module: JITModule,
    ctx: Context,

    /// Holds info about the functions that have already been defined.
    /// Use look them up by their original [UserFuncName] since that's how the caller
    /// passes them to us.
    defined_functions: HashMap<UserFuncName, DefinedFunction>,

    /// We deduplicate trampolines by the signature of the function that they target.
    /// This map holds as a key the [Signature] of the target function, and as a value
    /// the [UserFuncName] of the trampoline for that [Signature].
    ///
    /// The trampoline is defined in `defined_functions` as any other regular function.
    trampolines: HashMap<Signature, UserFuncName>,
}

impl TestFileCompiler {
    /// Build a [TestFileCompiler] from a [TargetIsa]. For functions to be runnable on the
    /// host machine, this [TargetIsa] must match the host machine's ISA (see
    /// [TestFileCompiler::with_host_isa]).
    pub fn new(isa: OwnedTargetIsa) -> Self {
        let mut builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        let _ = &mut builder; // require mutability on all architectures
        #[cfg(target_arch = "x86_64")]
        {
            builder.symbol_lookup_fn(Box::new(|name| {
                if name == "__cranelift_x86_pshufb" {
                    Some(__cranelift_x86_pshufb as *const u8)
                } else {
                    None
                }
            }));
        }

        // On Unix platforms force `libm` to get linked into this executable
        // because tests that use libcalls rely on this library being present.
        // Without this it's been seen that when cross-compiled to riscv64 the
        // final binary doesn't link in `libm`.
        #[cfg(unix)]
        {
            extern "C" {
                fn ceilf(f: f32) -> f32;
            }
            let f = 1.2_f32;
            assert_eq!(f.ceil(), unsafe { ceilf(f) });
        }

        let module = JITModule::new(builder);
        let ctx = module.make_context();

        Self {
            module,
            ctx,
            defined_functions: HashMap::new(),
            trampolines: HashMap::new(),
        }
    }

    /// Build a [TestFileCompiler] using the host machine's ISA and the passed flags.
    pub fn with_host_isa(flags: settings::Flags) -> Result<Self> {
        let builder =
            builder_with_options(true).expect("Unable to build a TargetIsa for the current host");
        let isa = builder.finish(flags)?;
        Ok(Self::new(isa))
    }

    /// Build a [TestFileCompiler] using the host machine's ISA and the default flags for this
    /// ISA.
    pub fn with_default_host_isa() -> Result<Self> {
        let flags = settings::Flags::new(settings::builder());
        Self::with_host_isa(flags)
    }

    /// Declares and compiles all functions in `functions`. Additionally creates a trampoline for
    /// each one of them.
    pub fn add_functions(
        &mut self,
        functions: &[Function],
        ctrl_planes: Vec<ControlPlane>,
    ) -> Result<()> {
        // Declare all functions in the file, so that they may refer to each other.
        for func in functions {
            self.declare_function(func)?;
        }

        let ctrl_planes = ctrl_planes
            .into_iter()
            .chain(std::iter::repeat(ControlPlane::default()));

        // Define all functions and trampolines
        for (func, ref mut ctrl_plane) in functions.iter().zip(ctrl_planes) {
            self.define_function(func.clone(), ctrl_plane)?;
            self.create_trampoline_for_function(func, ctrl_plane)?;
        }

        Ok(())
    }

    /// Registers all functions in a [TestFile]. Additionally creates a trampoline for each one
    /// of them.
    pub fn add_testfile(&mut self, testfile: &TestFile) -> Result<()> {
        let functions = testfile
            .functions
            .iter()
            .map(|(f, _)| f)
            .cloned()
            .collect::<Vec<_>>();

        self.add_functions(&functions[..], Vec::new())?;
        Ok(())
    }

    /// Declares a function an registers it as a linkable and callable target internally
    pub fn declare_function(&mut self, func: &Function) -> Result<()> {
        let next_id = self.defined_functions.len() as u32;
        match self.defined_functions.entry(func.name.clone()) {
            Entry::Occupied(_) => {
                anyhow::bail!("Duplicate function with name {} found!", &func.name)
            }
            Entry::Vacant(v) => {
                let name = func.name.to_string();
                let func_id =
                    self.module
                        .declare_function(&name, Linkage::Local, &func.signature)?;

                v.insert(DefinedFunction {
                    new_name: UserExternalName::new(TESTFILE_NAMESPACE, next_id),
                    signature: func.signature.clone(),
                    func_id,
                });
            }
        };

        Ok(())
    }

    /// Renames the function to its new [UserExternalName], as well as any other function that
    /// it may reference.
    ///
    /// We have to do this since the JIT cannot link Testcase functions.
    fn apply_func_rename(
        &self,
        mut func: Function,
        defined_func: &DefinedFunction,
    ) -> Result<Function> {
        // First, rename the function
        let func_original_name = func.name;
        func.name = UserFuncName::User(defined_func.new_name.clone());

        // Rename any functions that it references
        // Do this in stages to appease the borrow checker
        let mut redefines = Vec::with_capacity(func.dfg.ext_funcs.len());
        for (ext_ref, ext_func) in &func.dfg.ext_funcs {
            let old_name = match &ext_func.name {
                ExternalName::TestCase(tc) => UserFuncName::Testcase(tc.clone()),
                ExternalName::User(username) => {
                    UserFuncName::User(func.params.user_named_funcs()[*username].clone())
                }
                // The other cases don't need renaming, so lets just continue...
                _ => continue,
            };

            let target_df = self.defined_functions.get(&old_name).ok_or(anyhow!(
                "Undeclared function {} is referenced by {}!",
                &old_name,
                &func_original_name
            ))?;

            redefines.push((ext_ref, target_df.new_name.clone()));
        }

        // Now register the redefines
        for (ext_ref, new_name) in redefines.into_iter() {
            // Register the new name in the func, so that we can get a reference to it.
            let new_name_ref = func.params.ensure_user_func_name(new_name);

            // Finally rename the ExtFunc
            func.dfg.ext_funcs[ext_ref].name = ExternalName::User(new_name_ref);
        }

        Ok(func)
    }

    /// Defines the body of a function
    pub fn define_function(&mut self, func: Function, ctrl_plane: &mut ControlPlane) -> Result<()> {
        let defined_func = self
            .defined_functions
            .get(&func.name)
            .ok_or(anyhow!("Undeclared function {} found!", &func.name))?;

        self.ctx.func = self.apply_func_rename(func, defined_func)?;
        self.module.define_function_with_control_plane(
            defined_func.func_id,
            &mut self.ctx,
            ctrl_plane,
        )?;
        self.module.clear_context(&mut self.ctx);
        Ok(())
    }

    /// Creates and registers a trampoline for a function if none exists.
    pub fn create_trampoline_for_function(
        &mut self,
        func: &Function,
        ctrl_plane: &mut ControlPlane,
    ) -> Result<()> {
        if !self.defined_functions.contains_key(&func.name) {
            anyhow::bail!("Undeclared function {} found!", &func.name);
        }

        // Check if a trampoline for this function signature already exists
        if self.trampolines.contains_key(&func.signature) {
            return Ok(());
        }

        // Create a trampoline and register it
        let name = UserFuncName::user(TESTFILE_NAMESPACE, self.defined_functions.len() as u32);
        let trampoline = make_trampoline(name.clone(), &func.signature, self.module.isa());

        self.declare_function(&trampoline)?;
        self.define_function(trampoline, ctrl_plane)?;

        self.trampolines.insert(func.signature.clone(), name);

        Ok(())
    }

    /// Finalize this TestFile and link all functions.
    pub fn compile(mut self) -> Result<CompiledTestFile, CompilationError> {
        // Finalize the functions which we just defined, which resolves any
        // outstanding relocations (patching in addresses, now that they're
        // available).
        self.module.finalize_definitions()?;

        Ok(CompiledTestFile {
            module: Some(self.module),
            defined_functions: self.defined_functions,
            trampolines: self.trampolines,
        })
    }
}

/// A finalized Test File
pub struct CompiledTestFile {
    /// We need to store [JITModule] since it contains the underlying memory for the functions.
    /// Store it in an [Option] so that we can later drop it.
    module: Option<JITModule>,

    /// Holds info about the functions that have been registered in `module`.
    /// See [TestFileCompiler] for more info.
    defined_functions: HashMap<UserFuncName, DefinedFunction>,

    /// Trampolines available in this [JITModule].
    /// See [TestFileCompiler] for more info.
    trampolines: HashMap<Signature, UserFuncName>,
}

impl CompiledTestFile {
    /// Return a trampoline for calling.
    ///
    /// Returns None if [TestFileCompiler::create_trampoline_for_function] wasn't called for this function.
    pub fn get_trampoline(&self, func: &Function) -> Option<Trampoline> {
        let defined_func = self.defined_functions.get(&func.name)?;
        let trampoline_id = self
            .trampolines
            .get(&func.signature)
            .and_then(|name| self.defined_functions.get(name))
            .map(|df| df.func_id)?;
        Some(Trampoline {
            module: self.module.as_ref()?,
            func_id: defined_func.func_id,
            func_signature: &defined_func.signature,
            trampoline_id,
        })
    }
}

impl Drop for CompiledTestFile {
    fn drop(&mut self) {
        // Freeing the module's memory erases the compiled functions.
        // This should be safe since their pointers never leave this struct.
        unsafe { self.module.take().unwrap().free_memory() }
    }
}

/// A callable trampoline
pub struct Trampoline<'a> {
    module: &'a JITModule,
    func_id: FuncId,
    func_signature: &'a Signature,
    trampoline_id: FuncId,
}

impl<'a> Trampoline<'a> {
    /// Call the target function of this trampoline, passing in [DataValue]s using a compiled trampoline.
    pub fn call(&self, arguments: &[DataValue]) -> Vec<DataValue> {
        let mut values = UnboxedValues::make_arguments(arguments, &self.func_signature);
        let arguments_address = values.as_mut_ptr();

        let function_ptr = self.module.get_finalized_function(self.func_id);
        let trampoline_ptr = self.module.get_finalized_function(self.trampoline_id);

        let callable_trampoline: fn(*const u8, *mut u128) -> () =
            unsafe { mem::transmute(trampoline_ptr) };
        callable_trampoline(function_ptr, arguments_address);

        values.collect_returns(&self.func_signature)
    }
}

/// Compilation Error when compiling a function.
#[derive(Error, Debug)]
pub enum CompilationError {
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
                arg.ty() == param.value_type || arg.is_vector(),
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
/// convention (see [TestFileCompiler::compile]).
fn make_trampoline(name: UserFuncName, signature: &ir::Signature, isa: &dyn TargetIsa) -> Function {
    // Create the trampoline signature: (callee_address: pointer, values_vec: pointer) -> ()
    let pointer_type = isa.pointer_type();
    let mut wrapper_sig = ir::Signature::new(isa.frontend_config().default_call_conv);
    wrapper_sig.params.push(ir::AbiParam::new(pointer_type)); // Add the `callee_address` parameter.
    wrapper_sig.params.push(ir::AbiParam::new(pointer_type)); // Add the `values_vec` parameter.

    let mut func = ir::Function::with_name_signature(name, wrapper_sig);

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
            // We always store vector types in little-endian byte order as DataValue.
            let mut flags = ir::MemFlags::trusted();
            if param.value_type.is_vector() {
                flags.set_endianness(ir::Endianness::Little);
            }

            // Load the value.
            builder.ins().load(
                param.value_type,
                flags,
                values_vec_ptr_val,
                (i * UnboxedValues::SLOT_SIZE) as i32,
            )
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
        // We always store vector types in little-endian byte order as DataValue.
        let mut flags = ir::MemFlags::trusted();
        if param.value_type.is_vector() {
            flags.set_endianness(ir::Endianness::Little);
        }
        // Store the value.
        builder.ins().store(
            flags,
            *value,
            values_vec_ptr_val,
            (i * UnboxedValues::SLOT_SIZE) as i32,
        );
    }

    builder.ins().return_(&[]);
    builder.finalize();

    func
}

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::__m128i;
#[cfg(target_arch = "x86_64")]
#[allow(improper_ctypes_definitions)]
extern "C" fn __cranelift_x86_pshufb(a: __m128i, b: __m128i) -> __m128i {
    union U {
        reg: __m128i,
        mem: [u8; 16],
    }

    unsafe {
        let a = U { reg: a }.mem;
        let b = U { reg: b }.mem;

        let select = |arr: &[u8; 16], byte: u8| {
            if byte & 0x80 != 0 {
                0x00
            } else {
                arr[(byte & 0xf) as usize]
            }
        };

        U {
            mem: [
                select(&a, b[0]),
                select(&a, b[1]),
                select(&a, b[2]),
                select(&a, b[3]),
                select(&a, b[4]),
                select(&a, b[5]),
                select(&a, b[6]),
                select(&a, b[7]),
                select(&a, b[8]),
                select(&a, b[9]),
                select(&a, b[10]),
                select(&a, b[11]),
                select(&a, b[12]),
                select(&a, b[13]),
                select(&a, b[14]),
                select(&a, b[15]),
            ],
        }
        .reg
    }
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
            function %test() -> i8 {
            block0:
                nop
                v1 = iconst.i8 -1
                return v1
            }",
        );
        let ctrl_plane = &mut ControlPlane::default();

        // extract function
        let test_file = parse_test(code.as_str(), ParseOptions::default()).unwrap();
        assert_eq!(1, test_file.functions.len());
        let function = test_file.functions[0].0.clone();

        // execute function
        let mut compiler = TestFileCompiler::with_default_host_isa().unwrap();
        compiler.declare_function(&function).unwrap();
        compiler
            .define_function(function.clone(), ctrl_plane)
            .unwrap();
        compiler
            .create_trampoline_for_function(&function, ctrl_plane)
            .unwrap();
        let compiled = compiler.compile().unwrap();
        let trampoline = compiled.get_trampoline(&function).unwrap();
        let returned = trampoline.call(&[]);
        assert_eq!(returned, vec![DataValue::I8(-1)])
    }

    #[test]
    fn trampolines() {
        let function = parse(
            "
            function %test(f32, i8, i64x2, i8) -> f32x4, i64 {
            block0(v0: f32, v1: i8, v2: i64x2, v3: i8):
                v4 = vconst.f32x4 [0x0.1 0x0.2 0x0.3 0x0.4]
                v5 = iconst.i64 -1
                return v4, v5
            }",
        );

        let compiler = TestFileCompiler::with_default_host_isa().unwrap();
        let trampoline = make_trampoline(
            UserFuncName::user(0, 0),
            &function.signature,
            compiler.module.isa(),
        );
        println!("{trampoline}");
        assert!(format!("{trampoline}").ends_with(
            "sig0 = (f32, i8, i64x2, i8) -> f32x4, i64 fast

block0(v0: i64, v1: i64):
    v2 = load.f32 notrap aligned v1
    v3 = load.i8 notrap aligned v1+16
    v4 = load.i64x2 notrap aligned little v1+32
    v5 = load.i8 notrap aligned v1+48
    v6, v7 = call_indirect sig0, v0(v2, v3, v4, v5)
    store notrap aligned little v6, v1
    store notrap aligned v7, v1+16
    return
}
"
        ));
    }
}
