"""
Cretonne predicates.

A *predicate* is a function that computes a boolean result. The inputs to the
function determine the kind of predicate:

- An *ISA predicate* is evaluated on the current ISA settings together with the
  shared settings defined in the :py:mod:`settings` module. Once a target ISA
  has been configured, the value of all ISA predicates is known.

- An *Instruction predicate* is evaluated on an instruction instance, so it can
  inspect all the immediate fields and type variables of the instruction.
  Instruction predicates can be evaluatd before register allocation, so they
  can not depend on specific register assignments to the value operands or
  outputs.

Predicates can also be computed from other predicates using the `And`, `Or`,
and `Not` combinators defined in this module.

All predicates have a *context* which determines where they can be evaluated.
For an ISA predicate, the context is the ISA settings group. For an instruction
predicate, the context is the instruction format.
"""
from __future__ import absolute_import
from functools import reduce


def _is_parent(a, b):
    """
    Return true if a is a parent of b, or equal to it.
    """
    while b and a is not b:
        b = getattr(b, 'parent', None)
    return a is b


def _descendant(a, b):
    """
    If a is a parent of b or b is a parent of a, return the descendant of the
    two.

    If neiher is a parent of the other, return None.
    """
    if _is_parent(a, b):
        return b
    if _is_parent(b, a):
        return a
    return None


class Predicate(object):
    """
    Superclass for all computed predicates.

    Leaf predicates can have other types, such as `Setting`.

    :param parts: Tuple of components in the predicate expression.
    """

    def __init__(self, parts):
        self.name = None
        self.parts = parts
        self.context = reduce(
                _descendant,
                (p.predicate_context() for p in parts))
        assert self.context, "Incompatible predicate parts"

    def predicate_context(self):
        return self.context


class And(Predicate):
    """
    Computed predicate that is true if all parts are true.
    """

    precedence = 2

    def __init__(self, *args):
        super(And, self).__init__(args)

    def rust_predicate(self, prec):
        """
        Return a Rust expression computing the value of this predicate.

        The surrounding precedence determines whether parentheses are needed:

        0. An `if` statement.
        1. An `||` expression.
        2. An `&&` expression.
        3. A `!` expression.
        """
        s = ' && '.join(p.rust_predicate(And.precedence) for p in self.parts)
        if prec > And.precedence:
            s = '({})'.format(s)
        return s


class Or(Predicate):
    """
    Computed predicate that is true if any parts are true.
    """

    precedence = 1

    def __init__(self, *args):
        super(Or, self).__init__(args)

    def rust_predicate(self, prec):
        s = ' || '.join(p.rust_predicate(Or.precedence) for p in self.parts)
        if prec > Or.precedence:
            s = '({})'.format(s)
        return s


class Not(Predicate):
    """
    Computed predicate that is true if its single part is false.
    """

    precedence = 3

    def __init__(self, part):
        super(Not, self).__init__((part,))

    def rust_predicate(self, prec):
        return '!' + self.parts[0].rust_predicate(Not.precedence)
