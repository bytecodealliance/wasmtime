"""
Cretonne predicates that consider `Function` fields.
"""
from cdsl.predicates import FieldPredicate
from .formats import UnaryGlobalVar

try:
    from typing import TYPE_CHECKING
    if TYPE_CHECKING:
        from cdsl.formats import FormatField  # noqa
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
            UnaryGlobalVar.global_var, 'is_colocated_data', ('func',))
