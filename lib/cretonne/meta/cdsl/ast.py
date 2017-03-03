"""
Abstract syntax trees.

This module defines classes that can be used to create abstract syntax trees
for patern matching an rewriting of cretonne instructions.
"""
from __future__ import absolute_import
from . import instructions
from .typevar import TypeVar

try:
    from typing import Union, Tuple, Sequence  # noqa
except ImportError:
    pass


class Def(object):
    """
    An AST definition associates a set of variables with the values produced by
    an expression.

    Example:

    >>> from base.instructions import iadd_cout, iconst
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


class Expr(object):
    """
    An AST expression.
    """


class Var(Expr):
    """
    A free variable.

    When variables are used in `XForms` with source and destination patterns,
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
        # The `Def` defining this variable in a source pattern.
        self.src_def = None  # type: Def
        # The `Def` defining this variable in a destination pattern.
        self.dst_def = None  # type: Def
        # TypeVar representing the type of this variable.
        self.typevar = None  # type: TypeVar
        # The original 'typeof(x)' type variable that was created for this Var.
        # This one doesn't change. `self.typevar` above may be joined with
        # other typevars.
        self.original_typevar = None  # type: TypeVar

    def __str__(self):
        # type: () -> str
        return self.name

    def __repr__(self):
        # type: () -> str
        s = self.name
        if self.src_def:
            s += ", src"
        if self.dst_def:
            s += ", dst"
        return "Var({})".format(s)

    # Context bits for `set_def` indicating which pattern has defines of this
    # var.
    SRCCTX = 1
    DSTCTX = 2

    def set_def(self, context, d):
        # type: (int, Def) -> None
        """
        Set the `Def` that defines this variable in the given context.

        The `context` must be one of `SRCCTX` or `DSTCTX`
        """
        if context == self.SRCCTX:
            self.src_def = d
        else:
            self.dst_def = d

    def get_def(self, context):
        # type: (int) -> Def
        """
        Get the def of this variable in context.

        The `context` must be one of `SRCCTX` or `DSTCTX`
        """
        if context == self.SRCCTX:
            return self.src_def
        else:
            return self.dst_def

    def is_input(self):
        # type: () -> bool
        """Is this an input value to the src pattern?"""
        return self.src_def is None and self.dst_def is None

    def is_output(self):
        # type: () -> bool
        """Is this an output value, defined in both src and dst patterns?"""
        return self.src_def is not None and self.dst_def is not None

    def is_intermediate(self):
        # type: () -> bool
        """Is this an intermediate value, defined only in the src pattern?"""
        return self.src_def is not None and self.dst_def is None

    def is_temp(self):
        # type: () -> bool
        """Is this a temp value, defined only in the dst pattern?"""
        return self.src_def is None and self.dst_def is not None

    def get_typevar(self):
        # type: () -> TypeVar
        """Get the type variable representing the type of this variable."""
        if not self.typevar:
            # Create a TypeVar allowing all types.
            tv = TypeVar(
                    'typeof_{}'.format(self),
                    'Type of the pattern variable `{}`'.format(self),
                    ints=True, floats=True, bools=True,
                    scalars=True, simd=True)
            self.original_typevar = tv
            self.typevar = tv
        return self.typevar

    def link_typevar(self, base, derived_func):
        # type: (TypeVar, str) -> None
        """
        Link the type variable on this Var to the type variable `base` using
        `derived_func`.
        """
        self.original_typevar = None
        self.typevar.change_to_derived(base, derived_func)
        # Possibly eliminate redundant SAMEAS links.
        self.typevar = self.typevar.strip_sameas()

    def has_free_typevar(self):
        # type: () -> bool
        """
        Check if this variable has a free type variable.

        If not, the type of this variable is computed from the type of another
        variable.
        """
        if not self.typevar or self.typevar.is_derived:
            return False
        return self.typevar is self.original_typevar

    def rust_type(self):
        # type: () -> str
        """
        Get a Rust expression that computes the type of this variable.

        It is assumed that local variables exist corresponding to the free type
        variables.
        """
        return self.typevar.rust_expr()

    def constrain_typevar(self, sym_typevar, sym_ctrl, ctrl_var):
        # type: (TypeVar, TypeVar, Var) -> None
        """
        Constrain the set of allowed types for this variable.

        Merge type variables for the involved variables to minimize the set for
        free type variables.

        Suppose we're looking at an instruction defined like this:

            c = Operand('c', TxN.as_bool())
            x = Operand('x', TxN)
            y = Operand('y', TxN)
            a = Operand('a', TxN)
            vselect = Instruction('vselect', ins=(c, x, y), outs=a)

        And suppose the instruction is used in a pattern like this:

            v0 << vselect(v1, v2, v3)

        We want to reconcile the types of the variables v0-v3 with the
        constraints from the definition of vselect. This means that v0, v2, and
        v3 must all have the same type, and v1 must have the type
        `typeof(v2).as_bool()`.

        The types are reconciled by calling this function once for each
        input/output operand on the instruction in the pattern with these
        arguments.

        :param sym_typevar: Symbolic type variable constraining this variable
                            in the definition of the instruction.
        :param sym_ctrl: Controlling type variable of `sym_typevar` in the
                         definition of the instruction.
        :param ctrl_var: Variable determining the type of `sym_ctrl`.

        When processing `v1` as used in the pattern above, we would get:

        - self: v1
        - sym_typevar: TxN.as_bool()
        - sym_ctrl: TxN
        - ctrl_var: v2

        Here, 'v2' represents the controlling variable because of how the
        `Ternary` instruction format is defined with `typevar_operand=1`.
        """
        # First check if sym_typevar is tied to the controlling type variable
        # in the instruction definition. We also allow free type variables on
        # instruction inputs that can't be tied to anything else.
        #
        # This also covers non-polymorphic instructions and other cases where
        # we don't have a Var representing the controlling type variable.
        sym_free_var = sym_typevar.free_typevar()
        if not sym_free_var or sym_free_var is not sym_ctrl or not ctrl_var:
            # Just constrain our type to be compatible with the required
            # typeset.
            self.get_typevar().constrain_types(sym_typevar)
            return

        # Now sym_typevar is known to be tied to (or identical to) the
        # controlling type variable.

        if not self.typevar:
            # If this variable is not yet constrained, just infer its type and
            # link it to the controlling type variable.
            if not sym_typevar.is_derived:
                assert sym_typevar is sym_ctrl
                # Identity mapping.
                # Note that `self == ctrl_var` is both possible and common.
                self.typevar = ctrl_var.get_typevar()
            else:
                assert self is not ctrl_var, (
                        'Impossible type constraints for {}: {}'
                        .format(self, sym_typevar))
                # Create a derived type variable identical to sym_typevar, but
                # with a different base.
                self.typevar = TypeVar.derived(
                        ctrl_var.get_typevar(),
                        sym_typevar.derived_func)
            # Match the type set constraints of the instruction.
            self.typevar.constrain_types(sym_typevar)
            return

        # We already have a self.typevar describing our constraints. We need to
        # reconcile with the additional constraints.

        # It's likely that ctrl_var and self already share a type
        # variable. (Often because `ctrl_var == self`).
        if ctrl_var.typevar == self.typevar:
            return

        if not sym_typevar.is_derived:
            assert sym_typevar is sym_ctrl
            # sym_typevar is a direct use of sym_ctrl, so we need to reconcile
            # self with ctrl_var.
            assert not sym_typevar.is_derived
            self.typevar.constrain_types(sym_typevar)

            # It's possible that ctrl_var has not yet been assigned a type
            # variable.
            if not ctrl_var.typevar:
                ctrl_var.typevar = self.typevar
                return

            # We can also bind variables with a free type variable to another
            # variable. Prefer to do this to temps because they aren't allowed
            # to be free,
            if self.is_temp() and self.has_free_typevar():
                self.link_typevar(ctrl_var.typevar, TypeVar.SAMEAS)
                return
            if ctrl_var.is_temp() and ctrl_var.has_free_typevar():
                ctrl_var.link_typevar(self.typevar, TypeVar.SAMEAS)
                return
            if self.has_free_typevar():
                self.link_typevar(ctrl_var.typevar, TypeVar.SAMEAS)
                return
            if ctrl_var.has_free_typevar():
                ctrl_var.link_typevar(self.typevar, TypeVar.SAMEAS)
                return

            # TODO: Other cases are harder to handle.
            #
            # - If either variable is an independent free type variable, it
            #   should be changed to be linked to the other.
            # - If both variable are free, we should pick one to link to the
            #   other. In particular, if one is a temp, it should be linked.
        else:
            # sym_typevar is derived from sym_ctrl.
            # TODO: Other cases are harder to handle.
            pass


class Apply(Expr):
    """
    Apply an instruction to arguments.

    An `Apply` AST expression is created by using function call syntax on
    instructions. This applies to both bound and unbound polymorphic
    instructions:

    >>> from base.instructions import jump, iadd
    >>> jump('next', ())
    Apply(jump, ('next', ()))
    >>> iadd.i32('x', 'y')
    Apply(iadd.i32, ('x', 'y'))

    :param inst: The instruction being applied, an `Instruction` or
                 `BoundInstruction` instance.
    :param args: Tuple of arguments.
    """

    def __init__(self, inst, args):
        # type: (instructions.MaybeBoundInst, Tuple[Expr, ...]) -> None  # noqa
        if isinstance(inst, instructions.BoundInstruction):
            self.inst = inst.inst
            self.typevars = inst.typevars
        else:
            assert isinstance(inst, instructions.Instruction)
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

    def rust_builder(self, defs=None):
        # type: (Sequence[Var]) -> str
        """
        Return a Rust Builder method call for instantiating this instruction
        application.

        The `defs` argument should be a list of variables defined by this
        instruction. It is used to construct a result type if necessary.
        """
        args = ', '.join(map(str, self.args))
        # Do we need to pass an explicit type argument?
        if self.inst.is_polymorphic and not self.inst.use_typevar_operand:
            args = defs[0].rust_type() + ', ' + args
        method = self.inst.snake_name()
        return '{}({})'.format(method, args)
