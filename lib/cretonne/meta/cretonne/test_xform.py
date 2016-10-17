from __future__ import absolute_import
from unittest import TestCase
from doctest import DocTestSuite
from . import xform
from .base import iadd, iadd_imm, iconst
from .ast import Var
from .xform import Rtl, XForm


def load_tests(loader, tests, ignore):
    tests.addTests(DocTestSuite(xform))
    return tests


x = Var('x')
y = Var('y')
a = Var('a')
c = Var('c')


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
