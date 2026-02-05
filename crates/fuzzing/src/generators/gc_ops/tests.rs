use crate::generators::gc_ops::{
    limits::GcOpsLimits,
    ops::{GcOp, GcOps, OP_NAMES},
    types::{RecGroupId, TypeId, Types},
};
use mutatis;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use wasmparser;
use wasmprinter;

/// Creates empty GcOps
fn empty_test_ops() -> GcOps {
    let mut t = GcOps {
        limits: GcOpsLimits {
            num_params: 5,
            num_globals: 5,
            table_size: 5,
            max_rec_groups: 5,
            max_types: 5,
        },
        ops: vec![],
        types: Types::new(),
    };
    for i in 0..t.limits.max_rec_groups {
        t.types.insert_rec_group(RecGroupId(i));
    }
    t
}

/// Creates GcOps with all default opcodes
fn test_ops(num_params: u32, num_globals: u32, table_size: u32) -> GcOps {
    let mut t = GcOps {
        limits: GcOpsLimits {
            num_params,
            num_globals,
            table_size,
            max_rec_groups: 7,
            max_types: 10,
        },
        ops: vec![
            GcOp::NullExtern,
            GcOp::Drop,
            GcOp::Gc,
            GcOp::LocalSet { local_index: 0 },
            GcOp::LocalGet { local_index: 0 },
            GcOp::GlobalSet { global_index: 0 },
            GcOp::GlobalGet { global_index: 0 },
            GcOp::StructNew { type_index: 0 },
        ],
        types: Types::new(),
    };

    for i in 0..t.limits.max_rec_groups {
        t.types.insert_rec_group(RecGroupId(i));
    }

    let mut rng = StdRng::seed_from_u64(0xC0FFEE);
    if t.limits.max_rec_groups > 0 {
        for i in 0..t.limits.max_types {
            let gid = RecGroupId(rng.gen_range(0..t.limits.max_rec_groups));
            t.types.insert_empty_struct(TypeId(i), gid);
        }
    }

    t
}

#[test]
fn mutate_gc_ops_with_default_mutator() -> mutatis::Result<()> {
    let _ = env_logger::try_init();

    let mut features = wasmparser::WasmFeatures::default();
    features.insert(wasmparser::WasmFeatures::REFERENCE_TYPES);
    features.insert(wasmparser::WasmFeatures::FUNCTION_REFERENCES);
    features.insert(wasmparser::WasmFeatures::GC_TYPES);
    features.insert(wasmparser::WasmFeatures::GC);

    let mut ops = test_ops(5, 5, 5);

    let mut session = mutatis::Session::new();
    for _ in 0..2048 {
        session.mutate(&mut ops)?;

        let wasm = ops.to_wasm_binary();
        crate::oracles::log_wasm(&wasm);

        let mut validator = wasmparser::Validator::new_with_features(features);
        if let Err(e) = validator.validate_all(&wasm) {
            let mut config = wasmprinter::Config::new();
            config.print_offsets(true);
            config.print_operand_stack(true);
            let mut wat = String::new();
            let wat = match config.print(&wasm, &mut wasmprinter::PrintFmtWrite(&mut wat)) {
                Ok(()) => wat,
                Err(e) => format!("<failed to disassemble Wasm binary to WAT: {e}>"),
            };
            panic!(
                "Emitted Wasm binary is not valid!\n\n\
                 === Validation Error ===\n\n\
                 {e}\n\n\
                 === GcOps ===\n\n\
                 {ops:#?}\n\n\
                 === Wat ===\n\n\
                 {wat}"
            );
        }
    }
    Ok(())
}

#[test]
fn struct_new_removed_when_no_types() -> mutatis::Result<()> {
    let _ = env_logger::try_init();

    let mut ops = test_ops(0, 0, 0);
    ops.limits.max_types = 0;
    ops.ops = vec![GcOp::StructNew { type_index: 42 }];

    ops.fixup();
    assert!(
        ops.ops
            .iter()
            .all(|op| !matches!(op, GcOp::StructNew { .. })),
        "StructNew should be removed when there are no types"
    );
    Ok(())
}

#[test]
fn local_ops_removed_when_no_params() -> mutatis::Result<()> {
    let _ = env_logger::try_init();

    let mut ops = test_ops(0, 0, 0);
    ops.limits.num_params = 0;
    ops.ops = vec![
        GcOp::LocalGet { local_index: 42 },
        GcOp::LocalSet { local_index: 99 },
    ];

    ops.fixup();
    assert!(
        ops.ops
            .iter()
            .all(|op| !matches!(op, GcOp::LocalGet { .. } | GcOp::LocalSet { .. })),
        "LocalGet/LocalSet should be removed when there are no params"
    );
    Ok(())
}

#[test]
fn global_ops_removed_when_no_globals() -> mutatis::Result<()> {
    let _ = env_logger::try_init();

    let mut ops = test_ops(0, 0, 0);
    ops.limits.num_globals = 0;
    ops.ops = vec![
        GcOp::GlobalGet { global_index: 42 },
        GcOp::GlobalSet { global_index: 99 },
    ];

    ops.fixup();
    assert!(
        ops.ops
            .iter()
            .all(|op| !matches!(op, GcOp::GlobalGet { .. } | GcOp::GlobalSet { .. })),
        "GlobalGet/GlobalSet should be removed when there are no globals"
    );
    Ok(())
}

#[test]
fn every_op_generated() -> mutatis::Result<()> {
    let _ = env_logger::try_init();
    let mut unseen_ops: std::collections::HashSet<_> = OP_NAMES.iter().copied().collect();

    let mut res = empty_test_ops();
    let mut session = mutatis::Session::new().seed(0xC0FFEE);

    'outer: for _ in 0..=1024 {
        session.mutate(&mut res)?;
        for op in &res.ops {
            unseen_ops.remove(op.name());
            if unseen_ops.is_empty() {
                break 'outer;
            }
        }
    }

    assert!(unseen_ops.is_empty(), "Failed to generate {unseen_ops:?}");
    Ok(())
}

#[test]
fn emits_empty_rec_groups_and_validates() -> mutatis::Result<()> {
    let _ = env_logger::try_init();

    let mut ops = test_ops(5, 5, 5);

    let wasm = ops.to_wasm_binary();

    let feats = wasmparser::WasmFeatures::default();
    feats.reference_types();
    feats.gc();
    let mut validator = wasmparser::Validator::new_with_features(feats);
    assert!(
        validator.validate_all(&wasm).is_ok(),
        "GC validation failed"
    );

    let wat = wasmprinter::print_bytes(&wasm).expect("to WAT");
    let recs = wat.matches("(rec").count();
    let structs = wat.matches("(struct)").count();

    assert_eq!(recs, 7, "expected 2 (rec) blocks, got {recs}");
    assert_eq!(structs, 10, "expected no struct types, got {structs}");

    Ok(())
}

#[test]
fn fixup_check_types_and_indexes() -> mutatis::Result<()> {
    let _ = env_logger::try_init();

    let mut ops = test_ops(5, 5, 5);

    // These `GcOp`s do not have their operands satisfied, and their results are
    // not the operands of the next op, so `fixup` will need to deal with
    // that. Additionally, their immediates are out-of-bounds of their
    // respective index spaces, which `fixup` will also need to address.
    ops.ops = vec![
        GcOp::TakeTypedStructCall {
            type_index: ops.limits.max_types + 7,
        },
        GcOp::GlobalSet {
            global_index: ops.limits.num_globals * 2,
        },
        GcOp::StructNew {
            type_index: ops.limits.max_types + 9,
        },
        GcOp::LocalSet {
            local_index: ops.limits.num_params * 5,
        },
    ];

    // Call `fixup` to insert missing types, rewrite the immediates such that
    // they are within their bounds, insert missing operands, and drop unused
    // results.
    ops.fixup();

    // Check that we got the expected `GcOp` sequence after `fixup`:
    assert_eq!(
        ops.ops,
        [
            // Inserted by `fixup` to satisfy `TakeTypedStructCall`'s operands.
            GcOp::StructNew { type_index: 7 },
            // The `type_index` is now valid.
            GcOp::TakeTypedStructCall { type_index: 7 },
            // Inserted by `fixup` to satisfy `GlobalSet`'s operands.
            GcOp::NullExtern,
            // The `global_index` is now valid.
            GcOp::GlobalSet { global_index: 0 },
            // The `type_index` is now valid.
            GcOp::StructNew { type_index: 9 },
            // Inserted by `fixup` to satisfy `LocalSet`'s operands.
            GcOp::NullExtern,
            // The `local_index` is now valid.
            GcOp::LocalSet { local_index: 0 },
            // Inserted by `fixup` to make sure the operand stack is empty at
            // the end of the block.
            GcOp::Drop,
        ]
    );

    // Verify that we generate a valid Wasm binary after calling `fixup`.
    let wasm = ops.to_wasm_binary();
    let wat = wasmprinter::print_bytes(&wasm).unwrap();
    log::debug!("{wat}");
    let feats = wasmparser::WasmFeatures::default();
    feats.reference_types();
    feats.gc();
    let mut validator = wasmparser::Validator::new_with_features(feats);
    assert!(
        validator.validate_all(&wasm).is_ok(),
        "GC validation should pass after fixup"
    );

    Ok(())
}
