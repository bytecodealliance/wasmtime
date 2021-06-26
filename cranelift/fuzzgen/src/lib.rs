use crate::function_generator::FunctionGenerator;
use anyhow::Result;
use arbitrary::Unstructured;
use cranelift::codegen::data_value::DataValue;
use cranelift::codegen::ir::types::*;
use cranelift::codegen::{ir::Function, verify_function};
use cranelift::prelude::*;

mod function_generator;

pub type TestCaseInput = Vec<DataValue>;

pub struct TestCase {
    pub func: Function,
    pub inputs: Vec<TestCaseInput>,
}

pub struct FuzzGen<'a> {
    u: &'a mut Unstructured<'a>,
    vars: Vec<(Type, Variable)>,
}

impl<'a> FuzzGen<'a> {
    pub fn new(u: &'a mut Unstructured<'a>) -> Self {
        Self { u, vars: vec![] }
    }

    fn verify_function(&self, func: &Function) -> Result<()> {
        let flags = settings::Flags::new(settings::builder());
        verify_function(&func, &flags)?;
        Ok(())
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

    pub fn generate_test(&'a mut self) -> Result<TestCase> {
        let func = FunctionGenerator::new(self.u).generate()?;
        self.verify_function(&func)?;

        let inputs = self.generate_test_inputs(&func.signature)?;

        Ok(TestCase { func, inputs })
    }
}
