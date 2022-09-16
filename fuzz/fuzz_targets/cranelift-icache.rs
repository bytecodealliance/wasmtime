#![no_main]

use cranelift_codegen::{
    cursor::{Cursor, FuncCursor},
    incremental_cache as icache,
    ir::{self, immediates::Imm64, ExternalName},
    isa,
    settings::{self, Configurable as _},
    Context,
};
use libfuzzer_sys::fuzz_target;

use cranelift_fuzzgen::*;
use target_lexicon::Triple;

fuzz_target!(|func: SingleFunction| {
    let mut func = func.0;

    let flags = settings::Flags::new({
        let mut builder = settings::builder();
        // We need llvm ABI extensions for i128 values on x86
        builder.set("enable_llvm_abi_extensions", "true").unwrap();

        // This is the default, but we should ensure that it wasn't accidentally turned off anywhere.
        builder.set("enable_verifier", "true").unwrap();

        builder
    });

    let isa_builder = isa::lookup(Triple::host())
        .map_err(|err| match err {
            isa::LookupError::SupportDisabled => {
                "support for architecture disabled at compile time"
            }
            isa::LookupError::Unsupported => "unsupported architecture",
        })
        .unwrap();

    let isa = isa_builder.finish(flags).unwrap();

    let cache_key_hash = icache::compute_cache_key(&*isa, &mut func);

    let mut context = Context::for_function(func.clone());
    let prev_stencil = match context.compile_stencil(&*isa) {
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
                } = cursor.func.dfg[inst]
                {
                    let imm = imm.bits();
                    cursor.func.dfg[inst] = ir::InstructionData::UnaryImm {
                        opcode: ir::Opcode::Iconst,
                        imm: Imm64::new(imm.checked_add(1).unwrap_or_else(|| imm - 1)),
                    };
                } else {
                    cursor.func.dfg[inst] = ir::InstructionData::UnaryImm {
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

    let new_cache_key_hash = icache::compute_cache_key(&*isa, &mut func);

    if expect_cache_hit {
        assert!(cache_key_hash == new_cache_key_hash);
    } else {
        assert!(cache_key_hash != new_cache_key_hash);
    }

    context = Context::for_function(func.clone());

    let after_mutation_result = match context.compile(&*isa) {
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
