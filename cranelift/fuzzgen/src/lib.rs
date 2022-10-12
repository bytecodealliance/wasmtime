use crate::config::Config;
use crate::function_generator::FunctionGenerator;
use anyhow::Result;
use arbitrary::{Arbitrary, Unstructured};
use cranelift::codegen::data_value::DataValue;
use cranelift::codegen::ir::types::*;
use cranelift::codegen::ir::Function;
use cranelift::codegen::Context;
use cranelift::prelude::*;
use cranelift_native::builder_with_options;
use std::fmt;

mod config;
mod function_generator;
mod passes;

pub type TestCaseInput = Vec<DataValue>;

/// Simple wrapper to generate a single Cranelift `Function`.
#[derive(Debug)]
pub struct SingleFunction(pub Function);

impl<'a> Arbitrary<'a> for SingleFunction {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        FuzzGen::new(u)
            .generate_func()
            .map_err(|_| arbitrary::Error::IncorrectFormat)
            .map(Self)
    }
}

pub struct TestCase {
    pub func: Function,
    /// Generate multiple test inputs for each test case.
    /// This allows us to get more coverage per compilation, which may be somewhat expensive.
    pub inputs: Vec<TestCaseInput>,
}

impl fmt::Debug for TestCase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r#";; Fuzzgen test case

test interpret
test run
set enable_llvm_abi_extensions
target aarch64
target s390x
target x86_64

"#
        )?;

        writeln!(f, "{}", self.func)?;

        writeln!(f, "; Note: the results in the below test cases are simply a placeholder and probably will be wrong\n")?;

        for input in self.inputs.iter() {
            // TODO: We don't know the expected outputs, maybe we can run the interpreter
            // here to figure them out? Should work, however we need to be careful to catch
            // panics in case its the interpreter that is failing.
            // For now create a placeholder output consisting of the zero value for the type
            let returns = &self.func.signature.returns;
            let placeholder_output = returns
                .iter()
                .map(|param| DataValue::read_from_slice(&[0; 16][..], param.value_type))
                .map(|val| format!("{}", val))
                .collect::<Vec<_>>()
                .join(", ");

            // If we have no output, we don't need the == condition
            let test_condition = match returns.len() {
                0 => String::new(),
                1 => format!(" == {}", placeholder_output),
                _ => format!(" == [{}]", placeholder_output),
            };

            let args = input
                .iter()
                .map(|val| format!("{}", val))
                .collect::<Vec<_>>()
                .join(", ");

            writeln!(f, "; run: {}({}){}", self.func.name, args, test_condition)?;
        }

        Ok(())
    }
}

impl<'a> Arbitrary<'a> for TestCase {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        FuzzGen::new(u)
            .generate_test()
            .map_err(|_| arbitrary::Error::IncorrectFormat)
    }
}

pub struct FuzzGen<'r, 'data>
where
    'data: 'r,
{
    u: &'r mut Unstructured<'data>,
    config: Config,
}

impl<'r, 'data> FuzzGen<'r, 'data>
where
    'data: 'r,
{
    pub fn new(u: &'r mut Unstructured<'data>) -> Self {
        Self {
            u,
            config: Config::default(),
        }
    }

    fn generate_datavalue(&mut self, ty: Type) -> Result<DataValue> {
        Ok(match ty {
            ty if ty.is_int() => {
                let imm = match ty {
                    I8 => self.u.arbitrary::<i8>()? as i128,
                    I16 => self.u.arbitrary::<i16>()? as i128,
                    I32 => self.u.arbitrary::<i32>()? as i128,
                    I64 => self.u.arbitrary::<i64>()? as i128,
                    I128 => self.u.arbitrary::<i128>()?,
                    _ => unreachable!(),
                };
                DataValue::from_integer(imm, ty)?
            }
            // f{32,64}::arbitrary does not generate a bunch of important values
            // such as Signaling NaN's / NaN's with payload, so generate floats from integers.
            F32 => DataValue::F32(Ieee32::with_bits(u32::arbitrary(self.u)?)),
            F64 => DataValue::F64(Ieee64::with_bits(u64::arbitrary(self.u)?)),
            _ => unimplemented!(),
        })
    }

    fn generate_test_inputs(mut self, signature: &Signature) -> Result<Vec<TestCaseInput>> {
        let mut inputs = Vec::new();

        loop {
            let last_len = self.u.len();

            let test_args = signature
                .params
                .iter()
                .map(|p| self.generate_datavalue(p.value_type))
                .collect::<Result<TestCaseInput>>()?;

            inputs.push(test_args);

            // Continue generating input as long as we just consumed some of self.u. Otherwise
            // we'll generate the same test input again and again, forever. Note that once self.u
            // becomes empty we obviously can't consume any more of it, so this check is more
            // general. Also note that we need to generate at least one input or the fuzz target
            // won't actually test anything, so checking at the end of the loop is good, even if
            // self.u is empty from the start and we end up with all zeros in test_args.
            assert!(self.u.len() <= last_len);
            if self.u.len() == last_len {
                break;
            }
        }

        Ok(inputs)
    }

    fn run_func_passes(&mut self, func: Function) -> Result<Function> {
        // Do a NaN Canonicalization pass on the generated function.
        //
        // Both IEEE754 and the Wasm spec are somewhat loose about what is allowed
        // to be returned from NaN producing operations. And in practice this changes
        // from X86 to Aarch64 and others. Even in the same host machine, the
        // interpreter may produce a code sequence different from cranelift that
        // generates different NaN's but produces legal results according to the spec.
        //
        // These differences cause spurious failures in the fuzzer. To fix this
        // we enable the NaN Canonicalization pass that replaces any NaN's produced
        // with a single fixed canonical NaN value.
        //
        // This is something that we can enable via flags for the compiled version, however
        // the interpreter won't get that version, so call that pass manually here.

        let mut ctx = Context::for_function(func);
        // Assume that we are generating this function for the current ISA.
        // We disable the verifier here, since if it fails it prevents a test case from
        // being generated and formatted by `cargo fuzz fmt`.
        // We run the verifier before compiling the code, so it always gets verified.
        let flags = settings::Flags::new({
            let mut builder = settings::builder();
            builder.set("enable_verifier", "false").unwrap();
            builder
        });

        let isa = builder_with_options(false)
            .expect("Unable to build a TargetIsa for the current host")
            .finish(flags)
            .expect("Failed to build TargetISA");

        ctx.canonicalize_nans(isa.as_ref())
            .expect("Failed NaN canonicalization pass");

        // Run the int_divz pass
        //
        // This pass replaces divs and rems with sequences that do not trap
        passes::do_int_divz_pass(self, &mut ctx.func)?;

        // This pass replaces fcvt* instructions with sequences that do not trap
        passes::do_fcvt_trap_pass(self, &mut ctx.func)?;

        Ok(ctx.func)
    }

    fn generate_func(&mut self) -> Result<Function> {
        let func = FunctionGenerator::new(&mut self.u, &self.config).generate()?;
        self.run_func_passes(func)
    }

    pub fn generate_test(mut self) -> Result<TestCase> {
        // If we're generating test inputs as well as a function, then we're planning to execute
        // this function. That means that any function references in it need to exist. We don't yet
        // have infrastructure for generating multiple functions, so just don't generate funcrefs.
        self.config.funcrefs_per_function = 0..=0;

        let func = self.generate_func()?;
        let inputs = self.generate_test_inputs(&func.signature)?;
        Ok(TestCase { func, inputs })
    }
}
