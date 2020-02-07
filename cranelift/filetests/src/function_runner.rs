use core::mem;
use cranelift_codegen::binemit::{NullRelocSink, NullStackmapSink, NullTrapSink};
use cranelift_codegen::ir::Function;
use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::{settings, Context};
use cranelift_native::builder as host_isa_builder;
use memmap::MmapMut;

/// Run a function on a host
pub struct FunctionRunner {
    function: Function,
    isa: Box<dyn TargetIsa>,
}

impl FunctionRunner {
    /// Build a function runner from a function and the ISA to run on (must be the host machine's ISA)
    pub fn new(function: Function, isa: Box<dyn TargetIsa>) -> Self {
        Self { function, isa }
    }

    /// Build a function runner using the host machine's ISA and the passed flags
    pub fn with_host_isa(function: Function, flags: settings::Flags) -> Self {
        let builder = host_isa_builder().expect("Unable to build a TargetIsa for the current host");
        let isa = builder.finish(flags);
        Self::new(function, isa)
    }

    /// Build a function runner using the host machine's ISA and the default flags for this ISA
    pub fn with_default_host_isa(function: Function) -> Self {
        let flags = settings::Flags::new(settings::builder());
        Self::with_host_isa(function, flags)
    }

    /// Compile and execute a single function, expecting a boolean to be returned; a 'true' value is
    /// interpreted as a successful test execution and mapped to Ok whereas a 'false' value is
    /// interpreted as a failed test and mapped to Err.
    pub fn run(&self) -> Result<(), String> {
        let func = self.function.clone();
        if !(func.signature.params.is_empty()
            && func.signature.returns.len() == 1
            && func.signature.returns.first().unwrap().value_type.is_bool())
        {
            return Err(String::from(
                "Functions must have a signature like: () -> boolean",
            ));
        }

        if func.signature.call_conv != self.isa.default_call_conv() {
            return Err(String::from(
                "Functions only run on the host's default calling convention; remove the specified calling convention in the function signature to use the host's default.",
            ));
        }

        // set up the context
        let mut context = Context::new();
        context.func = func;

        // compile and encode the result to machine code
        let relocs = &mut NullRelocSink {};
        let traps = &mut NullTrapSink {};
        let stackmaps = &mut NullStackmapSink {};
        let code_info = context
            .compile(self.isa.as_ref())
            .map_err(|e| e.to_string())?;
        let mut code_page =
            MmapMut::map_anon(code_info.total_size as usize).map_err(|e| e.to_string())?;

        unsafe {
            context.emit_to_memory(
                self.isa.as_ref(),
                code_page.as_mut_ptr(),
                relocs,
                traps,
                stackmaps,
            );
        };

        let code_page = code_page.make_exec().map_err(|e| e.to_string())?;
        let callable_fn: fn() -> bool = unsafe { mem::transmute(code_page.as_ptr()) };

        // execute
        if callable_fn() {
            Ok(())
        } else {
            Err(format!("Failed: {}", context.func.name.to_string()))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use cranelift_reader::{parse_test, ParseOptions};

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
        let runner = FunctionRunner::with_default_host_isa(function);
        runner.run().unwrap() // will panic if execution fails
    }
}
