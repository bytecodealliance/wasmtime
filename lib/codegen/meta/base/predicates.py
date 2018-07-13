"""
Cranelift predicates that consider `Function` fields.
"""
from cdsl.predicates import FieldPredicate
from .formats import UnaryGlobalValue, InstructionFormat

try:
    from typing import TYPE_CHECKING
    if TYPE_CHECKING:
        from cdsl.formats import InstructionFormat, FormatField  # noqa
except ImportError:
    pass


class IsColocatedFunc(FieldPredicate):
    """
    An instruction predicate that checks the referenced function is colocated.
    """

    def __init__(self, field):
        # type: (FormatField) -> None
        super(IsColocatedFunc, self).__init__(
            field, 'is_colocated_func', ('func',))


class IsColocatedData(FieldPredicate):
    """
    An instruction predicate that checks the referenced data object is
    colocated.
    """

    def __init__(self):
        # type: () -> None
        super(IsColocatedData, self).__init__(
            UnaryGlobalValue.global_value, 'is_colocated_data', ('func',))


class LengthEquals(FieldPredicate):
    def __init__(self, iform, num):
        # type: (InstructionFormat, int) -> None
        super(LengthEquals, self).__init__(
            iform.args(), 'has_length_of', (num, 'func'))
