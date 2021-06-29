use crate::function_generator::FunctionGenerator;
use anyhow::Result;
use arbitrary::{Arbitrary, Unstructured};
use cranelift::codegen::data_value::DataValue;
use cranelift::codegen::ir::types::*;
use cranelift::codegen::ir::Function;
use cranelift::prelude::*;

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
}

impl<'r, 'data> FuzzGen<'r, 'data>
where
    'data: 'r,
{
    pub fn new(u: &'r mut Unstructured<'data>) -> Self {
        Self { u }
    }

    fn generate_test_inputs(&mut self, signature: &Signature) -> Result<Vec<TestCaseInput>> {
        // TODO: More test cases?
        let num_tests = self.u.int_in_range(1..=10)?;
        let mut inputs = Vec::with_capacity(num_tests);

        for _ in 0..num_tests {
            let test_args = signature
                .params
                .iter()
                .map(|p| {
                    let imm64 = match p.value_type {
                        I8 => self.u.arbitrary::<i8>()? as i64,
                        I16 => self.u.arbitrary::<i16>()? as i64,
                        I32 => self.u.arbitrary::<i32>()? as i64,
                        I64 => self.u.arbitrary::<i64>()?,
                        _ => unreachable!(),
                    };
                    Ok(DataValue::from_integer(imm64, p.value_type)?)
                })
                .collect::<Result<TestCaseInput>>()?;

            inputs.push(test_args);
        }

        Ok(inputs)
    }

    pub fn generate_test(mut self) -> Result<TestCase> {
        let func = FunctionGenerator::new(&mut self.u).generate()?;
        let inputs = self.generate_test_inputs(&func.signature)?;

        Ok(TestCase { func, inputs })
    }
}
