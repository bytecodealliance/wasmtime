# How to Add a New Instruction to CLIF

This guide walks through adding a new opcode to the Cranelift Intermediate
Representation (CLIF). Most contributors will not need to do this — CLIF's
opcode set is intentionally stable. If you just need to teach a backend how to
handle an existing opcode, see
[How to Add a Machine Instruction and Lowering](add-machine-instruction.md)
instead.

## Where CLIF opcodes are defined

CLIF opcodes are defined using a meta-language in Rust:

- `cranelift/codegen/meta/src/shared/instructions.rs` — almost all opcodes
- `cranelift/codegen/meta/src/shared/formats.rs` — instruction formats
  (the structural shape of each instruction: how many value inputs, how many
  immediate fields, etc.)
- `cranelift/codegen/meta/src/shared/immediates.rs` — immediate field types

The meta code is compiled during the build (via `cranelift/codegen/build.rs`)
to generate Rust source files in `OUT_DIR` (`target/`). These generated files
are then `include!()`-ed by the handwritten source files in
`cranelift/codegen/src/ir/`. Generated artifacts include:

- `opcodes.rs` — opcode definitions and instruction metadata
- `inst_builder.rs` — `InstBuilder` methods for each opcode
- ISLE `extern` declarations so ISLE rules can reference CLIF opcodes

## Step 1: Choose or create an instruction format

An *instruction format* defines the structural shape of an instruction. Look
at the existing formats in `cranelift/codegen/meta/src/shared/formats.rs`. If
none fit, add a new one by adding a field to the `Formats` struct and
initializing it in `Formats::new()`:

```rust
// In the Formats struct:
pub(crate) new_format: Rc<InstructionFormat>,

// In Formats::new():
new_format: Builder::new("NewFormat")
    .value()          // one SSA value input
    .value()          // another SSA value input
    .imm(&imm.imm64)  // a 64-bit immediate
    .build(),
```

A format is shared by multiple opcodes that have the same structural shape. For
example, many opcodes use the `Binary` format (two value inputs, one result).

## Step 2: Add the opcode definition

In `cranelift/codegen/meta/src/shared/instructions.rs`, add an entry in the
`define()` function. If your instruction is SIMD-related, it may belong in one
of the three sub-functions called by `define()`: `define_control_flow`,
`define_simd_lane_access`, or `define_simd_arithmetic`. Otherwise add it
directly in the body of `define()` alongside similar opcodes:

```rust
ig.push(
    Inst::new(
        "my_new_op",
        r#"
    My new operation.

    Detailed description of semantics: what the instruction does, any
    special cases (overflow behavior, NaN handling, alignment
    requirements, etc.).
    "#,
        &formats.binary,   // use the appropriate format
    )
    .operands_in(vec![
        Operand::new("x", iN).with_doc("Left operand"),
        Operand::new("y", iN).with_doc("Right operand"),
    ])
    .operands_out(vec![
        Operand::new("a", iN).with_doc("Result"),
    ]),
);
```

Where `iN` is a `TypeVar` describing which types this opcode is polymorphic
over. Look at existing definitions for examples of how to construct `TypeVar`
instances.

If the instruction can trap, call `.can_trap()` on the builder.  
If the instruction has side effects, call `.other_side_effects()`.  
If it is a memory operation, use `.can_load()` or `.can_store()`.

## Step 3: Rebuild the generated code

Run:

```
cargo build -p cranelift-codegen
```

This re-runs the meta build script and writes into `OUT_DIR` (`target/`):
- `opcodes.rs` — `include!()`-ed by `cranelift/codegen/src/ir/instructions.rs`
- `inst_builder.rs` — `include!()`-ed by `cranelift/codegen/src/ir/builder.rs`
- ISLE `extern` declarations for opcodes and types

The source files `instructions.rs` and `builder.rs` are not themselves
regenerated; they `include!()` the generated content from `OUT_DIR`.

If anything is structurally wrong with your definition, the build will fail
with a descriptive error.

## Step 4: Add a verifier check (optional but recommended)

The IR verifier (`cranelift/codegen/src/verifier/mod.rs`) checks semantic
invariants. If your new instruction has any invariants beyond type correctness
(e.g. alignment constraints, operand restrictions), add a case in the
`immediate_constraints` function's match on `InstructionData`:

```rust
ir::InstructionData::YourFormat { opcode: ir::instructions::Opcode::MyNewOp, .. } => {
    if /* constraint is violated */ {
        errors.fatal((inst, self.context(inst), "description of the error"))
    } else {
        Ok(())
    }
}
```

Use `errors.fatal` (returns `Err(())` and stops further checks on this
instruction) or `errors.nonfatal` (returns `Ok(())` and continues) depending
on severity. Do not use `errors.report` directly here — it returns `()` and
the match arm must return `VerifierStepResult`.

## Step 5: Add interpreter support

The Cranelift interpreter (`cranelift/interpreter/`) is used by the `test run`
and `test interpret` filetest commands. Add a case for your opcode in the
`step` function in `cranelift/interpreter/src/step.rs`:

```rust
Opcode::MyNewOp => {
    let x = arg(0);
    let y = arg(1);
    let result = /* compute semantics */;
    Ok(assign(result))
}
```

The `assign` closure (defined near the top of `step`) wraps a single
`DataValue` result into `ControlFlow::Assign`. Use `assign_multiple` for
instructions that produce more than one result value.

## Step 6: Add lowering rules in each backend

For every backend that should support your new opcode, add an ISLE lowering
rule. See [How to Add a Machine Instruction and Lowering](add-machine-instruction.md)
for details.

If a backend does not implement a rule for an opcode, attempting to compile a
function using that opcode on that backend will produce a compile-time error.

## Step 7: Add optimization rules (optional)

If your new opcode can be simplified algebraically (e.g. identity elements,
constant folding), add ISLE rewrite rules in
`cranelift/codegen/src/opts/`. For example, to constant-fold
`my_new_op(c1, c2)` where both operands are constants:

```lisp
(rule (simplify
       (my_new_op (fits_in_64 ty)
                  (iconst ty k1)
                  (iconst ty k2)))
      (subsume (iconst ty (imm64_my_new_op ty k1 k2))))
```

## Step 8: Write tests

Add a filetest in `cranelift/filetests/filetests/`:

```
test verifier

function %test_my_new_op(i32, i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = my_new_op v0, v1
    return v2
}
```

A `test verifier` function with no `; error:` annotations is expected to pass
verification without errors. To assert that a specific instruction *causes* a
verifier error, annotate it inline:

```
    v2 = my_new_op v0, v1  ; error: <expected error text>
```

Add a `test run` filetest to verify semantics:

```
test run
target x86_64

function %my_new_op_test() -> i32 {
block0:
    v0 = iconst.i32 3
    v1 = iconst.i32 4
    v2 = my_new_op v0, v1
    return v2
}
; run: %my_new_op_test() == 7
```
