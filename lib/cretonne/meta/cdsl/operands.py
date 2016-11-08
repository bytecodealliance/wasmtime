"""Classes for describing instruction operands."""
from __future__ import absolute_import
from . import camel_case


# Kinds of operands.
#
# Each instruction has an opcode and a number of operands. The opcode
# determines the instruction format, and the format determines the number of
# operands and the kind of each operand.
class OperandKind(object):
    """
    An instance of the `OperandKind` class corresponds to a kind of operand.
    Each operand kind has a corresponding type in the Rust representation of an
    instruction.
    """

    def __init__(self, name, doc, default_member=None, rust_type=None):
        # type: (str, str, str, str) -> None
        self.name = name
        self.__doc__ = doc
        self.default_member = default_member
        # The camel-cased name of an operand kind is also the Rust type used to
        # represent it.
        self.rust_type = rust_type or camel_case(name)

    def __str__(self):
        # type: () -> str
        return self.name

    def __repr__(self):
        # type: () -> str
        return 'OperandKind({})'.format(self.name)

    def operand_kind(self):
        # type: () -> OperandKind
        """
        An `OperandKind` instance can be used directly as the type of an
        `Operand` when defining an instruction.
        """
        return self

    def free_typevar(self):
        # Return the free typevariable controlling the type of this operand.
        return None

#: An SSA value operand. This is a value defined by another instruction.
VALUE = OperandKind(
        'value', """
        An SSA value defined by another instruction.

        This kind of operand can represent any SSA value type, but the
        instruction format may restrict the valid value types for a given
        operand.
        """)

#: A variable-sized list of value operands. Use for Ebb and function call
#: arguments.
VARIABLE_ARGS = OperandKind(
        'variable_args', """
        A variable size list of `value` operands.

        Use this to represent arguemtns passed to a function call, arguments
        passed to an extended basic block, or a variable number of results
        returned from an instruction.
        """,
        default_member='varargs')


# Instances of immediate operand types are provided in the
# `cretonne.immediates` module.
class ImmediateKind(OperandKind):
    """
    The kind of an immediate instruction operand.

    :param default_member: The default member name of this kind the
                           `InstructionData` data structure.
    """

    def __init__(self, name, doc, default_member='imm', rust_type=None):
        # type: (str, str, str, str) -> None
        super(ImmediateKind, self).__init__(
                name, doc, default_member, rust_type)

    def __repr__(self):
        # type: () -> str
        return 'ImmediateKind({})'.format(self.name)


# Instances of entity reference operand types are provided in the
# `cretonne.entities` module.
class EntityRefKind(OperandKind):
    """
    The kind of an entity reference instruction operand.
    """

    def __init__(self, name, doc, default_member=None, rust_type=None):
        # type: (str, str, str, str) -> None
        super(EntityRefKind, self).__init__(
                name, doc, default_member or name, rust_type)

    def __repr__(self):
        # type: () -> str
        return 'EntityRefKind({})'.format(self.name)
