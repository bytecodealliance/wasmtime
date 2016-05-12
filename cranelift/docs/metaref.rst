********************************
Cretonne Meta Language Reference
********************************

.. default-domain:: py
.. highlight:: python

The Cretonne meta language is used to define instructions for Cretonne. It is a
domain specific language embedded in Python.

An instruction set is described by a Python module under the :file:`meta`
directory that has a global variable called ``instructions``. The basic
Cretonne instruction set described in :doc:`langref` is defined by the Python
module :mod:`cretonne.base`.

.. module:: cretonne

Value Types
===========

Concrete value types are represented as instances of :class:`cretonne.ValueType`. There are
subclasses to represent scalar and vector types.

.. inheritance-diagram:: ValueType ScalarType VectorType IntType FloatType
    :parts: 1
.. autoclass:: ValueType
.. autoclass:: ScalarType
    :members:
.. autoclass:: VectorType
    :members:
.. autoclass:: IntType
    :members:
.. autoclass:: FloatType
    :members:

Predefined types
----------------
.. automodule:: cretonne.types
    :members:

.. currentmodule:: cretonne

Parametric polymorphism
-----------------------

Instruction operands can be defined with *type variables* instead of concrete
types for their operands. This makes the instructions polymorphic.

.. autoclass:: TypeVar

Instructions
============

New instructions are defined as instances of the :class:`cretonne.Instruction`
class.

.. autoclass:: Instruction
.. autoclass:: Operand
.. autoclass:: OperandKind
.. autoclass:: InstructionGroup
    :members:


Immediates
----------

Immediate instruction operands don't correspond to SSA values, but have values
that are encoded directly in the instruction. Immediate operands don't
have types from the :class:`cretonne.ValueType` type system; they often have
enumerated values of a specific type. The type of an immediate operand is
indicated with an instance of :class:`ImmediateKind`.

.. autoclass:: ImmediateKind

.. automodule:: cretonne.immediates
    :members:

.. currentmodule:: cretonne

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
