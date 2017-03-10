"""
Instruction transformations.
"""
from __future__ import absolute_import
from .ast import Def, Var, Apply

try:
    from typing import Union, Iterator, Sequence, Iterable, List, Dict  # noqa
    from .ast import Expr  # noqa
    DefApply = Union[Def, Apply]
except ImportError:
    pass


def canonicalize_defapply(node):
    # type: (DefApply) -> Def
    """
    Canonicalize a `Def` or `Apply` node into a `Def`.

    An `Apply` becomes a `Def` with an empty list of defs.
    """
    if isinstance(node, Apply):
        return Def((), node)
    else:
        return node


class Rtl(object):
    """
    Register Transfer Language list.

    An RTL object contains a list of register assignments in the form of `Def`
    objects.

    An RTL list can represent both a source pattern to be matched, or a
    destination pattern to be inserted.
    """

    def __init__(self, *args):
        # type: (*DefApply) -> None
        self.rtl = tuple(map(canonicalize_defapply, args))

    def __iter__(self):
        # type: () -> Iterator[Def]
        return iter(self.rtl)


class XForm(object):
    """
    An instruction transformation consists of a source and destination pattern.

    Patterns are expressed in *register transfer language* as tuples of
    `ast.Def` or `ast.Expr` nodes.

    A legalization pattern must have a source pattern containing only a single
    instruction.

    >>> from base.instructions import iconst, iadd, iadd_imm
    >>> a = Var('a')
    >>> c = Var('c')
    >>> v = Var('v')
    >>> x = Var('x')
    >>> XForm(
    ...     Rtl(c << iconst(v),
    ...         a << iadd(x, c)),
    ...     Rtl(a << iadd_imm(x, v)))
    XForm(inputs=[Var(v), Var(x)], defs=[Var(c, src), Var(a, src, dst)],
      c << iconst(v)
      a << iadd(x, c)
    =>
      a << iadd_imm(x, v)
    )
    """

    def __init__(self, src, dst):
        # type: (Rtl, Rtl) -> None
        self.src = src
        self.dst = dst
        # Variables that are inputs to the source pattern.
        self.inputs = list()  # type: List[Var]
        # Variables defined in either src or dst.
        self.defs = list()  # type: List[Var]

        # Rewrite variables in src and dst RTL lists to our own copies.
        # Map name -> private Var.
        symtab = dict()  # type: Dict[str, Var]
        self._rewrite_rtl(src, symtab, Var.SRCCTX)
        num_src_inputs = len(self.inputs)
        self._rewrite_rtl(dst, symtab, Var.DSTCTX)

        # Check for inconsistently used inputs.
        for i in self.inputs:
            if not i.is_input():
                raise AssertionError(
                        "'{}' used as both input and def".format(i))

        # Check for spurious inputs in dst.
        if len(self.inputs) > num_src_inputs:
            raise AssertionError(
                    "extra inputs in dst RTL: {}".format(
                        self.inputs[num_src_inputs:]))

        self._infer_types(self.src)
        self._infer_types(self.dst)
        self._collect_typevars()

    def __repr__(self):
        s = "XForm(inputs={}, defs={},\n  ".format(self.inputs, self.defs)
        s += '\n  '.join(str(n) for n in self.src)
        s += '\n=>\n  '
        s += '\n  '.join(str(n) for n in self.dst)
        s += '\n)'
        return s

    def _rewrite_rtl(self, rtl, symtab, context):
        # type: (Rtl, Dict[str, Var], int) -> None
        for line in rtl.rtl:
            if isinstance(line, Def):
                line.defs = tuple(
                        self._rewrite_defs(line, symtab, context))
                expr = line.expr
            else:
                expr = line
            self._rewrite_expr(expr, symtab, context)

    def _rewrite_expr(self, expr, symtab, context):
        # type: (Apply, Dict[str, Var], int) -> None
        """
        Find all uses of variables in `expr` and replace them with our own
        local symbols.
        """

        # Accept a whole expression tree.
        stack = [expr]
        while len(stack) > 0:
            expr = stack.pop()
            expr.args = tuple(
                    self._rewrite_uses(expr, stack, symtab, context))

    def _rewrite_defs(self, line, symtab, context):
        # type: (Def, Dict[str, Var], int) -> Iterable[Var]
        """
        Given a tuple of symbols defined in a Def, rewrite them to local
        symbols. Yield the new locals.
        """
        for sym in line.defs:
            name = str(sym)
            if name in symtab:
                var = symtab[name]
                if var.get_def(context):
                    raise AssertionError("'{}' multiply defined".format(name))
            else:
                var = Var(name)
                symtab[name] = var
                self.defs.append(var)
            var.set_def(context, line)
            yield var

    def _rewrite_uses(self, expr, stack, symtab, context):
        # type: (Apply, List[Apply], Dict[str, Var], int) -> Iterable[Expr]
        """
        Given an `Apply` expr, rewrite all uses in its arguments to local
        variables. Yield a sequence of new arguments.

        Append any `Apply` arguments to `stack`.
        """
        for arg, operand in zip(expr.args, expr.inst.ins):
            # Nested instructions are allowed. Visit recursively.
            if isinstance(arg, Apply):
                stack.append(arg)
                yield arg
                continue
            if not isinstance(arg, Var):
                assert not operand.is_value(), "Value arg must be `Var`"
                yield arg
                continue
            # This is supposed to be a symbolic value reference.
            name = str(arg)
            if name in symtab:
                var = symtab[name]
                # The variable must be used consistently as a def or input.
                if not var.is_input() and not var.get_def(context):
                    raise AssertionError(
                            "'{}' used as both input and def"
                            .format(name))
            else:
                # First time use of variable.
                var = Var(name)
                symtab[name] = var
                self.inputs.append(var)
            yield var

    def verify_legalize(self):
        # type: () -> None
        """
        Verify that this is a valid legalization XForm.

        - The source pattern must describe a single instruction.
        - All values defined in the output pattern must be defined in the
          destination pattern.
        """
        assert len(self.src.rtl) == 1, "Legalize needs single instruction."
        for d in self.src.rtl[0].defs:
            if not d.is_output():
                raise AssertionError(
                        '{} not defined in dest pattern'.format(d))

    def _infer_types(self, rtl):
        # type: (Rtl) -> None
        """Assign type variables to all value variables used in `rtl`."""
        for d in rtl.rtl:
            inst = d.expr.inst

            # Get the Var corresponding to the controlling type variable.
            ctrl_var = None  # type: Var
            if inst.is_polymorphic:
                if inst.use_typevar_operand:
                    # Should this be an assertion instead?
                    # Should all value operands be required to be Vars?
                    arg = d.expr.args[inst.format.typevar_operand]
                    if isinstance(arg, Var):
                        ctrl_var = arg
                else:
                    ctrl_var = d.defs[inst.value_results[0]]

            # Reconcile arguments with the requirements of `inst`.
            for opnum in inst.value_opnums:
                inst_tv = inst.ins[opnum].typevar
                v = d.expr.args[opnum]
                if isinstance(v, Var):
                    v.constrain_typevar(inst_tv, inst.ctrl_typevar, ctrl_var)

            # Reconcile results with the requirements of `inst`.
            for resnum in inst.value_results:
                inst_tv = inst.outs[resnum].typevar
                v = d.defs[resnum]
                v.constrain_typevar(inst_tv, inst.ctrl_typevar, ctrl_var)

    def _collect_typevars(self):
        # type: () -> None
        """
        Collect a list of variables whose type can be used to infer the types
        of all expressions.

        This should be called after `_infer_types()` above has computed type
        variables for all the used vars.
        """
        fvars = list(v for v in self.inputs if v.has_free_typevar())
        fvars += list(v for v in self.defs if v.has_free_typevar())
        self.free_typevars = fvars

        # When substituting a pattern, we know the types of all variables that
        # appear on the source side: inut, output, and intermediate values.
        # However, temporary values which appear only on the destination side
        # must have their type computed somehow.
        #
        # Some variables have a fixed type which appears as a type variable
        # with a singleton_type field set. That's allowed for temps too.
        for v in fvars:
            if v.is_temp() and not v.typevar.singleton_type:
                raise AssertionError(
                        "Cannot determine type of temp '{}' in xform:\n{}"
                        .format(v, self))


class XFormGroup(object):
    """
    A group of related transformations.
    """

    def __init__(self, name, doc):
        # type: (str, str) -> None
        self.xforms = list()  # type: List[XForm]
        self.name = name
        self.__doc__ = doc

    def legalize(self, src, dst):
        # type: (Union[Def, Apply], Rtl) -> None
        """
        Add a legalization pattern to this group.

        :param src: Single `Def` or `Apply` to be legalized.
        :param dst: `Rtl` list of replacement instructions.
        """
        xform = XForm(Rtl(src), dst)
        xform.verify_legalize()
        self.xforms.append(xform)
