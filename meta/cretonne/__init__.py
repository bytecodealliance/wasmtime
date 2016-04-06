"""
Cretonne meta language module.

This module provides classes and functions used to describe Cretonne
instructions.
"""

import re

camel_re = re.compile('(^|_)([a-z])')
def camel_case(s):
    """Convert the string s to CamelCase"""
    return camel_re.sub(lambda m: m.group(2).upper(), s)

# Concrete types.
#
# Instances (i8, i32, ...) are provided in the cretonne.types module.

class Type(object):
    """A concrete value type."""

    def __init__(self, name, membytes, doc):
        self.name = name
        self.membytes = membytes
        self.__doc__ = doc

    def __str__(self):
        return self.name

class ScalarType(Type):
    """
    A concrete scalar (not vector) type.

    Also tracks a unique set of :py:class:`VectorType` instances with this type
    as the lane type.
    """

    def __init__(self, name, membytes, doc):
        super(ScalarType, self).__init__(name, membytes, doc)
        self._vectors = dict()

    def __repr__(self):
        return 'ScalarType({})'.format(self.name)

    def by(self, lanes):
        """
        Get a vector type with this type as the lane type.

        For example, ``i32.by(4)`` returns the :obj:`i32x4` type.
        """
        if lanes in self._vectors:
            return self._vectors[lanes]
        else:
            v = VectorType(self, lanes)
            self._vectors[lanes] = v
            return v

class VectorType(Type):
    """
    A concrete SIMD vector type.

    A vector type has a lane type which is an instance of :class:`ScalarType`,
    and a positive number of lanes.
    """

    def __init__(self, base, lanes):
        assert isinstance(base, ScalarType), 'SIMD lanes must be scalar types'
        super(VectorType, self).__init__(
                name='{}x{}'.format(base.name, lanes),
                membytes=lanes*base.membytes,
                doc="""
                A SIMD vector with {} lanes containing a {} each.
                """.format(lanes, base.name))
        self.base = base
        self.lanes = lanes

    def __repr__(self):
        return 'VectorType(base={}, lanes={})'.format(self.base.name, self.lanes)

class IntType(ScalarType):
    """A concrete scalar integer type."""

    def __init__(self, bits):
        assert bits > 0, 'IntType must have positive number of bits'
        super(IntType, self).__init__(
                name='i{:d}'.format(bits),
                membytes=bits/8,
                doc="An integer type with {} bits.".format(bits))
        self.bits = bits

    def __repr__(self):
        return 'IntType(bits={})'.format(self.bits)

class FloatType(ScalarType):
    """A concrete scalar floating point type."""

    def __init__(self, bits, doc):
        assert bits > 0, 'FloatType must have positive number of bits'
        super(FloatType, self).__init__( name='f{:d}'.format(bits), membytes=bits/8, doc=doc)
        self.bits = bits

    def __repr__(self):
        return 'FloatType(bits={})'.format(self.bits)

class BoolType(ScalarType):
    """A concrete scalar boolean type."""

    def __init__(self, bits):
        assert bits > 0, 'BoolType must have positive number of bits'
        super(BoolType, self).__init__(
                name='b{:d}'.format(bits),
                membytes=bits/8,
                doc="A boolean type with {} bits.".format(bits))
        self.bits = bits

    def __repr__(self):
        return 'BoolType(bits={})'.format(self.bits)

#
# Parametric polymorphism.
#

class TypeVar(object):
    """
    A Type Variable.

    Type variables can be used in place of concrete types when defining
    instructions. This makes the instructions *polymorphic*.
    """

    def __init__(self, name, doc):
        self.name = name
        self.__doc__ = doc

#
# Immediate operands.
#
# Instances of immediate operand types are provided in the cretonne.immediates
# module.

class ImmediateType(object):
    """
    The type of an immediate instruction operand.
    """

    def __init__(self, name, doc):
        self.name = name
        self.__doc__ = doc

    def __str__(self):
        return self.name

    def __repr__(self):
        return 'ImmediateType({})'.format(self.name)

#
# Defining instructions.
#

class InstructionGroup(object):
    """
    An instruction group.

    Every instruction must belong to exactly one instruction group. A given
    target architecture can support instructions from multiple groups, and it
    does not necessarily support all instructions in a group.

    New instructions are automatically added to the currently open instruction
    group.
    """

    # The currently open instruction group.
    _current = None

    def open(self):
        """
        Open this instruction group such that future new instructions are
        added to this group.
        """
        assert InstructionGroup._current is None, (
                "Can't open {} since {} is already open".format(self, _current))
        InstructionGroup._current = self

    def close(self):
        """
        Close this instruction group. This function should be called before
        opening another instruction group.
        """
        assert InstructionGroup._current is self, (
                "Can't close {}, the open instuction group is {}".format(self, _current))
        InstructionGroup._current = None

    def __init__(self, name, doc):
        self.name = name
        self.__doc__ = doc
        self.instructions = []
        self.open()

    @staticmethod
    def append(inst):
        assert InstructionGroup._current, "Open an instruction group before defining instructions."
        InstructionGroup._current.instructions.append(inst)

class Operand(object):
    """
    An instruction operand.

    An instruction operand can be either an *immediate* or an *SSA value*. The
    type of the operand is one of:

    1. A :py:class:`Type` instance indicates an SSA value operand with a
       concrete type.

    2. A :py:class:`TypeVar` instance indicates an SSA value operand, and the
       instruction is polymorphic over the possible concrete types that the type
       variable can assume.

    3. An :py:class:`ImmediateType` instance indicates an immediate operand
       whose value is encoded in the instruction itself rather than being passed
       as an SSA value.

    """
    def __init__(self, name, typ, doc=''):
        self.name = name
        self.typ = typ
        self.__doc__ = doc

    def get_doc(self):
        if self.__doc__:
            return self.__doc__
        else:
            return self.typ.__doc__

class Instruction(object):
    """
    An instruction.

    The operands to the instruction are specified as two tuples: ``ins`` and
    ``outs``. Since the Python singleton tuple syntax is a bit awkward, it is
    allowed to specify a singleton as just the operand itself, i.e., `ins=x` and
    `ins=(x,)` are both allowed and mean the same thing.

    :param name: Instruction mnemonic, also becomes opcode name.
    :param doc: Documentation string.
    :param ins: Tuple of input operands. This can be a mix of SSA value operands
                and immediate operands.
    :param outs: Tuple of output operands. The output operands can't be
                 immediates.
    """

    def __init__(self, name, doc, ins=(), outs=(), **kwargs):
        self.name = name
        self.camel_name = camel_case(name)
        self.__doc__ = doc
        self.ins = self._to_operand_tuple(ins)
        self.outs = self._to_operand_tuple(outs)
        InstructionGroup.append(self)

    @staticmethod
    def _to_operand_tuple(x):
        # Allow a single Operand instance instead of the awkward singleton tuple
        # syntax.
        if isinstance(x, Operand):
            x = (x,)
        else:
            x = tuple(x)
        for op in x:
            assert isinstance(op, Operand)
        return x

#
# Defining targets
#
class Target(object):
    """
    A target instruction set architecture.

    The `Target` class collects everything known about a target ISA.

    :param name: Short mnemonic name for the ISA.
    :param instruction_groups: List of `InstructionGroup` instances that are
        relevant for this ISA.
    """

    def __init__(self, name, instrution_groups):
        self.name = name
        self.instruction_groups = instrution_groups
