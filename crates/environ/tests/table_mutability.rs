//! Integration tests for `analyze_table_mutability` and the surrounding
//! precompute ordering invariants.
//!
//! The per-table mutability bit is the foundation of the `call_indirect`
//! optimizations in `crates/cranelift/src/func_environ.rs`
//! (constant-index direct call, sig-check elision, NULL elision, bound-
//! load elision). A false negative here — failing to mark a table as
//! mutated when it actually is — would silently turn correct calls into
//! incorrect direct calls or skip required runtime checks. A false
//! positive — marking an immutable table as mutated — is merely a missed
//! optimization. Pin the analysis behaviour with focused module-level
//! tests so any regression surfaces immediately, not after a downstream
//! optimization fires on a now-invalid premise.
//!
//! Test scenario inspiration drawn from comparable bugs in peer
//! interpreters that have shipped fixes for analogous IC-invalidation
//! mistakes:
//!
//! - **Luau** (`LOP_NAMECALL`): inline cache had to be invalidated on
//!   `table.insert` / metatable change. Analogous wasm risk: `table.grow`
//!   not invalidating an immutability proof, so see `table_grow_marks…`.
//! - **JavaScriptCore** (`ic_table`): inline-cache corruption from missed
//!   shape transitions. Analogous risk: over-marking, e.g. `table.copy`
//!   wrongly marking the SOURCE table as mutated would forbid downstream
//!   optimizations on a perfectly read-only table. See
//!   `table_copy_marks_destination_only_not_source`.
//! - **Hermes** (`HiddenClass` cache): property cache misses with
//!   `Object.defineProperty`. Analogous risk: `table.init` (active-
//!   segment init at runtime) being treated as a no-op rather than a
//!   write. See `table_init_marks_destination`.
//!
//! Lives in `tests/` rather than as a `#[cfg(test)] mod` inside
//! `module_environ.rs` because the latter triggers a pre-existing
//! upstream compile failure in `key.rs` / `module_artifacts.rs` (their
//! `arbitrary::Arbitrary` derives are stale relative to the workspace's
//! pinned `arbitrary 1.4.2`). Integration tests build against the lib
//! as a normal dependency and so do not set `cfg(test)` on
//! `wasmtime-environ` itself.

use wasmparser::{Parser, Validator, WasmFeatures};
use wasmtime_environ::{
    ModuleEnvironment, ModuleTypesBuilder, StaticModuleIndex, TableIndex, Tunables,
};

/// Translate `wat` and return the resulting `tables_mutated` bits, in
/// table-index order. Helper to keep individual tests short.
fn translate_and_get_mutability(wat: &str) -> Vec<bool> {
    let bytes = wat::parse_str(wat).expect("WAT parse failed");
    let tunables = Tunables::default_host();
    // WASM2 covers reference-types + bulk-memory, which is what every
    // table-mutating opcode below needs (`table.set`, `table.fill`,
    // `table.grow`, `table.copy`, `table.init`, `elem.drop`).
    let features = WasmFeatures::WASM2;
    let mut validator = Validator::new_with_features(features);
    let mut types = ModuleTypesBuilder::new(&validator);
    let env = ModuleEnvironment::new(
        &tunables,
        &mut validator,
        &mut types,
        StaticModuleIndex::from_u32(0),
    );
    let parser = Parser::new(0);
    let translation = env.translate(parser, &bytes).expect("translate failed");
    let n: u32 = translation.module.tables.len().try_into().unwrap();
    (0..n)
        .map(|i| translation.module.tables_mutated[TableIndex::from_u32(i)])
        .collect()
}

/// A table only used as the source of `call_indirect` and `table.get` is
/// provably immutable. (Both ops READ the table; neither writes it.) The
/// table is intentionally NOT exported — exported tables are
/// conservatively pre-marked as mutated (see
/// `exported_tables_are_pre_marked` for the export case) since the host
/// can mutate them via the public wasmtime API.
#[test]
fn read_only_table_is_immutable() {
    let bits = translate_and_get_mutability(
        r#"
        (module
          (table 4 funcref)
          (func $f (result i32) i32.const 42)
          (elem (i32.const 0) $f $f $f $f)
          (func (export "call_zero") (result i32)
            i32.const 0
            call_indirect (param) (result i32))
          (func (export "read_zero") (result funcref)
            i32.const 0
            table.get 0))
        "#,
    );
    assert_eq!(bits, vec![false], "no opcode mutated this table");
}

/// Exported tables are always pre-marked as mutated, regardless of
/// whether any opcode in this module touches them. The host can call
/// `Table::set` / `Table::grow` via the public wasmtime API on any
/// exported table, and another module that imports the export can also
/// mutate it. Without this rule, downstream optimizations would
/// happily elide null traps and sig checks on exported tables on the
/// (false) assumption that the table contents are stable.
#[test]
fn exported_tables_are_pre_marked() {
    let bits = translate_and_get_mutability(
        r#"
        (module
          (table (export "t") 4 funcref)
          (func $f (result i32) i32.const 42)
          (elem (i32.const 0) $f $f $f $f))
        "#,
    );
    assert_eq!(bits, vec![true]);
}

/// `table.set` marks its destination as mutated.
#[test]
fn table_set_marks_destination() {
    let bits = translate_and_get_mutability(
        r#"
        (module
          (table 4 funcref)
          (func $f (result i32) i32.const 0)
          (func (export "do_set")
            i32.const 1
            ref.func $f
            table.set 0))
        "#,
    );
    assert_eq!(bits, vec![true]);
}

/// `table.fill` marks its destination as mutated.
#[test]
fn table_fill_marks_destination() {
    let bits = translate_and_get_mutability(
        r#"
        (module
          (table 4 funcref)
          (func $f (result i32) i32.const 0)
          (func (export "do_fill")
            i32.const 0
            ref.func $f
            i32.const 4
            table.fill 0))
        "#,
    );
    assert_eq!(bits, vec![true]);
}

/// `table.grow` is treated as mutating — analogous to Luau's NAMECALL IC
/// needing to invalidate on table-shape change.
#[test]
fn table_grow_marks_destination() {
    let bits = translate_and_get_mutability(
        r#"
        (module
          (table 4 funcref)
          (func (export "do_grow") (result i32)
            ref.null func
            i32.const 1
            table.grow 0))
        "#,
    );
    assert_eq!(bits, vec![true]);
}

/// `table.copy` marks the DESTINATION but explicitly NOT the source. The
/// source is read-only (its contents aren't changed by the op); marking
/// it as mutated would forbid downstream optimizations from treating it
/// as immutable, which would be incorrect over-conservatism — the JSC
/// `ic_table` analogue.
#[test]
fn table_copy_marks_destination_only_not_source() {
    let bits = translate_and_get_mutability(
        r#"
        (module
          (table $dst (export "dst") 4 funcref)
          (table $src 4 funcref)
          (func $f (result i32) i32.const 0)
          (elem (table $src) (i32.const 0) func $f $f $f $f)
          (func (export "do_copy")
            i32.const 0   ;; dst offset
            i32.const 0   ;; src offset
            i32.const 4   ;; len
            table.copy $dst $src))
        "#,
    );
    assert_eq!(
        bits,
        vec![true, false],
        "dst should be mutated, src should remain immutable",
    );
}

/// `table.init` writes to the destination table from a passive elem
/// segment, so it is treated as mutation (the destination's contents
/// change at runtime).
#[test]
fn table_init_marks_destination() {
    let bits = translate_and_get_mutability(
        r#"
        (module
          (table 4 funcref)
          (func $f (result i32) i32.const 0)
          (elem $e funcref (ref.func $f) (ref.func $f))
          (func (export "do_init")
            i32.const 0   ;; dst
            i32.const 0   ;; src offset within elem
            i32.const 2   ;; len
            table.init 0 $e))
        "#,
    );
    assert_eq!(bits, vec![true]);
}

/// `elem.drop` drops a passive element segment but does NOT write to any
/// table — distinct from `table.init` which DOES write. A pessimistic
/// implementation that marked all tables as mutated on `elem.drop` would
/// hand out false positives and shut off optimizations on perfectly-
/// immutable tables.
#[test]
fn elem_drop_does_not_mark_tables() {
    let bits = translate_and_get_mutability(
        r#"
        (module
          (table 4 funcref)
          (func $f (result i32) i32.const 0)
          (elem $e funcref (ref.func $f))
          (func (export "do_drop")
            elem.drop $e))
        "#,
    );
    assert_eq!(bits, vec![false]);
}

/// Imported tables are always pre-marked as mutated, regardless of
/// whether any opcode in this module touches them. The importer can
/// mutate the table in ways this module can't see.
#[test]
fn imported_tables_are_pre_marked() {
    let bits = translate_and_get_mutability(
        r#"
        (module
          (import "host" "t" (table 4 funcref)))
        "#,
    );
    assert_eq!(bits, vec![true]);
}

/// A mutation in ONE function correctly marks the table — the analysis
/// has to walk every function body, not just the first.
#[test]
fn mutation_in_any_function_counts() {
    let bits = translate_and_get_mutability(
        r#"
        (module
          (table 4 funcref)
          (func $f (result i32) i32.const 0)
          (func (export "innocent") (result i32)
            i32.const 0
            call_indirect (param) (result i32))
          (func (export "guilty")
            i32.const 0
            ref.func $f
            table.set 0))
        "#,
    );
    assert_eq!(bits, vec![true]);
}

/// Two tables, one mutated, one not. The analysis tracks per-table — a
/// mutation on one must not leak to the other.
#[test]
fn mutation_isolated_to_target_table() {
    let bits = translate_and_get_mutability(
        r#"
        (module
          (table $a 4 funcref)
          (table $b 4 funcref)
          (func $f (result i32) i32.const 0)
          (func (export "mut_a")
            i32.const 0
            ref.func $f
            table.set $a))
        "#,
    );
    assert_eq!(
        bits,
        vec![true, false],
        "$a should be mutated, $b should remain immutable",
    );
}

/// Translating without any tables at all must not panic. (Defensive: the
/// analysis indexes a `SecondaryMap` keyed by `TableIndex`, and we want
/// to confirm an empty module produces an empty result rather than e.g.
/// a default-allocated single entry.)
#[test]
fn module_with_no_tables_produces_empty_mutability_vec() {
    let bits = translate_and_get_mutability(
        r#"
        (module
          (func (export "noop")))
        "#,
    );
    assert!(bits.is_empty(), "no tables ⇒ no mutability bits");
}

// -----------------------------------------------------------------------------
// Leftover active elem segments — soundness gate for the call_indirect
// elisions in `crates/cranelift/src/func_environ.rs`.
//
// `try_func_table_init` folds *some* active `elem` segments into
// `table_initialization.initial_values[t].precomputed` at compile time
// and drains them from `table_initialization.segments`. Any segment it
// can't fold (dynamic offset, `Expressions` form, out-of-range, ...)
// stays in the `segments` list and runs at instantiation time *after*
// the precomputed image is applied — potentially overwriting slots
// that downstream optimizations have read from `precomputed` and
// assumed stable. Pin the analyzer to mark every such target as
// mutated so the elisions correctly bail out. Caught by review on
// PR #2 (`https://github.com/rebeckerspecialties/wasmtime/pull/2#
// discussion_r3193374159` and `…#discussion_r3193374164`).
// -----------------------------------------------------------------------------

/// A second elem segment whose offset is `(global.get $g)` (an imported
/// global, resolved at instantiation time) cannot be folded into
/// `precomputed` — `try_func_table_init` only folds segments with a
/// constant `i32.const`/`i64.const` offset. So the segment stays in
/// `table_initialization.segments` and overwrites slots at instance
/// time. Without the leftover-segment pass in `analyze_table_mutability`,
/// the table would be marked immutable and the type-confusion soundness
/// bug fires (a function of a different signature could end up at slot
/// 0 of an "immutable" table, defeating
/// `try_elide_sig_check_for_immutable_table`).
#[test]
fn dynamic_offset_leftover_segment_marks_table_mutated() {
    let bits = translate_and_get_mutability(
        r#"
        (module
          (import "" "g" (global $g i32))
          (table 4 4 funcref)
          (func $f (result i32) i32.const 42)
          (elem (i32.const 0) func $f)
          (elem (offset (global.get $g)) func $f))
        "#,
    );
    assert_eq!(
        bits,
        vec![true],
        "leftover segment with dynamic offset must mark its target table mutated"
    );
}

/// A segment in `Expressions` form (`funcref (item ref.func ...)`)
/// rather than `Functions` form is also rejected by
/// `try_func_table_init` and stays in `segments`. Same soundness
/// argument as the dynamic-offset case: the segment's evaluation
/// happens at instantiation time and can produce arbitrary funcrefs
/// (including null via `ref.null func`), which would invalidate any
/// elision proof that read from `precomputed`.
#[test]
fn expressions_form_leftover_segment_marks_table_mutated() {
    let bits = translate_and_get_mutability(
        r#"
        (module
          (table 4 4 funcref)
          (func $f (result i32) i32.const 42)
          (elem (i32.const 0) funcref (item ref.func $f) (item ref.null func)))
        "#,
    );
    assert_eq!(
        bits,
        vec![true],
        "Expressions-form leftover segment must mark its target table mutated"
    );
}

/// `try_func_table_init` short-circuits the *whole* segment-folding
/// loop on the first segment it can't fold — including any later
/// segments that target a different table. This preserves wasm's
/// trap-ordering semantics (the failing segment might trap, in which
/// case later segments shouldn't have been applied either). The
/// upshot: a single dynamic-offset segment can leave many leftover
/// segments behind. Verify the analyzer marks every targeted table,
/// not just the one whose segment broke the loop.
#[test]
fn leftover_segments_after_short_circuit_mark_all_targets() {
    let bits = translate_and_get_mutability(
        r#"
        (module
          (import "" "g" (global $g i32))
          (table $t0 4 4 funcref)
          (table $t1 4 4 funcref)
          (func $f (result i32) i32.const 42)
          ;; First segment for t0 has dynamic offset → breaks the
          ;; folding loop → both this segment AND the later t1
          ;; segment stay in `table_initialization.segments`.
          (elem (table $t0) (offset (global.get $g)) func $f)
          (elem (table $t1) (i32.const 0) func $f))
        "#,
    );
    assert_eq!(
        bits,
        vec![true, true],
        "both targets of leftover segments must be marked mutated"
    );
}

/// Independence sanity check: a leftover segment for table 0 must NOT
/// mark a separate table 1 that has only foldable segments. Mirrors
/// the `mutation_isolated_to_target_table` test for runtime opcodes.
#[test]
fn leftover_segment_marks_only_its_target_table() {
    let bits = translate_and_get_mutability(
        r#"
        (module
          (import "" "g" (global $g i32))
          (table $t0 4 4 funcref)
          (table $t1 4 4 funcref)
          (func $f (result i32) i32.const 42)
          ;; Foldable segment for t1 — applied first, before any
          ;; segment for t0 is reached. (Wasm specifies segments are
          ;; processed in order, but `try_func_table_init` walks them
          ;; in order too, so applying t1 first matches.)
          (elem (table $t1) (i32.const 0) func $f $f $f $f)
          ;; Dynamic-offset (leftover) segment for t0.
          (elem (table $t0) (offset (global.get $g)) func $f))
        "#,
    );
    assert_eq!(
        bits,
        vec![true, false],
        "leftover-segment marking must not bleed into other tables"
    );
}
