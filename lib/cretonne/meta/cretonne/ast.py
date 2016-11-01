"""
Abstract syntax trees.

This module defines classes that can be used to create abstract syntax trees
for patern matching an rewriting of cretonne instructions.
"""
from __future__ import absolute_import
from . import Instruction, BoundInstruction

try:
    from typing import Union, Tuple  # noqa
except ImportError:
    pass


class Def(object):
    """
    An AST definition associates a set of variables with the values produced by
    an expression.

    Example:

    >>> from .base import iadd_cout, iconst
    >>> x = Var('x')
    >>> y = Var('y')
    >>> x << iconst(4)
    (Var(x),) << Apply(iconst, (4,))
    >>> (x, y) << iadd_cout(4, 5)
    (Var(x), Var(y)) << Apply(iadd_cout, (4, 5))

    The `<<` operator is used to create variable definitions.

    :param defs: Single variable or tuple of variables to be defined.
    :param expr: Expression generating the values.
    """

    def __init__(self, defs, expr):
        # type: (Union[Var, Tuple[Var, ...]], Apply) -> None
        if not isinstance(defs, tuple):
            self.defs = (defs,)  # type: Tuple[Var, ...]
        else:
            self.defs = defs
        assert isinstance(expr, Apply)
        self.expr = expr

    def __repr__(self):
        return "{} << {!r}".format(self.defs, self.expr)

    def __str__(self):
        if len(self.defs) == 1:
            return "{!s} << {!s}".format(self.defs[0], self.expr)
        else:
            return "({}) << {!s}".format(
                    ', '.join(map(str, self.defs)), self.expr)

    def root_inst(self):
        # type: () -> Instruction
        """Get the instruction at the root of this tree."""
        return self.expr.root_inst()

    def defs_expr(self):
        # type: () -> Tuple[Tuple[Var, ...], Apply]
        """Split into a defs tuple and an Apply expr."""
        return (self.defs, self.expr)


class Expr(object):
    """
    An AST expression.
    """


class Var(Expr):
    """
    A free variable.

    When variables are used in `XForms` with source ans destination patterns,
    they are classified as follows:

    Input values
        Uses in the source pattern with no preceding def. These may appear as
        inputs in the destination pattern too, but no new inputs can be
        introduced.
    Output values
        Variables that are defined in both the source and destination pattern.
        These values may have uses outside the source pattern, and the
        destination pattern must compute the same value.
    Intermediate values
        Values that are defined in the source pattern, but not in the
        destination pattern. These may have uses outside the source pattern, so
        the defining instruction can't be deleted immediately.
    Temporary values
        Values that are defined only in the destination pattern.
    """

    def __init__(self, name):
        # type: (str) -> None
        self.name = name
        # Bitmask of contexts where this variable is defined.
        # See XForm._rewrite_defs().
        self.defctx = 0

    # Context bits for `defctx` indicating which pattern has defines of this
    # var.
    SRCCTX = 1
    DSTCTX = 2

    def __str__(self):
        # type: () -> str
        return self.name

    def __repr__(self):
        # type: () -> str
        s = self.name
        if self.defctx:
            s += ", d={:02b}".format(self.defctx)
        return "Var({})".format(s)

    def is_input(self):
        # type: () -> bool
        """Is this an input value to the source pattern?"""
        return self.defctx == 0

    def is_output(self):
        """Is this an output value, defined in both src and dest patterns?"""
        # type: () -> bool
        return self.defctx == self.SRCCTX | self.DSTCTX

    def is_intermediate(self):
        """Is this an intermediate value, defined only in the src pattern?"""
        # type: () -> bool
        return self.defctx == self.SRCCTX

    def is_temp(self):
        """Is this a temp value, defined only in the dest pattern?"""
        # type: () -> bool
        return self.defctx == self.DSTCTX


class Apply(Expr):
    """
    Apply an instruction to arguments.

    An `Apply` AST expression is created by using function call syntax on
    instructions. This applies to both bound and unbound polymorphic
    instructions:

    >>> from .base import jump, iadd
    >>> jump('next', ())
    Apply(jump, ('next', ()))
    >>> iadd.i32('x', 'y')
    Apply(iadd.i32, ('x', 'y'))

    :param inst: The instruction being applied, an `Instruction` or
                 `BoundInstruction` instance.
    :param args: Tuple of arguments.
    """

    def __init__(self, inst, args):
        # type: (Union[Instruction, BoundInstruction], Tuple[Expr, ...]) -> None  # noqa
        if isinstance(inst, BoundInstruction):
            self.inst = inst.inst
            self.typevars = inst.typevars
        else:
            assert isinstance(inst, Instruction)
            self.inst = inst
            self.typevars = ()
        self.args = args
        assert len(self.inst.ins) == len(args)

    def __rlshift__(self, other):
        # type: (Union[Var, Tuple[Var, ...]]) -> Def
        """
        Define variables using `var << expr` or `(v1, v2) << expr`.
        """
        return Def(other, self)

    def instname(self):
        i = self.inst.name
        for t in self.typevars:
            i += '.{}'.format(t)
        return i

    def __repr__(self):
        return "Apply({}, {})".format(self.instname(), self.args)

    def __str__(self):
        args = ', '.join(map(str, self.args))
        return '{}({})'.format(self.instname(), args)

    def root_inst(self):
        # type: () -> Instruction
        """Get the instruction at the root of this tree."""
        return self.inst

    def defs_expr(self):
        # type: () -> Tuple[Tuple[Var, ...], Apply]
        """Split into a defs tuple and an Apply expr."""
        return ((), self)
