# Cranelift Backend Architecture

This document describes the architecture of Cranelift's code generation
backend — the pipeline that takes optimized CLIF IR and produces binary
machine code.

## Overview

After machine-independent optimization passes run on CLIF IR, the backend
pipeline takes over. The pipeline consists of the following stages:

```
ir::Function            (optimized CLIF IR)
    |
    | [instruction selection / lowering via ISLE]
    |
VCode<arch::Inst>       (virtual-register machine instructions)
    |
    | [register allocation via regalloc2]
    |
    | [branch resolution and binary emission via MachBuffer]
    |
Vec<u8>                 (binary machine code)
```

## Key Data Structures

### `VCode<I>`

`VCode` (virtual-register code) is the central container for machine
instructions in the backend. It holds:

- A list of machine instructions of type `I` (the ISA-specific instruction
  type, e.g. `aarch64::inst::Inst`).
- A mapping from machine instruction indices to basic block indices.
- ABI/calling-convention state (the `Callee` struct).
- A table of embedded constants referenced by instructions.

`VCode` implements the `regalloc2::Function` trait so that the register
allocator can operate on it directly. The ISA-specific `Inst` type implements
the `MachInst` trait which provides information about register uses/defs,
branch targets, and instruction emission.

### `MachInst` trait

Every ISA backend defines a type that implements `MachInst`. This trait
provides:

- `get_operands`: enumerate register uses and definitions (for regalloc)
- `is_term`: whether this is a block terminator
- `branch_destination`: return target blocks for branch instructions
- `emit`: encode the instruction into bytes in a `MachBuffer`

### `MachBuffer`

`MachBuffer` is a streaming encoder that accumulates binary code and handles:

- Branch-target patching and relaxation (short vs. long branches)
- Constant pool islands inserted between blocks for out-of-range constants
- Relocation records

### `Callee` and the ABI Layer

`Callee` (from `machinst::abi`) manages the calling convention and stack frame
for a function. It is responsible for:

- Allocating stack slots and spill slots
- Emitting prologue (register saves, stack pointer adjustment) and epilogue
- Handling argument passing and return values according to the calling
  convention (System V, Windows x64, etc.)

Each ISA implements the `ABIMachineSpec` trait to specialize the generic
`Callee` logic for the particular register file and calling conventions of
that ISA.

## Instruction Selection via ISLE

Instruction selection is performed by ISLE (Instruction Selection and Lowering
Engine), a domain-specific language for writing pattern-matching lowering rules.

### ISLE compilation model

ISLE source files (`.isle`) are compiled at build time by the ISLE compiler
(in `cranelift/isle`) into Rust code. The generated Rust is checked in at
`cranelift/codegen/src/isa/<arch>/lower/isle/generated_code.rs` to allow
building Cranelift without running the ISLE compiler (though it is
automatically re-run when any `.isle` file changes).

### How lowering works

The entry point is `Lower::lower()` in `cranelift/codegen/src/machinst/lower.rs`.
It iterates over CLIF basic blocks in reverse post-order and for each
instruction calls the ISLE-generated `lower()` function.

The ISLE generated code operates as a term-rewriting system over CLIF
instruction patterns. A CLIF `Opcode` (plus optional type and immediates) is
matched against patterns, and the matching rule emits one or more machine
instructions into the `VCodeBuilder`.

### ISLE file organization

For a given backend (e.g. `aarch64`):

- `cranelift/codegen/src/isa/aarch64/inst/*.isle`: machine instruction
  definitions (the `MInstruction` term and its variants)
- `cranelift/codegen/src/isa/aarch64/lower.isle`: lowering rules (patterns
  matching CLIF and emitting machine instructions)
- `cranelift/codegen/src/prelude.isle`: shared declarations available to all
  backends
- `cranelift/codegen/src/prelude_lower.isle`: helpers for lowering shared
  across all backends
- `cranelift/codegen/src/opts/*.isle`: mid-end CLIF-to-CLIF optimization rules

Auto-generated ISLE files (from `cranelift/codegen/build.rs`) provide `extern`
declarations for CLIF opcodes and types so that ISLE rules can reference them
by name.

## Register Allocation

Register allocation is performed by [regalloc2], an external crate. After
lowering, `VCode` is passed to regalloc2 which returns an `Output` containing
the allocation decisions. The allocation is applied during binary emission:
each instruction's `emit` method receives the `regalloc2::Output` and looks
up the physical registers assigned to each virtual register.

Cranelift supports two regalloc algorithms, selectable via the
`regalloc_algorithm` setting:
- `backtracking` (default): the Ion backtracking allocator, good quality
- `single_pass`: a fast single-pass allocator, lower quality but faster

[regalloc2]: https://github.com/bytecodealliance/regalloc2

## Backend module layout

Each ISA backend lives in `cranelift/codegen/src/isa/<arch>/` and contains:

```
mod.rs          - TargetIsa implementation, compile_vcode()
settings.rs     - ISA-specific settings (generated from meta)
abi.rs          - ABIMachineSpec implementation
inst/
    mod.rs      - MachInst implementation and instruction enum
    args.rs     - Operand types (register kinds, addressing modes, immediates)
    emit.rs     - Binary encoding (MachInstEmit impl)
    regs.rs     - Register definitions
    imms.rs     - Immediate helper types
lower/
    isle.rs     - Glue between the Rust lowering entry point and ISLE
    isle/
        generated_code.rs  - ISLE compiler output (checked in)
*.isle          - ISLE source files
```

## Optimization pipeline context

The backend receives an already-optimized `ir::Function`. The optimization
pipeline that runs before backend lowering includes:

1. **E-graph optimization** (`cranelift/codegen/src/egraph/`): GVN, algebraic
   simplifications, and rewrites expressed as ISLE rules in
   `cranelift/codegen/src/opts/*.isle`.
2. **Alias analysis** (`cranelift/codegen/src/alias_analysis.rs`): redundant
   load elimination.

These passes are ISA-independent and run on CLIF IR before any ISA-specific
code is invoked.
