from __future__ import absolute_import
from unittest import TestCase
from doctest import DocTestSuite
from base.instructions import iadd, iadd_imm, iconst, icmp
from base.immediates import intcc
from . import xform
from .ast import Var
from .xform import Rtl, XForm


def load_tests(loader, tests, ignore):
    tests.addTests(DocTestSuite(xform))
    return tests


x = Var('x')
y = Var('y')
z = Var('z')
u = Var('u')
a = Var('a')
b = Var('b')
c = Var('c')

CC1 = Var('CC1')
CC2 = Var('CC2')


class TestXForm(TestCase):
    def test_macro_pattern(self):
        src = Rtl(a << iadd_imm(x, y))
        dst = Rtl(
                c << iconst(y),
                a << iadd(x, c))
        XForm(src, dst)

    def test_def_input(self):
        # Src pattern has a def which is an input in dst.
        src = Rtl(a << iadd_imm(x, 1))
        dst = Rtl(y << iadd_imm(a, 1))
        with self.assertRaisesRegexp(
                AssertionError,
                "'a' used as both input and def"):
            XForm(src, dst)

    def test_input_def(self):
        # Converse of the above.
        src = Rtl(y << iadd_imm(a, 1))
        dst = Rtl(a << iadd_imm(x, 1))
        with self.assertRaisesRegexp(
                AssertionError,
                "'a' used as both input and def"):
            XForm(src, dst)

    def test_extra_input(self):
        src = Rtl(a << iadd_imm(x, 1))
        dst = Rtl(a << iadd(x, y))
        with self.assertRaisesRegexp(AssertionError, "extra inputs in dst"):
            XForm(src, dst)

    def test_double_def(self):
        src = Rtl(
                a << iadd_imm(x, 1),
                a << iadd(x, y))
        dst = Rtl(a << iadd(x, y))
        with self.assertRaisesRegexp(AssertionError, "'a' multiply defined"):
            XForm(src, dst)

    def test_subst_imm(self):
        src = Rtl(a << iconst(x))
        dst = Rtl(c << iconst(y))
        assert src.substitution(dst, {}) == {a: c, x: y}

    def test_subst_enum_var(self):
        src = Rtl(a << icmp(CC1, x, y))
        dst = Rtl(b << icmp(CC2, z, u))
        assert src.substitution(dst, {}) == {a: b, CC1: CC2, x: z, y: u}

    def test_subst_enum_const(self):
        src = Rtl(a << icmp(intcc.eq, x, y))
        dst = Rtl(b << icmp(intcc.eq, z, u))
        assert src.substitution(dst, {}) == {a: b, x: z, y: u}

    def test_subst_enum_bad(self):
        src = Rtl(a << icmp(CC1, x, y))
        dst = Rtl(b << icmp(intcc.eq, z, u))
        assert src.substitution(dst, {}) is None

        src = Rtl(a << icmp(intcc.eq, x, y))
        dst = Rtl(b << icmp(CC1, z, u))
        assert src.substitution(dst, {}) is None

        src = Rtl(a << icmp(intcc.eq, x, y))
        dst = Rtl(b << icmp(intcc.sge, z, u))
        assert src.substitution(dst, {}) is None
