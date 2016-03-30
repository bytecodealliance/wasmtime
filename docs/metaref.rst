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
module :mod:`cretonne.instrs`.

.. module:: cretonne

Types
=====

Concrete value types are represented as instances of :class:`cretonne.Type`. There are
subclasses to represent scalar and vector types.

.. inheritance-diagram:: Type ScalarType VectorType IntType FloatType
    :parts: 1
.. autoclass:: Type
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

Immediates
----------

Immediate instruction operands don't correspond to SSA values, but have values
that are encoded directly in the instruction. Immediate operands don't
have types from the :class:`cretonne.Type` type system; they often have
enumerated values of a specific type. The type of an immediate operand is
indicated with an instance of :class:`ImmediateType`.

.. autoclass:: ImmediateType

.. automodule:: cretonne.immediates
    :members:

.. currentmodule:: cretonne
