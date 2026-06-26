# How to Add a New Cranelift Backend

This guide walks through the steps to add a new ISA backend to Cranelift.
Before starting, read [Backend Architecture](backend-architecture.md) for an
overview of the components you will be implementing.

The existing backends (`aarch64`, `riscv64`, `s390x`, `x64`, `pulley`) are
good references throughout this process.

## Step 1: Create the backend directory

Create `cranelift/codegen/src/isa/<arch>/` with the following modules:

```
cranelift/codegen/src/isa/<arch>/
    mod.rs
    abi.rs
    settings.rs     (usually generated — see Step 3)
    inst/
        mod.rs
        args.rs
        emit.rs
        regs.rs
    lower/
        isle.rs
        *.isle
```

## Step 2: Register the backend

In `cranelift/codegen/src/isa/mod.rs`:

1. Add a feature flag for the new backend (e.g. `#[cfg(feature = "newarch")]`)
   and `pub mod newarch;`.
2. In `isa::lookup()`, add a match arm that maps your triple's architecture to
   a new `Builder` for your backend.

In `Cargo.toml` for `cranelift-codegen`, add a feature like:

```toml
[features]
newarch = []
```

And add it to the default features if appropriate.

## Step 3: Define ISA-specific settings

Create `cranelift/codegen/meta/src/isa/<arch>/mod.rs` (or `settings.rs`)
defining ISA-specific settings using the meta DSL:

```rust
pub fn define() -> SettingGroup {
    let mut setting = SettingGroupBuilder::new("newarch");
    // add flags/enums as needed
    setting.build()
}
```

Run `cargo build -p cranelift-codegen` once to generate
`cranelift/codegen/src/isa/<arch>/settings.rs` from the meta definition.
This generated file defines `Flags` and `Builder` types for your ISA settings.

Register your settings in `cranelift/codegen/meta/src/isa/mod.rs` and in
`cranelift/codegen/meta/src/lib.rs`.

## Step 4: Define machine registers

In `inst/regs.rs`, define:

- Constants for physical registers (e.g. `pub const X0: PReg = PReg::new(0, RegClass::Int);`)
- A `create_machine_env()` function that returns a `regalloc2::MachineEnv`
  listing allocatable registers, callee-saved registers, etc.

Example:

```rust
use regalloc2::{MachineEnv, PReg, PRegSet, RegClass};

pub fn x_reg(enc: usize) -> Reg { ... }
pub fn f_reg(enc: usize) -> Reg { ... }

pub fn create_machine_env() -> MachineEnv {
    MachineEnv {
        preferred_regs_by_class: [
            // integer registers
            vec![x_reg(0), x_reg(1), /* ... */],
            // float registers
            vec![f_reg(0), f_reg(1), /* ... */],
            // vector registers (if any)
            vec![],
        ],
        non_preferred_regs_by_class: [vec![], vec![], vec![]],
        fixed_stack_slots: vec![],
        scratch_by_class: [None, None, None],
    }
}
```

## Step 5: Define machine instructions

In `inst/mod.rs`, define an enum `Inst` for all machine instructions. Each
variant holds the operands and addressing modes for that instruction.

`Inst` must implement the `MachInst` trait:

```rust
impl MachInst for Inst {
    type ABIMachineSpec = NewArchABIMachineSpec;
    type LabelUse = LabelUse;

    fn get_operands(&self, collector: &mut impl OperandVisitor) { ... }
    fn is_term(&self) -> MachTerminator { ... }
    fn branch_destination<'a>(&'a self, targets: &'a [MachLabel]) -> ... { ... }
    fn is_included_in_clobbers(&self) -> bool { ... }
    fn is_move(&self) -> Option<(Writable<Reg>, Reg)> { ... }
    fn gen_move(to: Writable<Reg>, from: Reg, ty: Type) -> Inst { ... }
    fn gen_nop(preferred_size: usize) -> Inst { ... }
    fn rc_for_type(ty: Type) -> CodegenResult<(&'static [RegClass], &'static [Type])> { ... }
    fn canonical_type_for_rc(rc: RegClass) -> Type { ... }
    fn gen_jump(target: MachLabel) -> Inst { ... }
    fn worst_case_size() -> CodeOffset { ... }
    fn ref_type_regclass(_: &settings::Flags) -> RegClass { ... }
}
```

## Step 6: Implement binary emission

In `inst/emit.rs`, implement `MachInstEmit` for `Inst`:

```rust
impl MachInstEmit for Inst {
    type State = EmitState;
    type Info = EmitInfo;

    fn emit(&self, allocs: &[regalloc2::Allocation], sink: &mut MachBuffer<Inst>, info: &EmitInfo, state: &mut EmitState) {
        match self {
            Inst::Add { rd, rn, rm } => {
                let rd = allocs[0].as_reg().unwrap();
                let rn = allocs[1].as_reg().unwrap();
                let rm = allocs[2].as_reg().unwrap();
                // encode and write bytes to sink
                sink.put4(encode_add(rd, rn, rm));
            }
            // ...
        }
    }
}
```

`EmitState` tracks per-function state during emission (stack frame size, etc.).
`EmitInfo` carries settings that are constant for the entire function.

## Step 7: Implement the ABI

In `abi.rs`, implement `ABIMachineSpec`:

```rust
pub struct NewArchABIMachineSpec;

impl ABIMachineSpec for NewArchABIMachineSpec {
    type I = Inst;
    type F = settings::Flags;

    fn word_bits() -> u32 { 64 }
    fn stack_align(_call_conv: isa::CallConv) -> u32 { 16 }
    fn compute_arg_locs(...) -> ... { ... }
    fn gen_load_stack(mem: StackAMode, into_reg: Writable<Reg>, ty: Type, flags: MemFlags) -> Inst { ... }
    fn gen_store_stack(mem: StackAMode, from_reg: Reg, ty: Type, flags: MemFlags) -> Inst { ... }
    fn gen_move(to: Writable<Reg>, from: Reg, ty: Type) -> Inst { ... }
    fn gen_extend(to: Writable<Reg>, from: Reg, signed: bool, from_bits: u8, to_bits: u8) -> Inst { ... }
    fn gen_ret(setup_frame: bool, isa_flags: &Self::F, rets: &[Reg]) -> Inst { ... }
    fn gen_add_imm(rd: Writable<Reg>, rn: Reg, imm: u32) -> SmallVec<[Inst; 4]> { ... }
    // ... many more
}
```

The full list of required methods is in `cranelift/codegen/src/machinst/abi.rs`.
The existing backends are the best reference for what each method should do.

## Step 8: Write ISLE lowering rules

Create `lower/<arch>.isle` (you can name it as you like) with lowering rules.
At minimum you need to provide a rule for every CLIF opcode your backend
supports.

A rule looks like:

```lisp
;; Lower iadd
(rule (lower (has_type (fits_in_64 ty) (iadd x y)))
      (alu_rrr (ALUOp.Add) ty x y))
```

You also need to define your machine instruction constructors as ISLE
*constructors* that emit instructions, e.g.:

```lisp
(decl alu_rrr (ALUOp Type Value Value) InstOutput)
(extern constructor alu_rrr alu_rrr)
```

where `alu_rrr` is implemented as a Rust function in `lower/isle.rs`:

```rust
pub fn alu_rrr(&mut self, op: ALUOp, ty: Type, rn: Value, rm: Value) -> InstOutput {
    let rn = self.put_in_reg(rn);
    let rm = self.put_in_reg(rm);
    let rd = self.alloc_tmp(ty);
    self.emit(Inst::AluRRR { op, rd, rn, rm });
    output_reg(rd.to_reg(), ty)
}
```

Refer to [How to Add a Machine Instruction and Lowering](add-machine-instruction.md)
for a detailed walkthrough.

## Step 9: Wire up the backend

In `mod.rs`, implement `TargetIsa` for your backend struct:

```rust
pub struct NewArchBackend {
    triple: Triple,
    flags: settings::Flags,
    isa_flags: newarch_settings::Flags,
}

impl TargetIsa for NewArchBackend {
    fn name(&self) -> &'static str { "newarch" }
    fn triple(&self) -> &Triple { &self.triple }
    fn flags(&self) -> &settings::Flags { &self.flags }
    fn isa_flags(&self) -> Vec<settings::Value> { self.isa_flags.iter().collect() }

    fn compile_function(&self, func: &Function, domtree: &DominatorTree, ...) -> CodegenResult<CompiledCodeStencil> {
        let (vcode, regalloc_result) = self.compile_vcode(func, domtree, ctrl_plane)?;
        let emit_result = vcode.emit(&regalloc_result, ...)?;
        Ok(emit_result.into())
    }
    // ...
}
```

## Step 10: Add tests

1. Add filetest `.clif` files in `cranelift/filetests/filetests/` under a
   subdirectory for your ISA.
2. Use `test compile` to verify end-to-end compilation, and `test run` if your
   ISA can be run via an interpreter (or natively on CI).
3. Add `test verifier` tests to verify that your backend correctly rejects
   invalid IR.

Run with:

```
cargo test -p cranelift-filetests
```

or for a specific file:

```
cargo run -p cranelift-codegen --example clif-util -- test path/to/test.clif
```
