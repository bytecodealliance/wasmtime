from __future__ import absolute_import
from unittest import TestCase
from doctest import DocTestSuite
from . import ast
from base.instructions import jump, iadd


def load_tests(loader, tests, ignore):
    tests.addTests(DocTestSuite(ast))
    return tests


x = 'x'
y = 'y'
a = 'a'


class TestPatterns(TestCase):
    def test_apply(self):
        i = jump(x, y)
        self.assertEqual(repr(i), "Apply(jump, ('x', 'y'))")

        i = iadd.i32(x, y)
        self.assertEqual(repr(i), "Apply(iadd.i32, ('x', 'y'))")

    def test_single_ins(self):
        pat = a << iadd.i32(x, y)
        self.assertEqual(repr(pat), "('a',) << Apply(iadd.i32, ('x', 'y'))")
