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
        .map(|i| translation.tables_mutated[TableIndex::from_u32(i)])
        .collect()
}

/// A table only used as the source of `call_indirect` and `table.get` is
/// provably immutable. (Both ops READ the table; neither writes it.)
#[test]
fn read_only_table_is_immutable() {
    let bits = translate_and_get_mutability(
        r#"
        (module
          (table (export "t") 4 funcref)
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
