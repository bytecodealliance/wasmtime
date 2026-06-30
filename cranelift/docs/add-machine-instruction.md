# How to Add a Machine Instruction and Lowering

This guide covers two related tasks:

1. Adding a new machine instruction variant to a backend's `Inst` enum.
2. Adding a CLIF-to-machine-instruction lowering rule in ISLE.

These often go together: a new lowering may need a new instruction form, and a
new instruction form is only useful if it can be selected by the lowering.

For background on how the backend and ISLE work, read
[Backend Architecture](backend-architecture.md).

## Part 1: Adding a new machine instruction

### Step 1: Add the instruction variant

Open `cranelift/codegen/src/isa/<arch>/inst/mod.rs` and add a variant to
the `Inst` enum:

```rust
/// A fused multiply-add: rd = rn * rm + ra
FMAdd {
    rd: Writable<Reg>,
    rn: Reg,
    rm: Reg,
    ra: Reg,
    ty: Type,
},
```

Add it to the `Display` / `Debug` implementation too, so that `VCode` can be
pretty-printed for debugging.

### Step 2: Implement `get_operands`

In the `MachInst::get_operands` implementation, add a case for the new
variant. This tells the register allocator about all register uses and
definitions:

```rust
Inst::FMAdd { rd, rn, rm, ra, .. } => {
    collector.reg_def(rd);
    collector.reg_use(rn);
    collector.reg_use(rm);
    collector.reg_use(ra);
}
```

Use `reg_def` for registers written, `reg_use` for registers read, and
`reg_mod` for registers both read and written (e.g., x86 two-operand instructions).

### Step 3: Implement `worst_case_size`

If the new instruction might be larger than the current `worst_case_size()`,
update it. This is used by `MachBuffer` for constant island spacing.

### Step 4: Implement binary emission

In `cranelift/codegen/src/isa/<arch>/inst/emit.rs`, add an arm in the
`emit` match:

```rust
Inst::FMAdd { rd, rn, rm, ra, ty } => {
    // By the time emit() is called, regalloc has already resolved all
    // virtual registers to physical registers in the instruction struct.
    let rd = rd.to_reg().to_real_reg().unwrap().hw_enc();
    let rn = rn.to_real_reg().unwrap().hw_enc();
    let rm = rm.to_real_reg().unwrap().hw_enc();
    let ra = ra.to_real_reg().unwrap().hw_enc();

    // Emit the encoding. AArch64 FMADD (scalar):
    // 0001_1111 | ftype | 0 | rm | ra | rn | rd
    let ftype = match ty { F32 => 0b00, F64 => 0b01, _ => panic!() };
    let enc = (0b0001_1111 << 24)
        | (ftype << 22)
        | ((rm as u32) << 16)
        | ((ra as u32) << 10)
        | ((rn as u32) << 5)
        | (rd as u32);
    sink.put4(enc);
}
```

### Step 5: Add ISLE declarations

The new machine instruction needs to be declared in ISLE so that lowering rules
can produce it. In the backend's ISLE instruction definition file
(`cranelift/codegen/src/isa/<arch>/inst.isle`), add:

```lisp
;; Declare the constructor that the lowering rules will call.
(decl fmadd (Type Value Value Value) InstOutput)
(extern constructor fmadd fmadd)
```

Then implement the Rust constructor in `lower/isle.rs`:

```rust
pub fn fmadd(&mut self, ty: Type, rn: Value, rm: Value, ra: Value) -> InstOutput {
    let rn = self.put_in_reg(rn);
    let rm = self.put_in_reg(rm);
    let ra = self.put_in_reg(ra);
    let rd = self.alloc_tmp(ty);
    self.emit(Inst::FMAdd { rd, rn, rm, ra, ty });
    output_reg(rd.to_reg(), ty)
}
```

## Part 2: Adding a CLIF lowering rule

### Step 1: Write the rule in ISLE

Open the backend's lowering ISLE file
(`cranelift/codegen/src/isa/<arch>/lower.isle`) and add a rule:

```lisp
;; Lower fma (fused multiply-add) to FMAdd on our ISA.
(rule (lower (has_type (ty_scalar_float ty)
                       (fma x y z)))
      (fmadd ty x y z))
```

Breaking down the rule:
- `(lower ...)` — top-level lowering term; the argument is a CLIF instruction
- `(has_type ty ...)` — constrains the instruction to have type `ty`
- `(ty_scalar_float ty)` — extractor that succeeds only for `f32`/`f64`
- `(fma x y z)` — matches the `fma` CLIF opcode with three value operands
- `(fmadd ty x y z)` — calls the `fmadd` constructor we defined above

### Step 2: Use ISLE pattern extractors

ISLE provides many built-in extractors for common patterns. Some useful ones:

- `(iconst k)` — matches an `iconst` instruction and binds `k` to the constant value
- `(fits_in_64 ty)` — succeeds if `ty` has ≤ 64 bits
- `(ty_int ty)` — succeeds if `ty` is an integer type
- `(ty_scalar_float ty)` — succeeds if `ty` is `f32` or `f64`
- `(value_type ty x)` — binds `ty` to the type of SSA value `x`
- `(put_in_reg x)` — materializes an SSA value into a register (used in constructors)
- `(put_in_regs x)` — same, for multi-register values (e.g. `i128`)

### Step 3: Handle type polymorphism

If the same lowering applies to multiple types, use a type variable:

```lisp
(rule (lower (has_type (fits_in_64 ty) (iadd x y)))
      (alu_rrr (ALUOp.Add) ty x y))
```

If you need different instructions for different types, add multiple rules with
different type constraints. ISLE picks the most specific matching rule.

### Step 4: Sink producer instructions

Sometimes it is beneficial to fold a producer into its consumer (instruction
selection). For example, an `iadd` followed by an `icmp` might be expressible
as a single compare-and-add. ISLE lets you match through the SSA def:

```lisp
;; cmp r, r  combined with  add r, r  ->  cmpadd r, r, r
(rule (lower (has_type (fits_in_64 ty)
                       (icmp cc (iadd x y) z)))
      (cmp_and_add ty cc x y z))
```

When a pattern matches a producer instruction like `(iadd x y)`, ISLE's
lowering framework automatically calls `sink_inst` to mark that instruction as
absorbed.

### Step 5: Build and test

Rebuild to regenerate ISLE output:

```
cargo build -p cranelift-codegen
```

If the ISLE compiler catches a type mismatch or missing rule, it will print an
error. For a detailed multi-line error, rebuild with:

```
cargo build -p cranelift-codegen --features isle-errors
```

To inspect the generated Rust code:

```
ISLE_SOURCE_DIR=$(pwd)/isle-generated cargo check -p cranelift-codegen
```

Then add a filetest:

```
test compile
target aarch64

function %test_fma(f32, f32, f32) -> f32 {
block0(v0: f32, v1: f32, v2: f32):
    v3 = fma v0, v1, v2
    return v3
}
; check: fmadd
```

The `; check: fmadd` filecheck directive asserts that the compiled output
contains an `fmadd` instruction.

Run the test:

```
cargo run -p cranelift-tools -- test path/to/test.clif
```
