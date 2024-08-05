use cranelift::codegen::data_value::DataValue;
use cranelift::codegen::ir::Function;
use cranelift::prelude::settings::SettingKind;
use cranelift::prelude::*;
use std::fmt;

use crate::TestCaseInput;

#[derive(Debug)]
enum TestCaseKind {
    Compile,
    Run,
}

/// Provides a way to format a `TestCase` in the .clif format.
pub struct PrintableTestCase<'a> {
    kind: TestCaseKind,
    isa: &'a isa::OwnedTargetIsa,
    functions: &'a [Function],
    // Only applicable for run test cases
    inputs: &'a [TestCaseInput],
}

impl<'a> PrintableTestCase<'a> {
    /// Emits a `test compile` test case.
    pub fn compile(isa: &'a isa::OwnedTargetIsa, functions: &'a [Function]) -> Self {
        Self {
            kind: TestCaseKind::Compile,
            isa,
            functions,
            inputs: &[],
        }
    }

    /// Emits a `test run` test case. These also include a `test interpret`.
    ///
    /// By convention the first function in `functions` will be considered the main function.
    pub fn run(
        isa: &'a isa::OwnedTargetIsa,
        functions: &'a [Function],
        inputs: &'a [TestCaseInput],
    ) -> Self {
        Self {
            kind: TestCaseKind::Run,
            isa,
            functions,
            inputs,
        }
    }

    /// Returns the main function of this test case.
    pub fn main(&self) -> &Function {
        &self.functions[0]
    }
}

impl<'a> fmt::Debug for PrintableTestCase<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            TestCaseKind::Compile => {
                writeln!(f, ";; Compile test case\n")?;
                writeln!(f, "test compile")?;
            }
            TestCaseKind::Run => {
                writeln!(f, ";; Run test case\n")?;
                writeln!(f, "test interpret")?;
                writeln!(f, "test run")?;
            }
        };

        write_non_default_flags(f, self.isa.flags())?;

        write!(f, "target {} ", self.isa.triple().architecture)?;
        write_non_default_isa_flags(f, &self.isa)?;
        write!(f, "\n\n")?;

        // Print the functions backwards, so that the main function is printed last
        // and near the test inputs for run test cases.
        for func in self.functions.iter().rev() {
            writeln!(f, "{func}\n")?;
        }

        if !self.inputs.is_empty() {
            writeln!(f, "; Note: the results in the below test cases are simply a placeholder and probably will be wrong\n")?;
        }

        for input in self.inputs.iter() {
            // TODO: We don't know the expected outputs, maybe we can run the interpreter
            // here to figure them out? Should work, however we need to be careful to catch
            // panics in case its the interpreter that is failing.
            // For now create a placeholder output consisting of the zero value for the type
            let returns = &self.main().signature.returns;
            let placeholder_output = returns
                .iter()
                .map(|param| DataValue::read_from_slice_ne(&[0; 16][..], param.value_type))
                .map(|val| format!("{val}"))
                .collect::<Vec<_>>()
                .join(", ");

            // If we have no output, we don't need the == condition
            let test_condition = match returns.len() {
                0 => String::new(),
                1 => format!(" == {placeholder_output}"),
                _ => format!(" == [{placeholder_output}]"),
            };

            let args = input
                .iter()
                .map(|val| format!("{val}"))
                .collect::<Vec<_>>()
                .join(", ");

            writeln!(f, "; run: {}({}){}", self.main().name, args, test_condition)?;
        }

        Ok(())
    }
}

/// Print only non default flags.
fn write_non_default_flags(f: &mut fmt::Formatter<'_>, flags: &settings::Flags) -> fmt::Result {
    let default_flags = settings::Flags::new(settings::builder());
    for (default, flag) in default_flags.iter().zip(flags.iter()) {
        assert_eq!(default.name, flag.name);

        if default.value_string() != flag.value_string() {
            writeln!(f, "set {}={}", flag.name, flag.value_string())?;
        }
    }

    Ok(())
}

/// Print non default ISA flags in a single line, as used in `target` declarations.
fn write_non_default_isa_flags(
    f: &mut fmt::Formatter<'_>,
    isa: &isa::OwnedTargetIsa,
) -> fmt::Result {
    let default_isa = isa::lookup(isa.triple().clone())
        .unwrap()
        .finish(isa.flags().clone())
        .unwrap();

    for (default, flag) in default_isa.isa_flags().iter().zip(isa.isa_flags()) {
        assert_eq!(default.name, flag.name);

        // Skip default flags, putting them all out there is too verbose.
        if default.value_string() == flag.value_string() {
            continue;
        }

        // On boolean flags we can use the shorthand syntax instead of just specifying the flag name.
        // This is slightly neater than the full syntax.
        if flag.kind() == SettingKind::Bool && flag.value_string() == "true" {
            write!(f, "{} ", flag.name)?;
        } else {
            write!(f, "{}={} ", flag.name, flag.value_string())?;
        }
    }

    Ok(())
}
