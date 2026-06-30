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
    lower.rs        (wires up the ISLE-based lowering)
    inst.isle       (machine instruction ISLE declarations)
    lower.isle      (CLIF-to-machine lowering rules)
    inst/
        mod.rs
        args.rs
        emit.rs
        regs.rs
    lower/
        isle.rs     (Rust constructors backing ISLE extern declarations)
```

Additional `.isle` files (e.g. `inst_neon.isle`, `lower_vector.isle`) live
at the same top-level arch directory alongside `inst.isle` and `lower.isle`.

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

Create `cranelift/codegen/meta/src/isa/<arch>.rs` (a single file alongside
the other ISA files in that directory) defining ISA-specific settings using
the meta DSL:

```rust
use crate::cdsl::isa::TargetIsa;
use crate::cdsl::settings::SettingGroupBuilder;

pub(crate) fn define() -> TargetIsa {
    let mut settings = SettingGroupBuilder::new("newarch");
    // add flags/enums as needed, e.g.:
    // settings.add_bool("has_feature_x", "Has feature X.", "", false);
    TargetIsa::new("newarch", settings.build())
}
```

The meta build will generate a `settings-<arch>.rs` file into `OUT_DIR`. You
must create `cranelift/codegen/src/isa/<arch>/settings.rs` as a stub that
`include!()`s it — exactly like the existing backends:

```rust
include!(concat!(env!("OUT_DIR"), "/settings-<arch>.rs"));
```

This file defines `Flags` and `Builder` types for your ISA settings.

Register your settings in `cranelift/codegen/meta/src/isa/mod.rs` and in
`cranelift/codegen/meta/src/lib.rs`.

## Step 4: Define machine registers

In `inst/regs.rs`, define helper functions and constants for physical registers,
for example:

```rust
use regalloc2::PReg;
use crate::machinst::{Reg, RegClass};

pub const fn x_reg(enc: usize) -> Reg { ... }
pub const fn f_reg(enc: usize) -> Reg { ... }
```

The `regalloc2::MachineEnv` (which lists allocatable, caller-saved, and
callee-saved registers) is constructed in `abi.rs` inside the
`ABIMachineSpec::get_machine_env()` implementation. A typical pattern:

```rust
use regalloc2::{MachineEnv, PRegSet};

const fn create_reg_environment() -> MachineEnv {
    MachineEnv {
        preferred_regs_by_class: [
            // integer registers (caller-saved / not callee-saved)
            PRegSet::empty()
                .with(PReg::new(0, RegClass::Int))
                .with(PReg::new(1, RegClass::Int)),
                /* ... */
            // float/vector registers
            PRegSet::empty()
                .with(PReg::new(0, RegClass::Float))
                /* ... */,
            // second vector class (unused on most ISAs)
            PRegSet::empty(),
        ],
        non_preferred_regs_by_class: [
            // callee-saved integer registers
            PRegSet::empty()
                .with(PReg::new(8, RegClass::Int))
                /* ... */,
            PRegSet::empty(),
            PRegSet::empty(),
        ],
        fixed_stack_slots: vec![],
        scratch_by_class: [None, None, None],
    }
}

impl ABIMachineSpec for NewArchABIMachineSpec {
    // ...
    fn get_machine_env(_flags: &settings::Flags, _call_conv: isa::CallConv) -> &MachineEnv {
        static MACHINE_ENV: MachineEnv = create_reg_environment();
        &MACHINE_ENV
    }
}
```

## Step 5: Define machine instructions

In `inst/mod.rs`, define an enum `Inst` for all machine instructions. Each
variant holds the operands and addressing modes for that instruction.

`Inst` must implement the `MachInst` trait. The key required methods are:

```rust
impl MachInst for Inst {
    type ABIMachineSpec = NewArchABIMachineSpec;
    type LabelUse = LabelUse;  // your label-use kind enum

    const TRAP_OPCODE: &'static [u8] = &[/* ISA-specific trap encoding */];

    fn get_operands(&mut self, collector: &mut impl OperandVisitor) { ... }
    fn is_move(&self) -> Option<(Writable<Reg>, Reg)> { ... }
    fn is_term(&self) -> MachTerminator { ... }
    fn is_trap(&self) -> bool { ... }
    fn is_args(&self) -> bool { ... }
    fn call_type(&self) -> CallType { ... }
    fn is_included_in_clobbers(&self) -> bool { ... }
    fn is_mem_access(&self) -> bool { ... }
    fn gen_move(to_reg: Writable<Reg>, from_reg: Reg, ty: Type) -> Self { ... }
    fn gen_dummy_use(reg: Reg) -> Self { ... }
    fn rc_for_type(ty: Type) -> CodegenResult<(&'static [RegClass], &'static [Type])> { ... }
    fn canonical_type_for_rc(rc: RegClass) -> Type { ... }
    fn gen_jump(target: MachLabel) -> Self { ... }
    fn gen_nop(preferred_size: usize) -> Self { ... }
    fn gen_nop_units() -> Vec<Vec<u8>> { ... }
    fn worst_case_size() -> CodeOffset { ... }
    fn worst_case_island_growth() -> CodeOffset { ... }
    fn ref_type_regclass(_flags: &Flags) -> RegClass { ... }
    fn is_safepoint(&self) -> bool { ... }
    fn function_alignment() -> FunctionAlignment { ... }
}
```

Several methods have default implementations and can be omitted — check the
trait definition in `cranelift/codegen/src/machinst/mod.rs` to see which ones.
`type LabelUse`, `const TRAP_OPCODE`, and `fn function_alignment` are required
with no default.

## Step 6: Implement binary emission

In `inst/emit.rs`, implement `MachInstEmit` for `Inst`:

```rust
impl MachInstEmit for Inst {
    type State = EmitState;
    type Info = EmitInfo;

    fn emit(&self, sink: &mut MachBuffer<Inst>, info: &EmitInfo, state: &mut EmitState) {
        match self {
            Inst::Add { rd, rn, rm } => {
                // Registers are already physical by the time emit() is called.
                let rd = rd.to_reg().to_real_reg().unwrap().hw_enc();
                let rn = rn.to_real_reg().unwrap().hw_enc();
                let rm = rm.to_real_reg().unwrap().hw_enc();
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
    fn gen_load_stack(mem: StackAMode, into_reg: Writable<Reg>, ty: Type) -> Self::I { ... }
    fn gen_store_stack(mem: StackAMode, from_reg: Reg, ty: Type) -> Self::I { ... }
    fn gen_move(to_reg: Writable<Reg>, from_reg: Reg, ty: Type) -> Self::I { ... }
    fn gen_rets(rets: Vec<RetPair>) -> Self::I { ... }
    fn gen_add_imm(into_reg: Writable<Reg>, from_reg: Reg, imm: u32) -> SmallVec<[Self::I; 4]> { ... }
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
2. Use `test compile precise-output` to verify end-to-end compilation (the test
   compares the full VCode and disassembly output exactly), and `test run` /
   `test interpret` if your ISA can be run via an interpreter or natively on CI.
3. Add `test verifier` tests to verify that your backend correctly rejects
   invalid IR.

Run with:

```
cargo test -p cranelift-filetests
```

or for a specific file:

```
cargo run -p cranelift-tools -- test path/to/test.clif
```
