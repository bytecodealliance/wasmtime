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
            let is_final = false;
            let supertype = None;
            t.types
                .insert_empty_struct(TypeId(i), gid, is_final, supertype);
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

#[test]
fn sort_types_by_supertype_orders_supertype_before_subtype() {
    let mut types = Types::new();
    let g = RecGroupId(0);
    types.insert_rec_group(g);

    let a = TypeId(0);
    let b = TypeId(1);
    let c = TypeId(2);
    let d = TypeId(3);

    types.insert_empty_struct(a, g, false, Some(b)); // A <: B
    types.insert_empty_struct(b, g, false, Some(d)); // B <: D
    types.insert_empty_struct(c, g, false, Some(a)); // C <: A
    types.insert_empty_struct(d, g, false, None); // D

    let mut sorted = Vec::new();
    types.sort_types_by_supertype(&mut sorted);

    // D(3) root, B(1)<:D, A(0)<:B, C(2)<:A => supertype-before-subtype order.
    assert_eq!(
        sorted,
        [TypeId(3), TypeId(1), TypeId(0), TypeId(2)],
        "topo order: supertype before subtype"
    );
}

#[test]
fn fixup_preserves_subtyping_within_same_rec_group() {
    let _ = env_logger::try_init();

    let mut types = Types::new();
    let g = RecGroupId(0);
    types.insert_rec_group(g);

    let super_ty = TypeId(0);
    let sub_ty = TypeId(1);

    // Both types are in the same rec group.
    // The second subtypes the first.
    types.insert_empty_struct(super_ty, g, false, None);
    types.insert_empty_struct(sub_ty, g, false, Some(super_ty));

    let limits = GcOpsLimits {
        num_params: 0,
        num_globals: 0,
        table_size: 0,
        max_rec_groups: 10,
        max_types: 10,
    };

    types.fixup(&limits);

    assert_eq!(types.type_defs.get(&super_ty).unwrap().rec_group, g);
    assert_eq!(types.type_defs.get(&sub_ty).unwrap().rec_group, g);
    assert_eq!(
        types.type_defs.get(&sub_ty).unwrap().supertype,
        Some(super_ty)
    );
}

#[test]
fn fixup_breaks_one_edge_in_multi_rec_group_type_cycle() {
    let _ = env_logger::try_init();

    let mut types = Types::new();

    let g_a = RecGroupId(0);
    let g_bc = RecGroupId(1);
    let g_d = RecGroupId(2);

    types.insert_rec_group(g_a);
    types.insert_rec_group(g_bc);
    types.insert_rec_group(g_d);

    let a = TypeId(0);
    let b = TypeId(1);
    let c = TypeId(2);
    let d = TypeId(3);

    // Rec(a)
    types.insert_empty_struct(a, g_a, false, Some(d));

    // Rec(b, c)
    types.insert_empty_struct(b, g_bc, false, None);
    types.insert_empty_struct(c, g_bc, false, Some(a));

    // Rec(d)
    types.insert_empty_struct(d, g_d, false, Some(c));

    let limits = GcOpsLimits {
        num_params: 0,
        num_globals: 0,
        table_size: 0,
        max_rec_groups: 10,
        max_types: 10,
    };

    types.fixup(&limits);

    let a_super = types.type_defs.get(&a).unwrap().supertype;
    let c_super = types.type_defs.get(&c).unwrap().supertype;
    let d_super = types.type_defs.get(&d).unwrap().supertype;

    let cleared = [a_super, c_super, d_super]
        .into_iter()
        .filter(|st| st.is_none())
        .count();

    assert!(
        cleared == 1,
        "fixup should clear exactly one edge to break the cycle"
    );
}

#[test]
fn merge_rec_groups_via_scc_merges_group_cycle_without_type_cycle() {
    let mut types = Types::new();

    let g0 = RecGroupId(0);
    let g1 = RecGroupId(1);
    let g2 = RecGroupId(2);

    types.insert_rec_group(g0);
    types.insert_rec_group(g1);
    types.insert_rec_group(g2);

    let a0 = TypeId(0);
    let a1 = TypeId(1);
    let b0 = TypeId(2);
    let b1 = TypeId(3);
    let c0 = TypeId(4);
    let c1 = TypeId(5);

    // g0 = {a0, a1}
    // g1 = {b0, b1}
    // g2 = {c0, c1}
    //
    // Cross-group subtype edges:
    //   a0 <: b0   => g0 -> g1
    //   b1 <: c0   => g1 -> g2
    //   c1 <: a1   => g2 -> g0
    //
    // This creates a cycle in the rec-group dependency graph:
    //   g0 -> g1 -> g2 -> g0
    //
    // But the type graph itself is acyclic, because these are three separate
    // subtype edges on different types:
    //   a0 -> b0
    //   b1 -> c0
    //   c1 -> a1
    //
    // Therefore, breaking type cycles is not enough here. Merging rec-group
    // SCCs is what resolves the cyclic dependency among rec groups.

    types.insert_empty_struct(a0, g0, false, Some(b0));
    types.insert_empty_struct(a1, g0, false, None);

    types.insert_empty_struct(b0, g1, false, None);
    types.insert_empty_struct(b1, g1, false, Some(c0));

    types.insert_empty_struct(c0, g2, false, None);
    types.insert_empty_struct(c1, g2, false, Some(a1));

    // There is no type cycle, so breaking supertype cycles should not change anything.
    types.break_supertype_cycles();

    assert_eq!(types.type_defs.get(&a0).unwrap().supertype, Some(b0));
    assert_eq!(types.type_defs.get(&b1).unwrap().supertype, Some(c0));
    assert_eq!(types.type_defs.get(&c1).unwrap().supertype, Some(a1));

    assert_eq!(types.rec_groups.len(), 3);

    types.merge_rec_group_sccs();

    // After merge: one canonical group (g0), all types in it.
    assert_eq!(types.rec_groups.len(), 1);
    assert!(types.rec_groups.contains(&g0));
    assert!(!types.rec_groups.contains(&g1));
    assert!(!types.rec_groups.contains(&g2));

    for ty in [a0, a1, b0, b1, c0, c1] {
        assert_eq!(types.type_defs.get(&ty).unwrap().rec_group, g0);
    }

    // And importantly, the valid supertype edges should still be preserved.
    assert_eq!(types.type_defs.get(&a0).unwrap().supertype, Some(b0));
    assert_eq!(types.type_defs.get(&b1).unwrap().supertype, Some(c0));
    assert_eq!(types.type_defs.get(&c1).unwrap().supertype, Some(a1));
}
