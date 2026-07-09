# How to Add a New Instruction to CLIF

This is a walkthrough of what's involved in adding a brand new opcode to
Cranelift's target-independent IR. It doesn't cover adding a new instruction
to a specific machine backend (the ISA-specific `MachInst`s);

Before adding a new instruction, it's worth checking whether the semantics you want can already be expressed as a combination of existing instructions. CLIF tries to keep its instruction set fairly small; new opcodes are for cases where composing existing ones would be either impossible or noticeably worse for codegen.

## 1. Declare the instruction

Instructions are declared in the meta-language, not directly in Rust. The
shared (ISA-independent) instructions are in
`cranelift/codegen/meta/src/shared/instructions.rs`. A declaration looks like
this:

```rust
ig.push(
    Inst::new(
        "iadd",
        r#"
    Wrapping integer addition: `a := x + y \pmod{2^B}`.

    This instruction does not depend on the signed/unsigned interpretation
    of the operands.
    "#,
        &formats.binary,
    )
    .operands_in(vec![Operand::new("x", Int), Operand::new("y", Int)])
    .operands_out(vec![Operand::new("a", Int)])
    .inst_builder_imm_method(true),
);
```

The third argument to `Inst::new` is an instruction format, chosen from
`cranelift/codegen/meta/src/shared/formats.rs` (`unary`, `binary`, `ternary`,
`call`, `load`, `store`, `atomic_rmw`, and so on). The format determines what
shape of operands the generated `InstructionData` variant has. That is to say,reuse an
existing format if your instruction fits one, rather than adding a new one.

If your instruction has effects beyond producing a result
say so on the `Inst` builder with methods like `.can_trap()`, `.can_load()`,
`.can_store()`, `.other_side_effects()`, `.branches()`, or `.call()` (see
`cranelift/codegen/meta/src/cdsl/instructions.rs` for the full list). 

## 2. Let the build regenerate the Rust and ISLE glue

`cranelift/codegen/build.rs` runs the meta crate as part of the normal
`cargo build`. From your instruction declaration it generates (into
`OUT_DIR`, under `target/`):

- A variant of the `Opcode` enum and of `InstructionData`.
- Support code for the format you used (encoding, decoding, `Display`).
- A method on the `InstBuilder` trait, so frontends can write
  `builder.ins().your_new_instr(...)`.
- `extern` declarations for the instruction in the auto-generated
  `clif_lower.isle` and `clif_opt.isle` files, so it's immediately usable
  as an ISLE pattern from lowering and mid-end rules. (See
  [How ISLE is Integrated with Cranelift](isle-integration.md) for more on
  those generated files.)

You don't write any of this by hand, by running `cargo check` (or `build`)
the compiler will tell you where you still need to put the new opcode in.

## 3. Fill in the remaining match arms

Because `Opcode` gained a new variant, several exhaustive `match`es elsewhere
in the codebase will stop compiling until you handle it. In practice this
means the compiler walks you through most of the checklist, but the usual
places are:

- **The interpreter**, `cranelift/interpreter/src/step.rs`, which matches
  on `inst.opcode()` to give every instruction an execution semantics.
  Without this, `test interpret` and `test run` won't be able to execute the
  instruction.
- **The verifier**, `cranelift/codegen/src/verifier/mod.rs`, if the new instruction introduces type rules or operand constraints that aren't already enforced by its format or type variables.
- **Alias analysis and the egraph mid-end**
  (`cranelift/codegen/src/alias_analysis.rs`,
  `cranelift/codegen/src/egraph/`) if the instruction touches memory or
  needs special handling to be optimized correctly. In practice, this is often an extension of the side-effect annotations defined in the first step.

Pure instructions (no side effects, standard type variables) often
need nothing beyond steps 1 and 2 plus a lowering rule in each backend that
should support them.

## 4. Add a lowering rule per backend

A CLIF instruction with no backend that can lower it will hit ISLE's "no rule
matched" panic the moment it's used. At a minimum, you should add a (rule (lower (your_instr ...)) ...) definition to each cranelift/codegen/src/isa/<arch>/lower.isle backend you intend to support. 

## 5. Test it

Add file tests under `cranelift/filetests/filetests/`. Use `test interpret` and
`test run` to verify the execution semantics implemented in step 3, `test optimize`
to validate any mid-end rewrites, and `test compile` (placed under
`filetests/isa/<arch>/` for each target architecture)to test the backend lowering implemented in step 4. See
[Testing Cranelift](testing.md) for how these test commands work.

## Worked example

[#12101](https://github.com/bytecodealliance/wasmtime/pull/12101), which
added the `patchable_call` instruction, is a reasonably self-contained recent
example that touches most of the above: the instruction declaration in
`instructions.rs`, and lowering rules in both the x64 and aarch64
`lower.isle` files.
