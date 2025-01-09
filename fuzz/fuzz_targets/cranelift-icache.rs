#![no_main]

use cranelift_codegen::{
    cursor::{Cursor, FuncCursor},
    incremental_cache as icache,
    ir::{
        self, immediates::Imm64, ExternalName, Function, LibCall, Signature, UserExternalName,
        UserFuncName,
    },
    isa, Context,
};
use libfuzzer_sys::{
    arbitrary::{self, Arbitrary, Unstructured},
    fuzz_target,
};
use std::fmt;

use cranelift_fuzzgen::*;

/// TODO: This *almost* could be replaced with `LibCall::all()`, but
/// `LibCall::signature` panics for some libcalls, so we need to avoid that.
const ALLOWED_LIBCALLS: &'static [LibCall] = &[
    LibCall::CeilF32,
    LibCall::CeilF64,
    LibCall::FloorF32,
    LibCall::FloorF64,
    LibCall::TruncF32,
    LibCall::TruncF64,
    LibCall::NearestF32,
    LibCall::NearestF64,
    LibCall::FmaF32,
    LibCall::FmaF64,
];

/// A generated function with an ISA that targets one of cranelift's backends.
pub struct FunctionWithIsa {
    /// TargetIsa to use when compiling this test case
    pub isa: isa::OwnedTargetIsa,

    /// Function under test
    pub func: Function,
}

impl FunctionWithIsa {
    pub fn generate(u: &mut Unstructured) -> anyhow::Result<Self> {
        // We filter out targets that aren't supported in the current build
        // configuration after randomly choosing one, instead of randomly choosing
        // a supported one, so that the same fuzz input works across different build
        // configurations.
        let target = u.choose(isa::ALL_ARCHITECTURES)?;
        let mut builder =
            isa::lookup_by_name(target).map_err(|_| arbitrary::Error::IncorrectFormat)?;
        let architecture = builder.triple().architecture;

        let mut generator = FuzzGen::new(u);
        let flags = generator
            .generate_flags(architecture)
            .map_err(|_| arbitrary::Error::IncorrectFormat)?;
        generator.set_isa_flags(&mut builder, IsaFlagGen::All)?;
        let isa = builder
            .finish(flags)
            .map_err(|_| arbitrary::Error::IncorrectFormat)?;

        // Function name must be in a different namespace than TESTFILE_NAMESPACE (0)
        let fname = UserFuncName::user(1, 0);

        // We don't actually generate these functions, we just simulate their signatures and names
        let func_count = generator
            .u
            .int_in_range(generator.config.testcase_funcs.clone())?;
        let usercalls = (0..func_count)
            .map(|i| {
                let name = UserExternalName::new(2, i as u32);
                let sig = generator.generate_signature(&*isa)?;
                Ok((name, sig))
            })
            .collect::<anyhow::Result<Vec<(UserExternalName, Signature)>>>()
            .map_err(|_| arbitrary::Error::IncorrectFormat)?;

        let func = generator
            .generate_func(fname, isa.clone(), usercalls, ALLOWED_LIBCALLS.to_vec())
            .map_err(|_| arbitrary::Error::IncorrectFormat)?;

        Ok(FunctionWithIsa { isa, func })
    }
}

impl fmt::Debug for FunctionWithIsa {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO: We could avoid the clone here.
        let funcs = &[self.func.clone()];
        PrintableTestCase::compile(&self.isa, funcs).fmt(f)
    }
}

impl<'a> Arbitrary<'a> for FunctionWithIsa {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        Self::generate(u).map_err(|_| arbitrary::Error::IncorrectFormat)
    }
}

fuzz_target!(|func: FunctionWithIsa| {
    let FunctionWithIsa { mut func, isa } = func;

    let cache_key_hash = icache::compute_cache_key(&*isa, &func);

    let mut context = Context::for_function(func.clone());
    let prev_stencil = match context.compile_stencil(&*isa, &mut Default::default()) {
        Ok(stencil) => stencil,
        Err(_) => return,
    };

    let (prev_stencil, serialized) = icache::serialize_compiled(prev_stencil);
    let serialized = serialized.expect("serialization should work");
    let prev_result = prev_stencil.apply_params(&func.params);

    let new_result = icache::try_finish_recompile(&func, &serialized)
        .expect("recompilation should always work for identity");

    assert_eq!(new_result, prev_result, "MachCompileResult:s don't match");

    let new_info = new_result.code_info();
    assert_eq!(new_info, prev_result.code_info(), "CodeInfo:s don't match");

    // If the func has at least one user-defined func ref, change it to match a
    // different external function.
    let expect_cache_hit = if let Some(user_ext_ref) =
        func.stencil.dfg.ext_funcs.values().find_map(|data| {
            if let ExternalName::User(user_ext_ref) = &data.name {
                Some(user_ext_ref)
            } else {
                None
            }
        }) {
        let mut prev = func.params.user_named_funcs()[*user_ext_ref].clone();
        prev.index = prev.index.checked_add(1).unwrap_or_else(|| prev.index - 1);
        func.params.reset_user_func_name(*user_ext_ref, prev);
        true
    } else {
        // otherwise just randomly change one instruction in the middle and see what happens.
        let mut changed = false;
        let mut cursor = FuncCursor::new(&mut func);
        'out: while let Some(_block) = cursor.next_block() {
            while let Some(inst) = cursor.next_inst() {
                // It's impractical to do any replacement at this point. Try to find any
                // instruction that returns one int value, and replace it with an iconst.
                if cursor.func.dfg.inst_results(inst).len() != 1 {
                    continue;
                }
                let out_ty = cursor
                    .func
                    .dfg
                    .value_type(cursor.func.dfg.first_result(inst));
                match out_ty {
                    ir::types::I32 | ir::types::I64 => {}
                    _ => continue,
                }

                if let ir::InstructionData::UnaryImm {
                    opcode: ir::Opcode::Iconst,
                    imm,
                } = cursor.func.dfg.insts[inst]
                {
                    let imm = imm.bits();
                    cursor.func.dfg.insts[inst] = ir::InstructionData::UnaryImm {
                        opcode: ir::Opcode::Iconst,
                        imm: Imm64::new(imm.checked_add(1).unwrap_or_else(|| imm - 1)),
                    };
                } else {
                    cursor.func.dfg.insts[inst] = ir::InstructionData::UnaryImm {
                        opcode: ir::Opcode::Iconst,
                        imm: Imm64::new(42),
                    };
                }

                changed = true;
                break 'out;
            }
        }

        if !changed {
            return;
        }

        // We made it so that there shouldn't be a cache hit.
        false
    };

    let new_cache_key_hash = icache::compute_cache_key(&*isa, &func);

    if expect_cache_hit {
        assert!(cache_key_hash == new_cache_key_hash);
    } else {
        assert!(cache_key_hash != new_cache_key_hash);
    }

    context = Context::for_function(func.clone());

    let after_mutation_result = match context.compile(&*isa, &mut Default::default()) {
        Ok(info) => info,
        Err(_) => return,
    };

    if expect_cache_hit {
        let after_mutation_result_from_cache = icache::try_finish_recompile(&func, &serialized)
            .expect("recompilation should always work for identity");
        assert_eq!(*after_mutation_result, after_mutation_result_from_cache);

        let new_info = after_mutation_result_from_cache.code_info();
        assert_eq!(
            new_info,
            after_mutation_result.code_info(),
            "CodeInfo:s don't match"
        );
    }
});
