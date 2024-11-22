//! Test command for testing the code generator pipeline
//!
//! The `compile` test command runs each function through the full code generator pipeline

use crate::subtest::{check_precise_output, run_filecheck, Context, SubTest};
use anyhow::Result;
use cranelift_codegen::ir;
use cranelift_reader::{TestCommand, TestOption};
use log::info;
use std::borrow::Cow;

struct TestCompile {
    /// Flag indicating that the text expectation, comments after the function,
    /// must be a precise 100% match on the compiled output of the function.
    /// This test assertion is also automatically-update-able to allow tweaking
    /// the code generator and easily updating all affected tests.
    precise_output: bool,
    /// Flag indicating that we expect compilation to fail, not succeed.
    expect_fail: bool,
}

pub fn subtest(parsed: &TestCommand) -> Result<Box<dyn SubTest>> {
    assert_eq!(parsed.command, "compile");
    let mut test = TestCompile {
        precise_output: false,
        expect_fail: false,
    };
    for option in parsed.options.iter() {
        match option {
            TestOption::Flag("precise-output") => test.precise_output = true,
            TestOption::Flag("expect-fail") => test.expect_fail = true,
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

        // With `MachBackend`s, we need to explicitly request disassembly results.
        comp_ctx.set_disasm(true);

        let compiled_code = comp_ctx.compile(isa, &mut Default::default());

        let compiled_code = if self.expect_fail {
            if compiled_code.is_ok() {
                anyhow::bail!("Expected compilation failure but compilation succeeded");
            }
            return Ok(());
        } else {
            compiled_code.map_err(|e| crate::pretty_anyhow_error(&e.func, e.inner))?
        };
        let total_size = compiled_code.code_info().total_size;

        let vcode = compiled_code.vcode.as_ref().unwrap();

        info!("Generated {} bytes of code:\n{}", total_size, vcode);

        if self.precise_output {
            let dis = match isa.triple().architecture {
                target_lexicon::Architecture::Pulley32 | target_lexicon::Architecture::Pulley64 => {
                    // Disable hexdumps/offsets to reduce the churn in these
                    // tests as instructions are encoded differently and/or
                    // their immediates change.
                    let mut disas =
                        pulley_interpreter::disas::Disassembler::new(compiled_code.buffer.data());
                    disas.hexdump(false).offsets(false);
                    pulley_interpreter::decode::Decoder::decode_all(&mut disas)?;
                    disas.disas().to_string()
                }
                _ => {
                    let cs = isa
                        .to_capstone()
                        .map_err(|e| anyhow::format_err!("{}", e))?;
                    compiled_code.disassemble(Some(&params), &cs)?
                }
            };

            let actual = Vec::from_iter(
                std::iter::once("VCode:")
                    .chain(compiled_code.vcode.as_ref().unwrap().lines())
                    .chain(["", "Disassembled:"])
                    .chain(dis.lines()),
            );

            check_precise_output(&actual, context)
        } else {
            run_filecheck(&vcode, context)
        }
    }
}
