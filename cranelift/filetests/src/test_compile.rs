//! Test command for testing the code generator pipeline
//!
//! The `compile` test command runs each function through the full code generator pipeline

use crate::subtest::{run_filecheck, Context, SubTest};
use anyhow::{bail, Result};
use cranelift_codegen::ir;
use cranelift_codegen::ir::function::FunctionParameters;
use cranelift_codegen::isa;
use cranelift_codegen::CompiledCode;
use cranelift_reader::{TestCommand, TestOption};
use log::info;
use similar::TextDiff;
use std::borrow::Cow;
use std::env;

struct TestCompile {
    /// Flag indicating that the text expectation, comments after the function,
    /// must be a precise 100% match on the compiled output of the function.
    /// This test assertion is also automatically-update-able to allow tweaking
    /// the code generator and easily updating all affected tests.
    precise_output: bool,
}

pub fn subtest(parsed: &TestCommand) -> Result<Box<dyn SubTest>> {
    assert_eq!(parsed.command, "compile");
    let mut test = TestCompile {
        precise_output: false,
    };
    for option in parsed.options.iter() {
        match option {
            TestOption::Flag("precise-output") => test.precise_output = true,
            _ => anyhow::bail!("unknown option on {}", parsed),
        }
    }
    Ok(Box::new(test))
}

impl SubTest for TestCompile {
    fn name(&self) -> &'static str {
        "compile"
    }

    fn is_mutating(&self) -> bool {
        true
    }

    fn needs_isa(&self) -> bool {
        true
    }

    fn run(&self, func: Cow<ir::Function>, context: &Context) -> Result<()> {
        let isa = context.isa.expect("compile needs an ISA");
        let params = func.params.clone();
        let mut comp_ctx = cranelift_codegen::Context::for_function(func.into_owned());

        // With `MachBackend`s, we need to explicitly request dissassembly results.
        comp_ctx.set_disasm(true);

        let compiled_code = comp_ctx
            .compile(isa)
            .map_err(|e| crate::pretty_anyhow_error(&e.func, e.inner))?;
        let total_size = compiled_code.code_info().total_size;

        let vcode = compiled_code.vcode.as_ref().unwrap();

        info!("Generated {} bytes of code:\n{}", total_size, vcode);

        if self.precise_output {
            check_precise_output(isa, &params, &compiled_code, context)
        } else {
            run_filecheck(&vcode, context)
        }
    }
}

fn check_precise_output(
    isa: &dyn isa::TargetIsa,
    params: &FunctionParameters,
    compiled_code: &CompiledCode,
    context: &Context,
) -> Result<()> {
    let cs = isa
        .to_capstone()
        .map_err(|e| anyhow::format_err!("{}", e))?;
    let dis = compiled_code.disassemble(Some(params), &cs)?;

    let actual = Vec::from_iter(
        std::iter::once("VCode:")
            .chain(compiled_code.vcode.as_ref().unwrap().lines())
            .chain(["", "Disassembled:"])
            .chain(dis.lines()),
    );

    // Use the comments after the function to build the test expectation.
    let expected = context
        .details
        .comments
        .iter()
        .filter(|c| !c.text.starts_with(";;"))
        .map(|c| c.text.strip_prefix("; ").unwrap_or(c.text))
        .collect::<Vec<_>>();

    // If the expectation matches what we got, then there's nothing to do.
    if actual == expected {
        return Ok(());
    }

    // If we're supposed to automatically update the test, then do so here.
    if env::var("CRANELIFT_TEST_BLESS").unwrap_or(String::new()) == "1" {
        return update_test(&actual, context);
    }

    // Otherwise this test has failed, and we can print out as such.
    bail!(
        "compilation of function on line {} does not match\n\
         the text expectation\n\
         \n\
         {}\n\
         \n\
         This test assertion can be automatically updated by setting the\n\
         CRANELIFT_TEST_BLESS=1 environment variable when running this test.
         ",
        context.details.location.line_number,
        TextDiff::from_slices(&expected, &actual)
            .unified_diff()
            .header("expected", "actual")
    )
}

fn update_test(output: &[&str], context: &Context) -> Result<()> {
    context
        .file_update
        .update_at(&context.details.location, |new_test, old_test| {
            // blank newline after the function
            new_test.push_str("\n");

            // Splice in the test output
            for output in output {
                new_test.push_str("; ");
                new_test.push_str(output);
                new_test.push_str("\n");
            }

            // blank newline after test assertion
            new_test.push_str("\n");

            // Drop all remaining commented lines (presumably the old test expectation),
            // but after we hit a real line then we push all remaining lines.
            let mut in_next_function = false;
            for line in old_test {
                if !in_next_function
                    && (line.trim().is_empty()
                        || (line.starts_with(";") && !line.starts_with(";;")))
                {
                    continue;
                }
                in_next_function = true;
                new_test.push_str(line);
                new_test.push_str("\n");
            }
        })
}
