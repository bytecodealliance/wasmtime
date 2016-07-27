********************************
Cretonne Meta Language Reference
********************************

.. default-domain:: py
.. highlight:: python
.. module:: cretonne

The Cretonne meta language is used to define instructions for Cretonne. It is a
domain specific language embedded in Python. This document describes the Python
modules that form the embedded DSL.

The meta language descriptions are Python modules under the :file:`meta`
top-level directory. The descriptions are processed in two steps:

1. The Python modules are imported. This has the effect of building static data
   structures in global variables in the modules. These static data structures
   use the classes in the :mod:`cretonne` module to describe instruction sets
   and other properties.

2. The static data structures are processed to produce Rust source code and
   constant tables.

The main driver for this source code generation process is the
:file:`meta/build.py` script which is invoked as part of the build process if
anything in the :file:`meta` directory has changed since the last build.

Instruction descriptions
========================

New instructions are defined as instances of the :class:`Instruction`
class. As instruction instances are created, they are added to the currently
open :class:`InstructionGroup`.

.. autoclass:: InstructionGroup
    :members:

The basic Cretonne instruction set described in :doc:`langref` is defined by the
Python module :mod:`cretonne.base`. This module has a global variable
:data:`cretonne.base.instructions` which is an :class:`InstructionGroup`
instance containing all the base instructions.

.. autoclass:: Instruction

An instruction is defined with a set of distinct input and output operands which
must be instances of the :class:`Operand` class.

.. autoclass:: Operand

Cretonne uses two separate type systems for operand kinds and SSA values.

Type variables
--------------

Instruction descriptions can be made polymorphic by using :class:`Operand`
instances that refer to a *type variable* instead of a concrete value type.
Polymorphism only works for SSA value operands. Other operands have a fixed
operand kind.

.. autoclass:: TypeVar
    :members:

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
have types from the :class:`cretonne.ValueType` type system; they often have
enumerated values of a specific type. The type of an immediate operand is
indicated with an instance of :class:`ImmediateKind`.

.. autoclass:: ImmediateKind

.. automodule:: cretonne.immediates
    :members:

.. currentmodule:: cretonne

Entity references
-----------------

Instruction operands can also refer to other entties in the same function. This
can be extended basic blocks, or entities declared in the function preamble.

.. autoclass:: EntityRefKind

.. automodule:: cretonne.entities
    :members:

.. currentmodule:: cretonne

Value types
-----------

Concrete value types are represented as instances of :class:`cretonne.ValueType`. There are
subclasses to represent scalar and vector types.

.. autoclass:: ValueType
.. inheritance-diagram:: ValueType ScalarType VectorType IntType FloatType BoolType
    :parts: 1
.. autoclass:: ScalarType
    :members:
.. autoclass:: VectorType
    :members:
.. autoclass:: IntType
    :members:
.. autoclass:: FloatType
    :members:
.. autoclass:: BoolType
    :members:

.. automodule:: cretonne.types
    :members:

.. currentmodule:: cretonne

There are no predefined vector types, but they can be created as needed with
the :func:`ScalarType.by` function.


Instruction representation
==========================

The Rust in-memory representation of instructions is derived from the
instruction descriptions. Part of the representation is generated, and part is
written as Rust code in the `cretonne.instructions` module. The instruction
representation depends on the input operand kinds and whether the instruction
can produce multiple results.

.. autoclass:: OperandKind
.. inheritance-diagram:: OperandKind ImmediateKind EntityRefKind

Since all SSA value operands are represented as a `Value` in Rust code, value
types don't affect the representation. Two special operand kinds are used to
represent SSA values:

.. autodata:: value
.. autodata:: variable_args

When an instruction description is created, it is automatically assigned a
predefined instruction format which is an instance of
:class:`InstructionFormat`:

.. autoclass:: InstructionFormat


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


Targets
=======

Cretonne can be compiled with support for multiple target instruction set
architectures. Each ISA is represented by a :py:class`cretonne.Target` instance.

.. autoclass:: Target

The definitions for each supported target live in a package under
:file:`meta/target`.

.. automodule:: target
    :members:

.. automodule:: target.riscv
