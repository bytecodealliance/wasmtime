use crate::generators::gc_ops::{
    limits::GcOpsLimits,
    ops::{GcOp, GcOps, OP_NAMES},
    types::{RecGroupId, StackType, TypeId, Types},
};
use mutatis;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use wasmparser;
use wasmprinter;

/// Flattened encoding order for use in tests.
fn encoding_order(types: &Types) -> Vec<TypeId> {
    let type_to_group = types.type_to_group_map();
    let mut grouped = Vec::new();
    types.encoding_order_grouped(&mut grouped, &type_to_group);
    grouped
        .into_iter()
        .flat_map(|(_, members)| members)
        .collect()
}

/// Returns true iff `sub_index` is the same as or a subtype of `sup_index`.
///
/// The `encoding_order` slice maps dense indices (0, 1, 2, …) to
/// [`TypeId`]s in the same order they appear in the encoded Wasm binary.
fn is_subtype_index(
    types: &Types,
    sub_index: u32,
    sup_index: u32,
    encoding_order: &[TypeId],
) -> bool {
    if sub_index == sup_index {
        return true;
    }

    let sub = match encoding_order
        .get(usize::try_from(sub_index).expect("sub_index is out of bounds"))
        .copied()
    {
        Some(t) => t,
        None => return false,
    };
    let sup = match encoding_order
        .get(usize::try_from(sup_index).expect("sup_index is out of bounds"))
        .copied()
    {
        Some(t) => t,
        None => return false,
    };

    types.is_subtype(sub, sup)
}

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

    ops.fixup(&mut Vec::new());
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

    ops.fixup(&mut Vec::new());
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

    ops.fixup(&mut Vec::new());
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
fn emits_rec_groups_and_validates() -> mutatis::Result<()> {
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

    assert_eq!(
        recs,
        ops.types.rec_groups.len(),
        "one (rec) block per rec group"
    );
    assert_eq!(
        structs,
        ops.types.type_defs.len(),
        "one (struct) per struct type in type_defs"
    );

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
    ops.fixup(&mut Vec::new());

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
fn sort_types_by_supertype_orders_supertype_before_subtype_across_rec_groups() {
    let mut types = Types::new();
    let ga = RecGroupId(0);
    let gb = RecGroupId(1);
    let gc = RecGroupId(2);
    let gd = RecGroupId(3);
    types.insert_rec_group(ga);
    types.insert_rec_group(gb);
    types.insert_rec_group(gc);
    types.insert_rec_group(gd);

    let a = TypeId(0);
    let b = TypeId(1);
    let c = TypeId(2);
    let d = TypeId(3);

    // Cross-rec-group chain: C <: A <: B <: D.
    types.insert_empty_struct(a, ga, false, Some(b)); // A <: B
    types.insert_empty_struct(b, gb, false, Some(d)); // B <: D
    types.insert_empty_struct(c, gc, false, Some(a)); // C <: A
    types.insert_empty_struct(d, gd, false, None); // D

    let mut sorted = Vec::new();
    types.sort_types_topo(&mut sorted);

    // Rec-group boundaries do not change topological ordering by supertype.
    assert_eq!(
        sorted,
        [TypeId(3), TypeId(1), TypeId(0), TypeId(2)],
        "topo order: supertype before subtype across rec groups"
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

    types.fixup(&limits, &mut Vec::new());

    assert_eq!(types.rec_group_of(super_ty), Some(g));
    assert_eq!(types.rec_group_of(sub_ty), Some(g));
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

    types.fixup(&limits, &mut Vec::new());

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
fn sort_rec_groups_topo_orders_dependencies_first() {
    let _ = env_logger::try_init();

    let mut types = Types::new();

    let g0 = RecGroupId(0);
    let g1 = RecGroupId(1);
    let g2 = RecGroupId(2);
    let g3 = RecGroupId(3);

    types.insert_rec_group(g0);
    types.insert_rec_group(g1);
    types.insert_rec_group(g2);
    types.insert_rec_group(g3);

    let a = TypeId(0);
    let b = TypeId(1);
    let c = TypeId(2);
    let d = TypeId(3);
    let e = TypeId(4);
    let f = TypeId(5);

    types.insert_empty_struct(a, g0, false, Some(b)); // g0 -> g1
    types.insert_empty_struct(b, g1, false, Some(c)); // g1 -> g2
    types.insert_empty_struct(c, g2, false, Some(d)); // g2 ->g3
    types.insert_empty_struct(d, g3, false, None);
    types.insert_empty_struct(e, g0, false, None);
    types.insert_empty_struct(f, g2, false, None);

    let type_to_group = types.type_to_group_map();
    let mut sorted = Vec::new();
    types.sort_rec_groups_topo(&mut sorted, &type_to_group);

    // g3 has no deps, g2 depends on g3, g1 on g2, g0 on g1.
    assert_eq!(
        sorted,
        [RecGroupId(3), RecGroupId(2), RecGroupId(1), RecGroupId(0)],
        "topo order: depended-on groups before dependent groups"
    );
}

#[test]
fn break_rec_group_cycles() {
    let _ = env_logger::try_init();

    let mut types = Types::new();

    let g0 = RecGroupId(0);
    let g1 = RecGroupId(1);
    let g2 = RecGroupId(2);
    let g3 = RecGroupId(3);

    types.insert_rec_group(g0);
    types.insert_rec_group(g1);
    types.insert_rec_group(g2);
    types.insert_rec_group(g3);

    let a0 = TypeId(0);
    let a1 = TypeId(1);
    let b0 = TypeId(2);
    let b1 = TypeId(3);
    let c0 = TypeId(4);
    let c1 = TypeId(5);
    let c2 = TypeId(6);
    let d0 = TypeId(7);
    let d1 = TypeId(8);

    // Before: t
    //
    //    ----------------------------------
    //    |          outer cycle           │
    //    v                                │
    //  +----+       +----+       +----+   │
    //  | g0 |------>| g1 |------>| g2 |---
    //  +----+       +----+       +----+
    //                 ^            │
    //                 │  inner     │
    //                 │  cycle     v
    //                 │          +----+
    //                 -----------| g3 |
    //                            +----+
    //
    // After: back edges dropped, clean chain
    //
    //  +----+       +----+       +----+       +----+
    //  | g0 |------>| g1 |------>| g2 |------>| g3 |
    //  +----+       +----+       +----+       +----+

    types.insert_empty_struct(a0, g0, false, Some(b0)); // g0 -> g1
    types.insert_empty_struct(a1, g0, false, None);

    types.insert_empty_struct(b0, g1, false, None);
    types.insert_empty_struct(b1, g1, false, Some(c0)); // g1 -> g2

    types.insert_empty_struct(c0, g2, false, None);
    types.insert_empty_struct(c1, g2, false, Some(a1)); // g2 -> g0 (outer back edge)
    types.insert_empty_struct(c2, g2, false, Some(d0)); // g2 -> g3

    types.insert_empty_struct(d0, g3, false, None);
    types.insert_empty_struct(d1, g3, false, Some(b0)); // g3 -> g1 (inner back edge)

    // Type graph is acyclic — breaking supertype cycles changes nothing.
    types.break_supertype_cycles();
    assert_eq!(types.type_defs.get(&a0).unwrap().supertype, Some(b0));
    assert_eq!(types.type_defs.get(&b1).unwrap().supertype, Some(c0));
    assert_eq!(types.type_defs.get(&c1).unwrap().supertype, Some(a1));
    assert_eq!(types.type_defs.get(&c2).unwrap().supertype, Some(d0));
    assert_eq!(types.type_defs.get(&d1).unwrap().supertype, Some(b0));

    assert_eq!(types.rec_groups.len(), 4);

    let type_to_group = types.type_to_group_map();
    types.break_rec_group_cycles(&type_to_group);

    // All four groups preserved.
    assert_eq!(types.rec_groups.len(), 4);
    assert!(types.rec_groups.contains_key(&g0));
    assert!(types.rec_groups.contains_key(&g1));
    assert!(types.rec_groups.contains_key(&g2));
    assert!(types.rec_groups.contains_key(&g3));

    // Back edge (g2->g0): c1's supertype cleared.
    assert_eq!(types.type_defs.get(&c1).unwrap().supertype, None);

    // Back edge (g3->g1): d1's supertype cleared.
    assert_eq!(types.type_defs.get(&d1).unwrap().supertype, None);

    // All other cross-group supertypes preserved.
    assert_eq!(types.type_defs.get(&a0).unwrap().supertype, Some(b0));
    assert_eq!(types.type_defs.get(&b1).unwrap().supertype, Some(c0));
    assert_eq!(types.type_defs.get(&c2).unwrap().supertype, Some(d0));

    // Result is a clean chain: g0 -> g1 -> g2 -> g3
    let type_to_group = types.type_to_group_map();
    let mut topo = Vec::new();
    types.sort_rec_groups_topo(&mut topo, &type_to_group);
    assert_eq!(topo.len(), 4);
    assert_eq!(topo, vec![g3, g2, g1, g0]);
}

#[test]
fn is_subtype_index_accepts_chain() {
    let _ = env_logger::try_init();

    let mut types = Types::new();
    let g0 = RecGroupId(0);
    let g1 = RecGroupId(1);
    let g2 = RecGroupId(2);
    let g3 = RecGroupId(3);

    types.insert_rec_group(g0);
    types.insert_rec_group(g1);
    types.insert_rec_group(g2);
    types.insert_rec_group(g3);

    // Build chain: 1 <- 2 <- 3
    //
    // TypeId(1): g0
    // TypeId(2): subtype of 1
    // TypeId(3): g2
    // TypeId(4): g3
    //
    // Since type_defs is a BTreeMap, the dense index order used by
    // is_subtype_index is:
    //
    //   0 -> TypeId(1)
    //   1 -> TypeId(2)
    //   2 -> TypeId(3)
    types.insert_empty_struct(TypeId(1), g0, false, None);
    types.insert_empty_struct(TypeId(2), g1, false, Some(TypeId(1)));
    types.insert_empty_struct(TypeId(3), g2, false, Some(TypeId(2)));
    types.insert_empty_struct(TypeId(4), g3, false, Some(TypeId(3)));

    let order = encoding_order(&types);

    // self
    assert!(is_subtype_index(&types, 0, 0, &order)); // 1 <: 1
    assert!(is_subtype_index(&types, 1, 1, &order)); // 2 <: 2
    assert!(is_subtype_index(&types, 2, 2, &order)); // 3 <: 3

    // requested checks
    assert!(is_subtype_index(&types, 1, 0, &order)); // 2 <: 1
    assert!(is_subtype_index(&types, 2, 0, &order)); // 3 <: 1
    assert!(is_subtype_index(&types, 2, 1, &order)); // 3 <: 2

    // reverse directions must fail
    assert!(!is_subtype_index(&types, 0, 1, &order)); // 1 </: 2
    assert!(!is_subtype_index(&types, 0, 2, &order)); // 1 </: 3
    assert!(!is_subtype_index(&types, 1, 2, &order)); // 2 </: 3
}

/// Encoding order can differ from BTreeMap key order when a higher-numbered
/// TypeId lives in a group that must be emitted *before* a lower-numbered
/// TypeId's group (because of cross-group supertype dependencies).
///
/// With plain BTreeMap key order the dense indices would be:
///   0 -> TypeId(1)   1 -> TypeId(10)
///
/// But the correct encoding order (group topo sort) is:
///   0 -> TypeId(10)  1 -> TypeId(1)
///
/// A naive key-order approach would conclude "index 0 (TypeId(1)) <: index 1
/// (TypeId(10))" is false (they're unrelated), while the real encoding order
/// says "index 1 (TypeId(1)) <: index 0 (TypeId(10))" is true.
#[test]
fn is_subtype_index_encoding_order_differs_from_key_order() {
    let _ = env_logger::try_init();

    let mut types = Types::new();
    let g0 = RecGroupId(0);
    let g1 = RecGroupId(1);

    types.insert_rec_group(g0);
    types.insert_rec_group(g1);

    // TypeId(10) in g0: the supertype (no parent).
    // TypeId(1)  in g1: subtype of TypeId(10).
    //
    // BTreeMap key order:  [TypeId(1), TypeId(10)]  -> dense 0=TypeId(1), 1=TypeId(10)
    // Encoding order:      [TypeId(10), TypeId(1)]  -> dense 0=TypeId(10), 1=TypeId(1)
    //   (g0 must come before g1 because g1's type has a supertype in g0)
    types.insert_empty_struct(TypeId(10), g0, false, None);
    types.insert_empty_struct(TypeId(1), g1, false, Some(TypeId(10)));

    let order = encoding_order(&types);

    // Verify that encoding order is indeed reversed from key order.
    assert_eq!(order, vec![TypeId(10), TypeId(1)]);

    // index 1 (TypeId(1)) is a subtype of index 0 (TypeId(10))
    assert!(is_subtype_index(&types, 1, 0, &order));

    // index 0 (TypeId(10)) is NOT a subtype of index 1 (TypeId(1))
    assert!(!is_subtype_index(&types, 0, 1, &order));

    // Also verify the direct TypeId-based method works.
    assert!(types.is_subtype(TypeId(1), TypeId(10)));
    assert!(!types.is_subtype(TypeId(10), TypeId(1)));
}

#[test]
fn stacktype_fixup_accepts_subtype_for_supertype_requirement() {
    let _ = env_logger::try_init();
    let mut types = Types::new();
    let g = RecGroupId(0);
    types.insert_rec_group(g);

    // Same chain: 1 <- 2 <- 3
    //
    // Dense indices:
    //   0 -> TypeId(1)
    //   1 -> TypeId(2)
    //   2 -> TypeId(3)
    types.insert_empty_struct(TypeId(1), g, false, None);
    types.insert_empty_struct(TypeId(2), g, false, Some(TypeId(1)));
    types.insert_empty_struct(TypeId(3), g, false, Some(TypeId(2)));

    let num_types = u32::try_from(types.type_defs.len()).unwrap();
    let order = encoding_order(&types);

    // Case 1: stack has subtype 3, op requires supertype 2.
    let mut stack = vec![StackType::Struct(Some(2))];
    let mut out = vec![];

    StackType::fixup(
        Some(StackType::Struct(Some(1))),
        &mut stack,
        &mut out,
        num_types,
        &types,
        &order,
    );

    // Accepted as-is:
    // - no fixup ops inserted
    // - operand consumed from stack
    assert!(
        out.is_empty(),
        "subtype 3 should satisfy required supertype 2"
    );
    assert!(stack.is_empty(), "accepted operand should be popped");

    // Case 2: stack has subtype 3, op requires supertype 1.
    let mut stack = vec![StackType::Struct(Some(2))];
    let mut out = vec![];

    StackType::fixup(
        Some(StackType::Struct(Some(0))),
        &mut stack,
        &mut out,
        num_types,
        &types,
        &order,
    );
    // Accepted as-is:
    assert!(
        out.is_empty(),
        "subtype 3 should satisfy required supertype 1"
    );
    assert!(stack.is_empty(), "accepted operand should be popped");

    // Case 3: stack has type 1, op requires subtype 2.
    let mut stack = vec![StackType::Struct(Some(0))];
    let mut out = vec![];

    StackType::fixup(
        Some(StackType::Struct(Some(1))),
        &mut stack,
        &mut out,
        num_types,
        &types,
        &order,
    );
    // Not accepted. Fixup should synthesize the requested concrete type.
    assert_eq!(out, vec![GcOp::StructNew { type_index: 1 }]);
    assert_eq!(stack, vec![StackType::Struct(Some(0))]);
}
