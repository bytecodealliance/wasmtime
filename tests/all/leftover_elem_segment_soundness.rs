//! Regression tests for the leftover-active-`elem`-segment soundness
//! gate on the `call_indirect` elisions added in commits 2-4 of the
//! per-table-mutability stack.
//!
//! Originally caught by an automated reviewer on PR #2 of the
//! rebeckerspecialties/wasmtime fork
//! (`https://github.com/rebeckerspecialties/wasmtime/pull/2#
//! discussion_r3193374159` and `…#discussion_r3193374164`). The bug
//! shape is:
//!
//! - `try_static_resolve_indirect_call`,
//!   `try_elide_sig_check_for_immutable_table`, and
//!   `precomputed_table_has_no_null_slots` read from
//!   `module.table_initialization.initial_values[t].precomputed`.
//! - `try_func_table_init` only folds *some* active `elem` segments
//!   into `precomputed`. Anything with a dynamic offset, an
//!   `Expressions`-form payload, an out-of-range top, etc. stays in
//!   `module.table_initialization.segments` and runs at instantiation
//!   time *after* the precomputed image is applied.
//! - Without the soundness gate, the elisions trust `precomputed` but
//!   a leftover segment can overwrite slots with a different funcref,
//!   a function of a different signature, or a null. That defeats the
//!   sig-check, the null-check, and (for constant-index calls) the
//!   direct-call rewrite.
//!
//! The fix: `analyze_table_mutability` now also marks any table
//! targeted by a leftover segment as mutated, so the predicates above
//! correctly bail out. These tests instantiate the witness modules
//! end-to-end and assert that the runtime catches the segment-driven
//! mismatch — direct evidence the elisions did NOT fire.

use wasmtime::*;

/// Type-confusion witness: precomputed says slot 0 is a `() -> i32`
/// function, but a leftover segment with a dynamic offset (resolved
/// at instantiate-time from an imported global) overwrites slot 0
/// with a `() -> f64` function. The runtime sig check must fire and
/// trap the call_indirect; if the sig-check elision had erroneously
/// fired, the call would silently dispatch into `$f_f64` and pop an
/// `i32` off a wrong-shape return.
#[test]
#[cfg_attr(miri, ignore)]
fn leftover_segment_dynamic_offset_keeps_sig_check() -> Result<()> {
    let wat = r#"
        (module
          (import "" "g" (global $g i32))
          (type $sig_i32 (func (result i32)))
          (type $sig_f64 (func (result f64)))
          (table 4 4 funcref)
          (func $f_i32 (type $sig_i32) i32.const 42)
          (func $f_f64 (type $sig_f64) f64.const 3.14)

          ;; Foldable: precomputed[0] = $f_i32 (sig_i32).
          (elem (i32.const 0) func $f_i32)

          ;; Leftover (dynamic offset): runs at instantiation, can
          ;; overwrite precomputed[0] with $f_f64 (sig_f64).
          (elem (offset (global.get $g)) func $f_f64)

          (func (export "call_at_zero") (result i32)
            i32.const 0
            call_indirect (type $sig_i32)))
    "#;

    let engine = Engine::default();
    let module = Module::new(&engine, wat)?;
    let mut store = Store::new(&engine, ());

    // Resolve $g to 0 so the leftover segment overwrites slot 0 with
    // the wrong-signature function.
    let g = Global::new(
        &mut store,
        GlobalType::new(ValType::I32, Mutability::Const),
        Val::I32(0),
    )?;
    let instance = Instance::new(&mut store, &module, &[g.into()])?;

    let call_at_zero = instance.get_typed_func::<(), i32>(&mut store, "call_at_zero")?;
    let err = call_at_zero
        .call(&mut store, ())
        .expect_err("call_indirect with sig mismatch must trap");

    let trap = err
        .downcast_ref::<Trap>()
        .copied()
        .unwrap_or_else(|| panic!("expected wasmtime Trap, got: {err:#}"));
    assert_eq!(
        trap,
        Trap::BadSignature,
        "the leftover-segment-overwrites-with-different-sig case must \
         hit the runtime sig check, not the (unsoundly-elided) static \
         match"
    );
    Ok(())
}

/// Null-deref witness: precomputed says every in-bounds slot is a
/// concrete funcref (no nulls), but an `Expressions`-form leftover
/// segment writes a `ref.null func` into slot 0. The runtime
/// funcref-NULL check must fire; if the null-check elision had
/// erroneously fired (`may_be_null = false`), the call would
/// dereference a null funcref pointer.
#[test]
#[cfg_attr(miri, ignore)]
fn leftover_segment_expressions_form_null_keeps_null_check() -> Result<()> {
    let wat = r#"
        (module
          (type $sig (func (result i32)))
          (table 3 3 funcref)
          (func $f1 (type $sig) i32.const 1)
          (func $f2 (type $sig) i32.const 2)
          (func $f3 (type $sig) i32.const 3)

          ;; Foldable: precomputed = [$f1, $f2, $f3] (no nulls).
          (elem (i32.const 0) func $f1 $f2 $f3)

          ;; Leftover: Expressions-form segment, not foldable. Writes
          ;; null into slot 0 at instantiation time.
          (elem (i32.const 0) funcref (item ref.null func))

          (func (export "call_at_zero") (result i32)
            i32.const 0
            call_indirect (type $sig)))
    "#;

    let engine = Engine::default();
    let module = Module::new(&engine, wat)?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;

    let call_at_zero = instance.get_typed_func::<(), i32>(&mut store, "call_at_zero")?;
    let err = call_at_zero
        .call(&mut store, ())
        .expect_err("call_indirect through a null funcref must trap");

    let trap = err
        .downcast_ref::<Trap>()
        .copied()
        .unwrap_or_else(|| panic!("expected wasmtime Trap, got: {err:#}"));
    assert_eq!(
        trap,
        Trap::IndirectCallToNull,
        "the leftover-segment-overwrites-with-null case must hit the \
         runtime funcref-NULL check"
    );
    Ok(())
}

/// Stale-direct-call witness: precomputed says slot 0 is `$f_a`, but
/// a leftover segment overwrites slot 0 with `$f_b`. With both
/// functions having the same signature, the sig check would NOT trap
/// — so this specifically tests that
/// `try_static_resolve_indirect_call` (the constant-index direct-call
/// rewrite) is suppressed: if it had fired on the unsound premise,
/// the call would dispatch to `$f_a` returning 1; with the fix, the
/// runtime indirection finds `$f_b` returning 2.
#[test]
#[cfg_attr(miri, ignore)]
fn leftover_segment_dynamic_offset_keeps_runtime_dispatch() -> Result<()> {
    let wat = r#"
        (module
          (import "" "g" (global $g i32))
          (type $sig (func (result i32)))
          (table 4 4 funcref)
          (func $f_a (type $sig) i32.const 1)
          (func $f_b (type $sig) i32.const 2)

          ;; precomputed[0] = $f_a → would direct-call to $f_a.
          (elem (i32.const 0) func $f_a)

          ;; Leftover writes $f_b at offset $g.
          (elem (offset (global.get $g)) func $f_b)

          (func (export "call_at_zero") (result i32)
            i32.const 0
            call_indirect (type $sig)))
    "#;

    let engine = Engine::default();
    let module = Module::new(&engine, wat)?;
    let mut store = Store::new(&engine, ());
    let g = Global::new(
        &mut store,
        GlobalType::new(ValType::I32, Mutability::Const),
        Val::I32(0),
    )?;
    let instance = Instance::new(&mut store, &module, &[g.into()])?;

    let call_at_zero = instance.get_typed_func::<(), i32>(&mut store, "call_at_zero")?;
    let result = call_at_zero.call(&mut store, ())?;
    assert_eq!(
        result, 2,
        "leftover segment must overwrite precomputed[0]; if direct-call \
         rewrite had fired we'd get 1 from $f_a"
    );
    Ok(())
}

/// Negative control: the same module shape but WITHOUT a leftover
/// segment compiles cleanly and dispatches through the (now-elided
/// or not, depends on Cranelift opts) direct path. This pins the
/// witness modules above as actually exercising the leftover-segment
/// codepath, not some unrelated trap.
#[test]
#[cfg_attr(miri, ignore)]
fn no_leftover_segment_baseline_dispatches_correctly() -> Result<()> {
    let wat = r#"
        (module
          (type $sig (func (result i32)))
          (table 4 4 funcref)
          (func $f_a (type $sig) i32.const 1)

          (elem (i32.const 0) func $f_a)

          (func (export "call_at_zero") (result i32)
            i32.const 0
            call_indirect (type $sig)))
    "#;

    let engine = Engine::default();
    let module = Module::new(&engine, wat)?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;
    let call_at_zero = instance.get_typed_func::<(), i32>(&mut store, "call_at_zero")?;
    let result = call_at_zero.call(&mut store, ())?;
    assert_eq!(result, 1);
    Ok(())
}
