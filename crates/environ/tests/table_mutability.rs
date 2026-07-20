//! Integration tests for the table-mutability analysis behind
//! `Module::table_is_immutable`.
//!
//! The per-table mutability bit is the foundation of the `call_indirect`
//! optimizations in `crates/cranelift/src/func_environ.rs` (bound-load
//! elision on never-grown tables, and the follow-up sig-check / null-check
//! elisions). A false negative here — failing to mark a table as mutated
//! when it actually is — would silently skip required runtime checks. A
//! false positive — marking an immutable table as mutated — is merely a
//! missed optimization. Pin the analysis behaviour with focused
//! module-level tests so any regression surfaces immediately, not after a
//! downstream optimization fires on a now-invalid premise.
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
//!   `Object.defineProperty`. Analogous risk: `table.init` (passive-
//!   segment write at runtime) being treated as a no-op rather than a
//!   write. See `table_init_marks_destination`.
//!
//! Lives in `tests/` rather than as a `#[cfg(test)] mod` inside
//! `module_environ.rs` so it builds against the lib as a normal
//! dependency.

use wasmparser::{Parser, Validator, WasmFeatures};
use wasmtime_environ::{
    ModuleEnvironment, ModuleTypesBuilder, StaticModuleIndex, TableIndex, Tunables,
};

/// Translate `wat` and return the per-table "may be mutated" bits, in
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
        .map(|i| {
            !translation
                .module
                .table_is_immutable(TableIndex::from_u32(i))
        })
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

/// Translating without any tables at all must not panic, and must produce
/// an empty result rather than e.g. a default-allocated single entry.
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

/// The scan runs before function bodies are validated, so a body can name
/// a table index that doesn't exist. The analysis must neither panic nor
/// size any per-table structure from that unvalidated index; the module
/// is rejected later when the body is validated. (Only the code section
/// is malformed here — translation itself succeeds because body
/// validation is deferred to compilation.)
#[test]
fn out_of_range_table_index_in_unvalidated_body_is_ignored() {
    let bits = translate_and_get_mutability(
        r#"
        (module
          (table 4 funcref)
          (func $f (result i32) i32.const 0)
          (func (export "bogus")
            i32.const 0
            ref.func $f
            table.set 99))
        "#,
    );
    assert_eq!(
        bits,
        vec![false],
        "an out-of-range destination index marks nothing",
    );
}

/// When every table is pre-marked (here: one exported, one imported), the
/// result is all-mutated regardless of what the code section contains —
/// this is also the shape where the analysis skips the body walk
/// entirely.
#[test]
fn all_tables_pre_marked_without_code_mutation() {
    let bits = translate_and_get_mutability(
        r#"
        (module
          (import "host" "t" (table 4 funcref))
          (table (export "u") 4 funcref)
          (func $f (result i32) i32.const 0)
          (func (export "innocent") (result i32)
            i32.const 0
            call_indirect (param) (result i32)))
        "#,
    );
    assert_eq!(bits, vec![true, true]);
}

/// Translate + `finalize_table_init`, then return, per table, whether a
/// static funcref image is available (`Module::static_funcref_image`).
fn translate_and_get_static_images(wat: &str) -> Vec<bool> {
    let bytes = wat::parse_str(wat).expect("WAT parse failed");
    let tunables = Tunables::default_host();
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
    let mut translation = env.translate(parser, &bytes).expect("translate failed");
    translation.finalize_table_init(&tunables, &mut types);
    let n: u32 = translation.module.tables.len().try_into().unwrap();
    (0..n)
        .map(|i| {
            translation
                .module
                .static_funcref_image(TableIndex::from_u32(i))
                .is_some()
        })
        .collect()
}

/// A single constant-offset function-list segment folds completely, so
/// the static image is available.
#[test]
fn fully_folded_segments_provide_static_image() {
    let images = translate_and_get_static_images(
        r#"
        (module
          (table 4 4 funcref)
          (func $f (result i32) i32.const 0)
          (elem (i32.const 0) $f $f))
        "#,
    );
    assert_eq!(images, vec![true]);
}

/// An expressions-form segment cannot be folded; it is deferred to
/// instantiation, so the table's runtime contents exceed its
/// compile-time image and no static image may be exposed. Without this
/// guard, an optimization consuming the image would miss the deferred
/// write — the segment here installs a function whose signature differs
/// from the folded prefix.
#[test]
fn deferred_segments_disable_static_image() {
    let images = translate_and_get_static_images(
        r#"
        (module
          (table 3 3 funcref)
          (func $a (result i32) i32.const 1)
          (func $b (result i64) i64.const 2)
          (elem (i32.const 0) $a $a)
          (elem (i32.const 2) funcref (ref.func $b)))
        "#,
    );
    assert_eq!(
        images,
        vec![false],
        "a deferred segment must disqualify the static image",
    );
}
