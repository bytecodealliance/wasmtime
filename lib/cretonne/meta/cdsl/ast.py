"""
Abstract syntax trees.

This module defines classes that can be used to create abstract syntax trees
for patern matching an rewriting of cretonne instructions.
"""
from __future__ import absolute_import
from . import instructions
from .typevar import TypeVar
from .predicates import IsEqual, And

try:
    from typing import Union, Tuple, Sequence, TYPE_CHECKING, Dict, List  # noqa
    from typing import Optional, Set # noqa
    if TYPE_CHECKING:
        from .operands import ImmediateKind  # noqa
        from .predicates import PredNode  # noqa
        VarMap = Dict["Var", "Var"]
except ImportError:
    pass


def replace_var(arg, m):
    # type: (Expr, VarMap) -> Expr
    """
    Given a var v return either m[v] or a new variable v' (and remember
    m[v]=v'). Otherwise return the argument unchanged
    """
    if isinstance(arg, Var):
        new_arg = m.get(arg, Var(arg.name))  # type: Var
        m[arg] = new_arg
        return new_arg
    return arg


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
        # type: () -> str
        return "{} << {!r}".format(self.defs, self.expr)

    def __str__(self):
        # type: () -> str
        if len(self.defs) == 1:
            return "{!s} << {!s}".format(self.defs[0], self.expr)
        else:
            return "({}) << {!s}".format(
                    ', '.join(map(str, self.defs)), self.expr)

    def copy(self, m):
        # type: (VarMap) -> Def
        """
        Return a copy of this Def with vars replaced with fresh variables,
        in accordance with the map m. Update m as neccessary.
        """
        new_expr = self.expr.copy(m)
        new_defs = []  # type: List[Var]
        for v in self.defs:
            new_v = replace_var(v, m)
            assert(isinstance(new_v, Var))
            new_defs.append(new_v)

        return Def(tuple(new_defs), new_expr)

    def definitions(self):
        # type: () -> Set[Var]
        """ Return the set of all Vars that are defined by self"""
        return set(self.defs)

    def uses(self):
        # type: () -> Set[Var]
        """ Return the set of all Vars that are used(read) by self"""
        return set(self.expr.vars())

    def vars(self):
        # type: () -> Set[Var]
        """ Return the set of all Vars that appear in self"""
        return self.definitions().union(self.uses())

    def substitution(self, other, s):
        # type: (Def, VarMap) -> Optional[VarMap]
        """
        If the Defs self and other agree structurally, return a variable
        substitution to transform self ot other. Two Defs agree structurally
        if the contained Apply's agree structurally.
        """
        s = self.expr.substitution(other.expr, s)

        if (s is None):
            return s

        assert len(self.defs) == len(other.defs)
        for (self_d, other_d) in zip(self.defs, other.defs):
            assert self_d not in s  # Guaranteed by SSA form
            s[self_d] = other_d

        return s


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
        # This one doesn't change. `self.typevar` above may be changed to
        # another typevar by type inference.
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
                    scalars=True, simd=True, bitvecs=True)
            self.original_typevar = tv
            self.typevar = tv
        return self.typevar

    def set_typevar(self, tv):
        # type: (TypeVar) -> None
        self.typevar = tv

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
        # type: () -> str
        i = self.inst.name
        for t in self.typevars:
            i += '.{}'.format(t)
        return i

    def __repr__(self):
        # type: () -> str
        return "Apply({}, {})".format(self.instname(), self.args)

    def __str__(self):
        # type: () -> str
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

    def inst_predicate(self):
        # type: () -> PredNode
        """
        Construct an instruction predicate that verifies the immediate operands
        on this instruction.

        Immediate operands in a source pattern can be either free variables or
        constants like `Enumerator`. We don't currently support constraints on
        free variables, but we may in the future.
        """
        pred = None  # type: PredNode
        iform = self.inst.format

        # Examine all of the immediate operands.
        for ffield, opnum in zip(iform.imm_fields, self.inst.imm_opnums):
            arg = self.args[opnum]

            # Ignore free variables for now. We may add variable predicates
            # later.
            if isinstance(arg, Var):
                continue

            pred = And.combine(pred, IsEqual(ffield, arg))

        return pred

    def copy(self, m):
        # type: (VarMap) -> Apply
        """
        Return a copy of this Expr with vars replaced with fresh variables,
        in accordance with the map m. Update m as neccessary.
        """
        return Apply(self.inst, tuple(map(lambda e: replace_var(e, m),
                                          self.args)))

    def vars(self):
        # type: () -> Set[Var]
        """ Return the set of all Vars that appear in self"""
        res = set()
        for i in self.inst.value_opnums:
            arg = self.args[i]
            assert isinstance(arg, Var)
            res.add(arg)
        return res

    def substitution(self, other, s):
        # type: (Apply, VarMap) -> Optional[VarMap]
        """
        If the application self and other agree structurally, return a variable
        substitution to transform self ot other. Two applications agree
        structurally if:
            1) They are over the same instruction
            2) Every Var v in self, maps to a single Var w in other. I.e for
               each use of v in self, w is used in the corresponding place in
               other.
        """
        if self.inst != other.inst:
            return None

        # TODO: Should we check imm/cond codes here as well?
        for i in self.inst.value_opnums:
            self_a = self.args[i]
            other_a = other.args[i]

            assert isinstance(self_a, Var) and isinstance(other_a, Var)
            if (self_a not in s):
                s[self_a] = other_a
            else:
                if (s[self_a] != other_a):
                    return None
        return s


class Enumerator(Expr):
    """
    A value of an enumerated immediate operand.

    Some immediate operand kinds like `intcc` and `floatcc` have an enumerated
    range of values corresponding to a Rust enum type. An `Enumerator` object
    is an AST leaf node representing one of the values.

    :param kind: The enumerated `ImmediateKind` containing the value.
    :param value: The textual IL representation of the value.

    `Enumerator` nodes are not usually created directly. They are created by
    using the dot syntax on immediate kinds: `intcc.ult`.
    """

    def __init__(self, kind, value):
        # type: (ImmediateKind, str) -> None
        self.kind = kind
        self.value = value

    def __str__(self):
        # type: () -> str
        """
        Get the Rust expression form of this enumerator.
        """
        return self.kind.rust_enumerator(self.value)

    def __repr__(self):
        # type: () -> str
        return '{}.{}'.format(self.kind, self.value)
