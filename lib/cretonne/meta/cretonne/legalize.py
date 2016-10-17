"""
Patterns for legalizing the `base` instruction set.

The base Cretonne instruction set is 'fat', and many instructions don't have
legal representations in a given target ISA. This module defines legalization
patterns that describe how base instructions can be transformed to other base
instructions that are legal.
"""
from __future__ import absolute_import
from .base import iadd, iadd_cout, iadd_cin, isplit_lohi, iconcat_lohi
from .base import isub, isub_bin, isub_bout
from .ast import Var
from .xform import Rtl, XFormGroup


narrow = XFormGroup()

x = Var('x')
y = Var('y')
a = Var('a')
b = Var('b')
c = Var('c')
xl = Var('xl')
xh = Var('xh')
yl = Var('yl')
yh = Var('yh')
al = Var('al')
ah = Var('ah')

narrow.legalize(
        a << iadd(x, y),
        Rtl(
            (xl, xh) << isplit_lohi(x),
            (yl, yh) << isplit_lohi(y),
            (al, c) << iadd_cout(xl, yl),
            ah << iadd_cin(xh, yh, c),
            a << iconcat_lohi(al, ah)
        ))

narrow.legalize(
        a << isub(x, y),
        Rtl(
            (xl, xh) << isplit_lohi(x),
            (yl, yh) << isplit_lohi(y),
            (al, b) << isub_bout(xl, yl),
            ah << isub_bin(xh, yh, b),
            a << iconcat_lohi(al, ah)
        ))
