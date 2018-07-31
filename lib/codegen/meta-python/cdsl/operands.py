"""Classes for describing instruction operands."""
from __future__ import absolute_import
from . import camel_case
from .types import ValueType
from .typevar import TypeVar

try:
    from typing import Union, Dict, TYPE_CHECKING, Iterable  # noqa
    OperandSpec = Union['OperandKind', ValueType, TypeVar]
    if TYPE_CHECKING:
        from .ast import Enumerator, ConstantInt, ConstantBits, Literal  # noqa
except ImportError:
    pass


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
        self.rust_type = rust_type or ('ir::' + camel_case(name))

    def __str__(self):
        # type: () -> str
        return self.name

    def __repr__(self):
        # type: () -> str
        return 'OperandKind({})'.format(self.name)


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

        Use this to represent arguments passed to a function call, arguments
        passed to an extended basic block, or a variable number of results
        returned from an instruction.
        """,
        rust_type='&[Value]')


# Instances of immediate operand types are provided in the
# `cranelift.immediates` module.
class ImmediateKind(OperandKind):
    """
    The kind of an immediate instruction operand.

    :param default_member: The default member name of this kind the
                           `InstructionData` data structure.
    """

    def __init__(
            self, name, doc,
            default_member='imm',
            rust_type=None,
            values=None):
        # type: (str, str, str, str, Dict[str, str]) -> None
        if rust_type is None:
            rust_type = 'ir::immediates::' + camel_case(name)
        super(ImmediateKind, self).__init__(
                name, doc, default_member, rust_type)
        self.values = values

    def __repr__(self):
        # type: () -> str
        return 'ImmediateKind({})'.format(self.name)

    def __getattr__(self, value):
        # type: (str) -> Enumerator
        """
        Enumerated immediate kinds allow the use of dot syntax to produce
        `Enumerator` AST nodes: `icmp.i32(intcc.ult, a, b)`.
        """
        from .ast import Enumerator  # noqa
        if not self.values:
            raise AssertionError(
                    '{n} is not an enumerated operand kind: {n}.{a}'.format(
                        n=self.name, a=value))
        if value not in self.values:
            raise AssertionError(
                    'No such {n} enumerator: {n}.{a}'.format(
                        n=self.name, a=value))
        return Enumerator(self, value)

    def __call__(self, value):
        # type: (int) -> ConstantInt
        """
        Create an AST node representing a constant integer:

            iconst(imm64(0))
        """
        from .ast import ConstantInt  # noqa
        if self.values:
            raise AssertionError(
                    "{}({}): Can't make a constant numeric value for an enum"
                    .format(self.name, value))
        return ConstantInt(self, value)

    def bits(self, bits):
        # type: (int) -> ConstantBits
        """
        Create an AST literal node for the given bitwise representation of this
        immediate operand kind.
        """
        from .ast import ConstantBits  # noqa
        return ConstantBits(self, bits)

    def rust_enumerator(self, value):
        # type: (str) -> str
        """
        Get the qualified Rust name of the enumerator value `value`.
        """
        return '{}::{}'.format(self.rust_type, self.values[value])

    def is_enumerable(self):
        # type: () -> bool
        return self.values is not None

    def possible_values(self):
        # type: () -> Iterable[Literal]
        from cdsl.ast import Enumerator # noqa
        assert self.is_enumerable()
        for v in self.values.keys():
            yield Enumerator(self, v)


# Instances of entity reference operand types are provided in the
# `cranelift.entities` module.
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


class Operand(object):
    """
    An instruction operand can be an *immediate*, an *SSA value*, or an *entity
    reference*. The type of the operand is one of:

    1. A :py:class:`ValueType` instance indicates an SSA value operand with a
       concrete type.

    2. A :py:class:`TypeVar` instance indicates an SSA value operand, and the
       instruction is polymorphic over the possible concrete types that the
       type variable can assume.

    3. An :py:class:`ImmediateKind` instance indicates an immediate operand
       whose value is encoded in the instruction itself rather than being
       passed as an SSA value.

    4. An :py:class:`EntityRefKind` instance indicates an operand that
       references another entity in the function, typically something declared
       in the function preamble.

    """
    def __init__(self, name, typ, doc=''):
        # type: (str, OperandSpec, str) -> None
        self.name = name
        self.__doc__ = doc

        # Decode the operand spec and set self.kind.
        # Only VALUE operands have a typevar member.
        if isinstance(typ, ValueType):
            self.kind = VALUE
            self.typevar = TypeVar.singleton(typ)
        elif isinstance(typ, TypeVar):
            self.kind = VALUE
            self.typevar = typ
        else:
            assert isinstance(typ, OperandKind)
            self.kind = typ

    def get_doc(self):
        # type: () -> str
        if self.__doc__:
            return self.__doc__
        if self.kind is VALUE:
            return self.typevar.__doc__
        return self.kind.__doc__

    def __str__(self):
        # type: () -> str
        return "`{}`".format(self.name)

    def is_value(self):
        # type: () -> bool
        """
        Is this an SSA value operand?
        """
        return self.kind is VALUE

    def is_varargs(self):
        # type: () -> bool
        """
        Is this a VARIABLE_ARGS operand?
        """
        return self.kind is VARIABLE_ARGS

    def is_immediate(self):
        # type: () -> bool
        """
        Is this an immediate operand?

        Note that this includes both `ImmediateKind` operands *and* entity
        references. It is any operand that doesn't represent a value
        dependency.
        """
        return self.kind is not VALUE and self.kind is not VARIABLE_ARGS

    def is_cpu_flags(self):
        # type: () -> bool
        """
        Is this a CPU flags operand?
        """
        return self.kind is VALUE and self.typevar.name in ['iflags', 'fflags']
