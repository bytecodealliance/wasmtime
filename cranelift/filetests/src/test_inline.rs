//! Test command for testing inlining.
//!
//! The `inline` test command inlines all calls, and optionally optimizes each
//! function before and after the optimization passes. It does not perform
//! lowering or regalloc. The output for filecheck purposes is the resulting
//! CLIF.
//!
//! Some legalization may be ISA-specific, so this requires an ISA
//! (for now).

use crate::subtest::{Context, SubTest, check_precise_output, run_filecheck};
use anyhow::{Context as _, Result};
use cranelift_codegen::{
    inline::{Inline, InlineCommand},
    ir,
    print_errors::pretty_verifier_error,
};
use cranelift_control::ControlPlane;
use cranelift_reader::{TestCommand, TestOption};
use std::{
    borrow::Cow,
    cell::{Ref, RefCell},
    collections::HashMap,
};

#[derive(Default)]
struct TestInline {
    /// Flag indicating that the text expectation, comments after the function,
    /// must be a precise 100% match on the compiled output of the function.
    /// This test assertion is also automatically-update-able to allow tweaking
    /// the code generator and easily updating all affected tests.
    precise_output: bool,

    /// Flag indicating whether to run optimizations on the function after
    /// inlining.
    optimize: bool,

    /// The already-defined functions we have seen, available for inlining into
    /// future functions.
    funcs: RefCell<HashMap<ir::UserFuncName, ir::Function>>,
}

pub fn subtest(parsed: &TestCommand) -> Result<Box<dyn SubTest>> {
    assert_eq!(parsed.command, "inline");
    let mut test = TestInline::default();
    for option in parsed.options.iter() {
        match option {
            TestOption::Flag("precise-output") => test.precise_output = true,
            TestOption::Flag("optimize") => test.optimize = true,
            _ => anyhow::bail!("unknown option on {}", parsed),
        }
    }
    Ok(Box::new(test))
}

impl SubTest for TestInline {
    fn name(&self) -> &'static str {
        "inline"
    }

    fn is_mutating(&self) -> bool {
        true
    }

    fn needs_isa(&self) -> bool {
        true
    }

    fn run(&self, func: Cow<ir::Function>, context: &Context) -> Result<()> {
        // Legalize this function.
        let isa = context.isa.unwrap();
        let mut comp_ctx = cranelift_codegen::Context::for_function(func.into_owned());
        comp_ctx
            .legalize(isa)
            .map_err(|e| crate::pretty_anyhow_error(&comp_ctx.func, e))
            .context("error while legalizing")?;

        // Insert this function in our map for inlining into subsequent
        // functions.
        let func_name = comp_ctx.func.name.clone();
        self.funcs
            .borrow_mut()
            .insert(func_name, comp_ctx.func.clone());

        // Run the inliner.
        let inlined_any = comp_ctx.inline(Inliner(self.funcs.borrow()))?;

        // Verify that the CLIF is still valid.
        comp_ctx
            .verify(context.flags_or_isa())
            .map_err(|errors| {
                anyhow::Error::msg(pretty_verifier_error(&comp_ctx.func, None, errors))
            })
            .context("CLIF verification error after inlining")?;

        // If requested, run optimizations.
        if self.optimize {
            comp_ctx
                .optimize(isa, &mut ControlPlane::default())
                .map_err(|e| crate::pretty_anyhow_error(&comp_ctx.func, e))
                .context("error while optimizing")?;
        }

        // Check the filecheck expectations.
        let actual = if inlined_any {
            format!("{:?}", comp_ctx.func)
        } else {
            format!("(no functions inlined into {})", comp_ctx.func.name)
        };
        log::debug!("filecheck input: {actual}");
        if self.precise_output {
            let actual: Vec<_> = actual.lines().collect();
            check_precise_output(&actual, context)
        } else {
            run_filecheck(&actual, context)
        }
    }
}

struct Inliner<'a>(Ref<'a, HashMap<ir::UserFuncName, ir::Function>>);

impl<'a> Inline for Inliner<'a> {
    fn inline(
        &mut self,
        caller: &ir::Function,
        _inst: ir::Inst,
        _opcode: ir::Opcode,
        callee: ir::FuncRef,
        _args: &[ir::Value],
    ) -> InlineCommand<'_> {
        match &caller.dfg.ext_funcs[callee].name {
            ir::ExternalName::User(name) => match caller
                .params
                .user_named_funcs()
                .get(*name)
                .and_then(|name| self.0.get(&ir::UserFuncName::User(name.clone())))
            {
                None => InlineCommand::KeepCall,
                Some(f) => InlineCommand::Inline(Cow::Borrowed(f)),
            },
            ir::ExternalName::TestCase(name) => {
                match self.0.get(&ir::UserFuncName::Testcase(name.clone())) {
                    None => InlineCommand::KeepCall,
                    Some(f) => InlineCommand::Inline(Cow::Borrowed(f)),
                }
            }
            ir::ExternalName::LibCall(_) | ir::ExternalName::KnownSymbol(_) => {
                InlineCommand::KeepCall
            }
        }
    }
}
