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
            GcOp::Null(),
            GcOp::Drop(),
            GcOp::Gc(),
            GcOp::LocalSet(0),
            GcOp::LocalGet(0),
            GcOp::GlobalSet(0),
            GcOp::GlobalGet(0),
            GcOp::StructNew(0),
        ],
        types: Types::new(),
    };
    for i in 0..t.limits.max_rec_groups {
        t.types.insert_rec_group(RecGroupId(i));
    }

    if t.limits.max_rec_groups > 0 {
        let mut rng = StdRng::seed_from_u64(0xC0FFEE);
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
    let mut res = test_ops(5, 5, 5);

    let mut session = mutatis::Session::new();

    for _ in 0..2048 {
        session.mutate(&mut res)?;
        let wasm = res.to_wasm_binary();

        let feats = wasmparser::WasmFeatures::default();
        feats.reference_types();
        feats.gc();
        let mut validator = wasmparser::Validator::new_with_features(feats);

        let wat = wasmprinter::print_bytes(&wasm).expect("[-] Failed .print_bytes(&wasm).");
        let result = validator.validate_all(&wasm);
        log::debug!("{wat}");
        println!("{wat}");
        assert!(
            result.is_ok(),
            "\n[-] Invalid wat: {}\n\t\t==== Failed Wat ====\n{}",
            result.err().expect("[-] Failed .err() in assert macro."),
            wat
        );
    }
    Ok(())
}

#[test]
fn struct_new_removed_when_no_types() -> mutatis::Result<()> {
    let _ = env_logger::try_init();

    let mut ops = test_ops(0, 0, 0);
    ops.limits.max_types = 0;
    ops.ops = vec![GcOp::StructNew(42)];

    let _ = ops.fixup();

    assert!(
        ops.ops.iter().all(|op| !matches!(op, GcOp::StructNew(..))),
        "StructNew should be removed when there are no types"
    );
    Ok(())
}

#[test]
fn local_ops_removed_when_no_params() -> mutatis::Result<()> {
    let _ = env_logger::try_init();

    let mut ops = test_ops(0, 0, 0);
    ops.limits.num_params = 0;
    ops.ops = vec![GcOp::LocalGet(42), GcOp::LocalSet(99)];

    ops.fixup();

    assert!(
        ops.ops
            .iter()
            .all(|op| !matches!(op, GcOp::LocalGet(..) | GcOp::LocalSet(..))),
        "LocalGet/LocalSet should be removed when there are no params"
    );
    Ok(())
}

#[test]
fn global_ops_removed_when_no_globals() -> mutatis::Result<()> {
    let _ = env_logger::try_init();

    let mut ops = test_ops(0, 0, 0);
    ops.limits.num_globals = 0;
    ops.ops = vec![GcOp::GlobalGet(42), GcOp::GlobalSet(99)];

    ops.fixup();

    assert!(
        ops.ops
            .iter()
            .all(|op| !matches!(op, GcOp::GlobalGet(..) | GcOp::GlobalSet(..))),
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
fn test_wat_string() -> mutatis::Result<()> {
    let _ = env_logger::try_init();

    let mut gc_ops = test_ops(2, 2, 5);

    let wasm = gc_ops.to_wasm_binary();

    let actual_wat = wasmprinter::print_bytes(&wasm).expect("Failed to convert to WAT");
    let actual_wat = actual_wat.trim();

    let expected_wat = r#"
(module
  (type (;0;) (func (result externref externref externref)))
  (type (;1;) (func (param externref externref)))
  (type (;2;) (func (param externref externref externref)))
  (type (;3;) (func (result externref externref externref)))
  (type (;4;) (func (param (ref any))))
  (rec
    (type (;5;) (struct))
  )
  (rec)
  (rec
    (type (;6;) (struct))
  )
  (rec
    (type (;7;) (struct))
    (type (;8;) (struct))
    (type (;9;) (struct))
  )
  (rec
    (type (;10;) (struct))
    (type (;11;) (struct))
  )
  (rec
    (type (;12;) (struct))
    (type (;13;) (struct))
  )
  (rec
    (type (;14;) (struct))
  )
  (type (;15;) (func (param (ref 5))))
  (type (;16;) (func (param (ref 6))))
  (type (;17;) (func (param (ref 7))))
  (type (;18;) (func (param (ref 8))))
  (type (;19;) (func (param (ref 9))))
  (type (;20;) (func (param (ref 10))))
  (type (;21;) (func (param (ref 11))))
  (type (;22;) (func (param (ref 12))))
  (type (;23;) (func (param (ref 13))))
  (type (;24;) (func (param (ref 14))))
  (import "" "gc" (func (;0;) (type 0)))
  (import "" "take_refs" (func (;1;) (type 2)))
  (import "" "make_refs" (func (;2;) (type 3)))
  (import "" "take_struct" (func (;3;) (type 4)))
  (import "" "take_struct_5" (func (;4;) (type 15)))
  (import "" "take_struct_6" (func (;5;) (type 16)))
  (import "" "take_struct_7" (func (;6;) (type 17)))
  (import "" "take_struct_8" (func (;7;) (type 18)))
  (import "" "take_struct_9" (func (;8;) (type 19)))
  (import "" "take_struct_10" (func (;9;) (type 20)))
  (import "" "take_struct_11" (func (;10;) (type 21)))
  (import "" "take_struct_12" (func (;11;) (type 22)))
  (import "" "take_struct_13" (func (;12;) (type 23)))
  (import "" "take_struct_14" (func (;13;) (type 24)))
  (table (;0;) 5 externref)
  (global (;0;) (mut externref) ref.null extern)
  (global (;1;) (mut externref) ref.null extern)
  (export "run" (func 14))
  (func (;14;) (type 1) (param externref externref)
    (local externref)
    loop ;; label = @1
      ref.null extern
      drop
      call 0
      local.set 0
      local.get 0
      global.set 0
      global.get 0
      struct.new 5
      drop
      drop
      drop
      drop
      br 0 (;@1;)
    end
  )
)
    "#;
    let expected_wat = expected_wat.trim();

    eprintln!("=== actual ===\n{actual_wat}");
    eprintln!("=== expected ===\n{expected_wat}");
    assert_eq!(
        actual_wat, expected_wat,
        "actual WAT does not match expected"
    );

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

    // Create GcOps with 5 types so that 7 % 5 = 2
    let mut ops = test_ops(5, 5, 5);
    ops.limits.max_types = 5;

    // We create max types 5 and out ouf bounds for the type index
    ops.ops = vec![
        GcOp::TakeTypedStructCall(27),
        GcOp::GlobalSet(0),
        GcOp::StructNew(24),
        GcOp::LocalSet(0),
    ];

    // Call fixup to resolve dependencies
    // this should fix the types by inserting missing types
    // also put the indexes in the correct bounds
    ops.fixup();

    // Verify that fixup()
    // The expected sequence should be:
    // 1. StructNew(_) - inserted by fixup to satisfy TakeTypedStructCall(_)
    // 2. TakeTypedStructCall(_) - now has Struct(_) on stack
    // 3. Null() - inserted by fixup to satisfy GlobalSet(0)
    // 4. GlobalSet(0) - now has ExternRef on stack
    // 5. StructNew(_) - produces Struct(_)
    // 6. Null() - inserted by fixup to satisfy LocalSet(0)
    // 7. LocalSet(0) - now has ExternRef on stack
    // 8. Drop() - inserted by fixup to consume ExternRef before Drop()

    // This is the expected sequence in wat format:
    //  loop ;; label = @1
    //     struct.new 7
    //     call 6
    //     ref.null extern
    //     global.set 0
    //     struct.new 9
    //     ref.null extern
    //     local.set 0
    //     drop
    //     br 0 (;@1;)
    // end

    // Find the index of TakeTypedStructCall(_) after fixup
    let take_call_idx = ops
        .ops
        .iter()
        .position(|op| matches!(op, GcOp::TakeTypedStructCall(_)))
        .expect("TakeTypedStructCall(_) should be present after fixup");

    // Verify that StructNew(_) appears before TakeTypedStructCall(_)
    let struct_new_2_before = ops
        .ops
        .iter()
        .take(take_call_idx)
        .any(|op| matches!(op, GcOp::StructNew(_)));

    assert!(
        struct_new_2_before,
        "fixup should insert StructNew(_) before TakeTypedStructCall(_) to satisfy the dependency"
    );

    // Verify the sequence validates correctly
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
