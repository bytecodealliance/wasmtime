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

mod config;
mod function_generator;

pub type TestCaseInput = Vec<DataValue>;

#[derive(Debug)]
pub struct TestCase {
    pub func: Function,
    /// Generate multiple test inputs for each test case.
    /// This allows us to get more coverage per compilation, which may be somewhat expensive.
    pub inputs: Vec<TestCaseInput>,
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
                    _ => unreachable!(),
                };
                DataValue::from_integer(imm, ty)?
            }
            ty if ty.is_bool() => DataValue::B(bool::arbitrary(self.u)?),
            // f{32,64}::arbitrary does not generate a bunch of important values
            // such as Signaling NaN's / NaN's with payload, so generate floats from integers.
            F32 => DataValue::F32(Ieee32::with_bits(u32::arbitrary(self.u)?)),
            F64 => DataValue::F64(Ieee64::with_bits(u64::arbitrary(self.u)?)),
            _ => unimplemented!(),
        })
    }

    fn generate_test_inputs(&mut self, signature: &Signature) -> Result<Vec<TestCaseInput>> {
        let num_tests = self.u.int_in_range(self.config.test_case_inputs.clone())?;
        let mut inputs = Vec::with_capacity(num_tests);

        for _ in 0..num_tests {
            let test_args = signature
                .params
                .iter()
                .map(|p| self.generate_datavalue(p.value_type))
                .collect::<Result<TestCaseInput>>()?;

            inputs.push(test_args);
        }

        Ok(inputs)
    }

    fn run_func_passes(&self, func: Function) -> Result<Function> {
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
        // Assume that we are generating this function for the current ISA
        // this is only used for the verifier after `canonicalize_nans` so
        // it's not too important.
        let flags = settings::Flags::new(settings::builder());
        let isa = builder_with_options(false)
            .expect("Unable to build a TargetIsa for the current host")
            .finish(flags)?;

        ctx.canonicalize_nans(isa.as_ref())?;

        Ok(ctx.func)
    }

    pub fn generate_test(mut self) -> Result<TestCase> {
        let func = FunctionGenerator::new(&mut self.u, &self.config).generate()?;
        let inputs = self.generate_test_inputs(&func.signature)?;

        let func = self.run_func_passes(func)?;

        Ok(TestCase { func, inputs })
    }
}
