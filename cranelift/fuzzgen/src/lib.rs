use crate::config::Config;
use crate::function_generator::FunctionGenerator;
use crate::settings::{Flags, OptLevel};
use anyhow::Result;
use arbitrary::{Arbitrary, Unstructured};
use cranelift::codegen::data_value::DataValue;
use cranelift::codegen::ir::{types::*, UserExternalName, UserFuncName};
use cranelift::codegen::ir::{Function, LibCall};
use cranelift::codegen::Context;
use cranelift::prelude::*;
use cranelift_arbitrary::CraneliftArbitrary;
use cranelift_native::builder_with_options;
use std::fmt;
use target_lexicon::{Architecture, Triple};

mod config;
mod cranelift_arbitrary;
mod function_generator;
mod passes;

/// These libcalls need a interpreter implementation in `cranelift-fuzzgen.rs`
const ALLOWED_LIBCALLS: &'static [LibCall] = &[
    LibCall::CeilF32,
    LibCall::CeilF64,
    LibCall::FloorF32,
    LibCall::FloorF64,
    LibCall::TruncF32,
    LibCall::TruncF64,
];

pub type TestCaseInput = Vec<DataValue>;

/// Print only non default flags.
pub fn write_non_default_flags(f: &mut fmt::Formatter<'_>, flags: &settings::Flags) -> fmt::Result {
    let default_flags = settings::Flags::new(settings::builder());
    for (default, flag) in default_flags.iter().zip(flags.iter()) {
        assert_eq!(default.name, flag.name);

        if default.value_string() != flag.value_string() {
            writeln!(f, "set {}={}", flag.name, flag.value_string())?;
        }
    }

    Ok(())
}

pub struct FuzzGen<'r, 'data>
where
    'data: 'r,
{
    pub u: &'r mut Unstructured<'data>,
    pub config: Config,
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

    pub fn generate_signature(&mut self, architecture: Architecture) -> Result<Signature> {
        let max_params = self.u.int_in_range(self.config.signature_params.clone())?;
        let max_rets = self.u.int_in_range(self.config.signature_rets.clone())?;
        Ok(self.u.signature(architecture, max_params, max_rets)?)
    }

    pub fn generate_test_inputs(mut self, signature: &Signature) -> Result<Vec<TestCaseInput>> {
        let mut inputs = Vec::new();

        // Generate up to "max_test_case_inputs" inputs, we need an upper bound here since
        // the fuzzer at some point starts trying to feed us way too many inputs. (I found one
        // test case with 130k inputs!)
        for _ in 0..self.config.max_test_case_inputs {
            let last_len = self.u.len();

            let test_args = signature
                .params
                .iter()
                .map(|p| self.u.datavalue(p.value_type))
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

    pub fn generate_func(
        &mut self,
        name: UserFuncName,
        target_triple: Triple,
        usercalls: Vec<(UserExternalName, Signature)>,
    ) -> Result<Function> {
        let sig = self.generate_signature(target_triple.architecture)?;

        let func = FunctionGenerator::new(
            &mut self.u,
            &self.config,
            target_triple,
            name,
            sig,
            usercalls,
            ALLOWED_LIBCALLS.to_vec(),
        )
        .generate()?;

        self.run_func_passes(func)
    }

    /// Generate a random set of cranelift flags.
    /// Only semantics preserving flags are considered
    pub fn generate_flags(&mut self, target_arch: Architecture) -> Result<Flags> {
        let mut builder = settings::builder();

        let opt = self.u.choose(OptLevel::all())?;
        builder.set("opt_level", &format!("{}", opt)[..])?;

        // Boolean flags
        // TODO: enable_pinned_reg does not work with our current trampolines. See: #4376
        // TODO: is_pic has issues:
        //   x86: https://github.com/bytecodealliance/wasmtime/issues/5005
        //   aarch64: https://github.com/bytecodealliance/wasmtime/issues/2735
        let bool_settings = [
            "enable_alias_analysis",
            "enable_safepoints",
            "unwind_info",
            "preserve_frame_pointers",
            "enable_jump_tables",
            "enable_heap_access_spectre_mitigation",
            "enable_table_access_spectre_mitigation",
            "enable_incremental_compilation_cache_checks",
            "regalloc_checker",
            "enable_llvm_abi_extensions",
            "use_egraphs",
        ];
        for flag_name in bool_settings {
            let enabled = self
                .config
                .compile_flag_ratio
                .get(&flag_name)
                .map(|&(num, denum)| self.u.ratio(num, denum))
                .unwrap_or_else(|| bool::arbitrary(self.u))?;

            let value = format!("{}", enabled);
            builder.set(flag_name, value.as_str())?;
        }

        let supports_inline_probestack = match target_arch {
            Architecture::X86_64 => true,
            Architecture::Aarch64(_) => true,
            Architecture::Riscv64(_) => true,
            _ => false,
        };

        // Optionally test inline stackprobes on supported platforms
        // TODO: Test outlined stack probes.
        if supports_inline_probestack && bool::arbitrary(self.u)? {
            builder.enable("enable_probestack")?;
            builder.set("probestack_strategy", "inline")?;

            let size = self
                .u
                .int_in_range(self.config.stack_probe_size_log2.clone())?;
            builder.set("probestack_size_log2", &format!("{}", size))?;
        }

        // Fixed settings

        // We need llvm ABI extensions for i128 values on x86, so enable it regardless of
        // what we picked above.
        if target_arch == Architecture::X86_64 {
            builder.enable("enable_llvm_abi_extensions")?;
        }

        // This is the default, but we should ensure that it wasn't accidentally turned off anywhere.
        builder.enable("enable_verifier")?;

        // These settings just panic when they're not enabled and we try to use their respective functionality
        // so they aren't very interesting to be automatically generated.
        builder.enable("enable_atomics")?;
        builder.enable("enable_float")?;
        builder.enable("enable_simd")?;

        // `machine_code_cfg_info` generates additional metadata for the embedder but this doesn't feed back
        // into compilation anywhere, we leave it on unconditionally to make sure the generation doesn't panic.
        builder.enable("machine_code_cfg_info")?;

        Ok(Flags::new(builder))
    }
}
