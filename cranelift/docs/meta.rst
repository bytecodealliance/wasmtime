*********************************
Cranelift Meta Language Reference
*********************************

.. default-domain:: py
.. highlight:: python

The Cranelift meta language is used to define instructions for Cranelift. It is a
domain specific language embedded in Rust.

.. todo:: Point to the Rust documentation of the meta crate here.

   This document is very out-of-date. Instead, you can have a look at the
   work-in-progress documentation of the `meta` crate there:
   https://docs.rs/cranelift-codegen-meta/0.34.0/cranelift_codegen_meta/.

This document describes the Python modules that form the embedded DSL.

The meta language descriptions are Python modules under the
`cranelift-codegen/meta-python` directory. The descriptions are processed in two
steps:

1. The Python modules are imported. This has the effect of building static data
   structures in global values in the modules. These static data structures
   in the `base` and `isa` packages use the classes in the
   `cdsl` package to describe instruction sets and other properties.

2. The static data structures are processed to produce Rust source code and
   constant tables.

The main driver for this source code generation process is the
`cranelift-codegen/meta-python/build.py` script which is invoked as part of the build
process if anything in the `cranelift-codegen/meta-python` directory has changed
since the last build.

Settings
========

Settings are used by the environment embedding Cranelift to control the details
of code generation. Each setting is defined in the meta language so a compact
and consistent Rust representation can be generated. Shared settings are defined
in the `base.settings` module. Some settings are specific to a target ISA,
and defined in a `settings.py` module under the appropriate
`cranelift-codegen/meta-python/isa/*` directory.

Settings can take boolean on/off values, small numbers, or explicitly enumerated
symbolic values.

All settings must belong to a *group*, represented by a :class:`SettingGroup` object.

Normally, a setting group corresponds to all settings defined in a module. Such
a module looks like this::

    group = SettingGroup('example')

    foo = BoolSetting('use the foo')
    bar = BoolSetting('enable bars', True)
    opt = EnumSetting('optimization level', 'Debug', 'Release')

    group.close(globals())

Instruction descriptions
========================

New instructions are defined as instances of the :class:`Instruction`
class. As instruction instances are created, they are added to the currently
open :class:`InstructionGroup`.

The basic Cranelift instruction set described in :doc:`ir` is defined by the
Python module `base.instructions`. This module has a global value
`base.instructions.GROUP` which is an :class:`InstructionGroup` instance
containing all the base instructions.

An instruction is defined with a set of distinct input and output operands which
must be instances of the :class:`Operand` class.

Cranelift uses two separate type systems for operand kinds and SSA values.

Type variables
--------------

Instruction descriptions can be made polymorphic by using
:class:`cdsl.operands.Operand` instances that refer to a *type variable*
instead of a concrete value type. Polymorphism only works for SSA value
operands. Other operands have a fixed operand kind.

If multiple operands refer to the same type variable they will be required to
have the same concrete type. For example, this defines an integer addition
instruction::

    Int = TypeVar('Int', 'A scalar or vector integer type', ints=True, simd=True)
    a = Operand('a', Int)
    x = Operand('x', Int)
    y = Operand('y', Int)

    iadd = Instruction('iadd', 'Integer addition', ins=(x, y), outs=a)

The type variable `Int` is allowed to vary over all scalar and vector integer
value types, but in a given instance of the `iadd` instruction, the two
operands must have the same type, and the result will be the same type as the
inputs.

There are some practical restrictions on the use of type variables, see
:ref:`restricted-polymorphism`.

Immediate operands
------------------

Immediate instruction operands don't correspond to SSA values, but have values
that are encoded directly in the instruction. Immediate operands don't
have types from the :class:`cdsl.types.ValueType` type system; they often have
enumerated values of a specific type. The type of an immediate operand is
indicated with an instance of :class:`ImmediateKind`.

Entity references
-----------------

Instruction operands can also refer to other entities in the same function. This
can be extended basic blocks, or entities declared in the function preamble.

Value types
-----------

Concrete value types are represented as instances of :class:`ValueType`. There
are subclasses to represent scalar and vector types.

There are no predefined vector types, but they can be created as needed with
the :func:`LaneType.by` function.

Instruction representation
==========================

The Rust in-memory representation of instructions is derived from the
instruction descriptions. Part of the representation is generated, and part is
written as Rust code in the ``cranelift.instructions`` module. The instruction
representation depends on the input operand kinds and whether the instruction
can produce multiple results.

Since all SSA value operands are represented as a `Value` in Rust code, value
types don't affect the representation.

When an instruction description is created, it is automatically assigned a
predefined instruction format which is an instance of
:class:`InstructionFormat`.

.. _restricted-polymorphism:

Restricted polymorphism
-----------------------

The instruction format strictly controls the kinds of operands on an
instruction, but it does not constrain value types at all. A given instruction
description typically does constrain the allowed value types for its value
operands. The type variables give a lot of freedom in describing the value type
constraints, in practice more freedom than what is needed for normal instruction
set architectures. In order to simplify the Rust representation of value type
constraints, some restrictions are imposed on the use of type variables.

A polymorphic instruction has a single *controlling type variable*. For a given
opcode, this type variable must be the type of the first result or the type of
the input value operand designated by the `typevar_operand` argument to the
:py:class:`InstructionFormat` constructor. By default, this is the first value
operand, which works most of the time.

The value types of instruction results must be one of the following:

1. A concrete value type.
2. The controlling type variable.
3. A type variable derived from the controlling type variable.

This means that all result types can be computed from the controlling type
variable.

Input values to the instruction are allowed a bit more freedom. Input value
types must be one of:

1. A concrete value type.
2. The controlling type variable.
3. A type variable derived from the controlling type variable.
4. A free type variable that is not used by any other operands.

This means that the type of an input operand can either be computed from the
controlling type variable, or it can vary independently of the other operands.


Encodings
=========

Encodings describe how Cranelift instructions are mapped to binary machine code
for the target architecture. After the legalization pass, all remaining
instructions are expected to map 1-1 to native instruction encodings. Cranelift
instructions that can't be encoded for the current architecture are called
:term:`illegal instruction`\s.

Some instruction set architectures have different :term:`CPU mode`\s with
incompatible encodings. For example, a modern ARMv8 CPU might support three
different CPU modes: *A64* where instructions are encoded in 32 bits, *A32*
where all instructions are 32 bits, and *T32* which has a mix of 16-bit and
32-bit instruction encodings. These are incompatible encoding spaces, and while
an `iadd` instruction can be encoded in 32 bits in each of them, it's
not the same 32 bits. It's a judgement call if CPU modes should be modelled as
separate targets, or as sub-modes of the same target. In the ARMv8 case, the
different register banks means that it makes sense to model A64 as a separate
target architecture, while A32 and T32 are CPU modes of the 32-bit ARM target.

In a given CPU mode, there may be multiple valid encodings of the same
instruction. Both RISC-V and ARMv8's T32 mode have 32-bit encodings of all
instructions with 16-bit encodings available for some opcodes if certain
constraints are satisfied.

Encodings are guarded by :term:`sub-target predicate`\s. For example, the RISC-V
"C" extension which specifies the compressed encodings may not be supported, and
a predicate would be used to disable all of the 16-bit encodings in that case.
This can also affect whether an instruction is legal. For example, x86 has a
predicate that controls the SSE 4.1 instruction encodings. When that predicate
is false, the SSE 4.1 instructions are not available.

Encodings also have a :term:`instruction predicate` which depends on the
specific values of the instruction's immediate fields. This is used to ensure
that immediate address offsets are within range, for example. The instructions
in the base Cranelift instruction set can often represent a wider range of
immediates than any specific encoding. The fixed-size RISC-style encodings tend
to have more range limitations than CISC-style variable length encodings like
x86.

The diagram below shows the relationship between the classes involved in
specifying instruction encodings:

.. digraph:: encoding

    node [shape=record]
    EncRecipe -> SubtargetPred
    EncRecipe -> InstrFormat
    EncRecipe -> InstrPred
    Encoding [label="{Encoding|Opcode+TypeVars}"]
    Encoding -> EncRecipe [label="+EncBits"]
    Encoding -> CPUMode
    Encoding -> SubtargetPred
    Encoding -> InstrPred
    Encoding -> Opcode
    Opcode -> InstrFormat
    CPUMode -> Target

An :py:class:`Encoding` instance specifies the encoding of a concrete
instruction. The following properties are used to select instructions to be
encoded:

- An opcode, i.e. `iadd_imm`, that must match the instruction's
  opcode.
- Values for any type variables if the opcode represents a polymorphic
  instruction.
- An :term:`instruction predicate` that must be satisfied by the instruction's
  immediate operands.
- The CPU mode that must be active.
- A :term:`sub-target predicate` that must be satisfied by the currently active
  sub-target.

An encoding specifies an *encoding recipe* along with some *encoding bits* that
the recipe can use for native opcode fields etc. The encoding recipe has
additional constraints that must be satisfied:

- An :py:class:`InstructionFormat` that must match the format required by the
  opcodes of any encodings that use this recipe.
- An additional :term:`instruction predicate`.
- An additional :term:`sub-target predicate`.

The additional predicates in the :py:class:`EncRecipe` are merged with the
per-encoding predicates when generating the encoding matcher code. Often
encodings only need the recipe predicates.

Register constraints
====================

After an encoding recipe has been chosen for an instruction, it is the register
allocator's job to make sure that the recipe's :term:`Register constraint`\s
are satisfied. Most ISAs have separate integer and floating point registers,
and instructions can usually only use registers from one of the banks. Some
instruction encodings are even more constrained and can only use a subset of
the registers in a bank. These constraints are expressed in terms of register
classes.

Sometimes the result of an instruction is placed in a register that must be the
same as one of the input registers. Some instructions even use a fixed register
for inputs or results.

Each encoding recipe specifies separate constraints for its value operands and
result. These constraints are separate from the instruction predicate which can
only evaluate the instruction's immediate operands.

Register class constraints
--------------------------

The most common type of register constraint is the register class. It specifies
that an operand or result must be allocated one of the registers from the given
register class::

    IntRegs = RegBank('IntRegs', ISA, 'General purpose registers', units=16, prefix='r')
    GPR = RegClass(IntRegs)
    R = EncRecipe('R', Binary, ins=(GPR, GPR), outs=GPR)

This defines an encoding recipe for the ``Binary`` instruction format where
both input operands must be allocated from the ``GPR`` register class.

Tied register operands
----------------------

In more compact machine code encodings, it is common to require that the result
register is the same as one of the inputs. This is represented with tied
operands::

    CR = EncRecipe('CR', Binary, ins=(GPR, GPR), outs=0)

This indicates that the result value must be allocated to the same register as
the first input value. Tied operand constraints can only be used for result
values, so the number always refers to one of the input values.

Fixed register operands
-----------------------

Some instructions use hard-coded input and output registers for some value
operands. An example is the ``pblendvb`` x86 SSE instruction which takes one
of its three value operands in the hard-coded ``%xmm0`` register::

    XMM0 = FPR[0]
    SSE66_XMM0 = EncRecipe('SSE66_XMM0', Ternary, ins=(FPR, FPR, XMM0), outs=0)

The syntax ``FPR[0]`` selects the first register from the ``FPR`` register
class which consists of all the XMM registers.

Stack operands
--------------

Cranelift's register allocator can assign an SSA value to a stack slot if there
isn't enough registers. It will insert `spill` and `fill`
instructions as needed to satisfy instruction operand constraints, but it is
also possible to have instructions that can access stack slots directly::

    CSS = EncRecipe('CSS', Unary, ins=GPR, outs=Stack(GPR))

An output stack value implies a store to the stack, an input value implies a
load.

Targets
=======

Cranelift can be compiled with support for multiple target instruction set
architectures. Each ISA is represented by a :py:class:`cdsl.isa.TargetISA` instance.

The definitions for each supported target live in a package under
`cranelift-codegen/meta-python/isa`.

Glossary
========

.. glossary::

    Illegal instruction
        An instruction is considered illegal if there is no encoding available
        for the current CPU mode. The legality of an instruction depends on the
        value of :term:`sub-target predicate`\s, so it can't always be
        determined ahead of time.

    CPU mode
        Every target defines one or more CPU modes that determine how the CPU
        decodes binary instructions. Some CPUs can switch modes dynamically with
        a branch instruction (like ARM/Thumb), while other modes are
        process-wide (like x86 32/64-bit).

    Sub-target predicate
        A predicate that depends on the current sub-target configuration.
        Examples are "Use SSE 4.1 instructions", "Use RISC-V compressed
        encodings". Sub-target predicates can depend on both detected CPU
        features and configuration settings.

    Instruction predicate
        A predicate that depends on the immediate fields of an instruction. An
        example is "the load address offset must be a 10-bit signed integer".
        Instruction predicates do not depend on the registers selected for value
        operands.

    Register constraint
        Value operands and results correspond to machine registers. Encodings may
        constrain operands to either a fixed register or a register class. There
        may also be register constraints between operands, for example some
        encodings require that the result register is one of the input
        registers.
