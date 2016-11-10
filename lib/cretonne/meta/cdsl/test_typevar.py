from __future__ import absolute_import
from unittest import TestCase
from doctest import DocTestSuite
from . import typevar
from .typevar import TypeSet, TypeVar
from base.types import i32


def load_tests(loader, tests, ignore):
    tests.addTests(DocTestSuite(typevar))
    return tests


class TestTypeSet(TestCase):
    def test_invalid(self):
        with self.assertRaises(AssertionError):
            TypeSet(lanes=(2, 1))
        with self.assertRaises(AssertionError):
            TypeSet(ints=(32, 16))
        with self.assertRaises(AssertionError):
            TypeSet(floats=(32, 16))
        with self.assertRaises(AssertionError):
            TypeSet(bools=(32, 16))
        with self.assertRaises(AssertionError):
            TypeSet(ints=(32, 33))

    def test_hash(self):
        a = TypeSet(lanes=True, ints=True, floats=True)
        b = TypeSet(lanes=True, ints=True, floats=True)
        c = TypeSet(lanes=True, ints=(8, 16), floats=True)
        self.assertEqual(a, b)
        self.assertNotEqual(a, c)
        s = set()
        s.add(a)
        self.assertTrue(a in s)
        self.assertTrue(b in s)
        self.assertFalse(c in s)

    def test_hash_modified(self):
        a = TypeSet(lanes=True, ints=True, floats=True)
        s = set()
        s.add(a)
        a.max_int = 32
        # Can't rehash after modification.
        with self.assertRaises(AssertionError):
            a in s


class TestTypeVar(TestCase):
    def test_functions(self):
        x = TypeVar('x', 'all ints', ints=True)
        with self.assertRaises(AssertionError):
            x.double_width()
        with self.assertRaises(AssertionError):
            x.half_width()

        x2 = TypeVar('x2', 'i16 and up', ints=(16, 64))
        with self.assertRaises(AssertionError):
            x2.double_width()
        self.assertEqual(str(x2.half_width()), '`half_width(x2)`')
        self.assertEqual(x2.half_width().rust_expr(), 'x2.half_width()')
        self.assertEqual(
                x2.half_width().double_width().rust_expr(),
                'x2.half_width().double_width()')

        x3 = TypeVar('x3', 'up to i32', ints=(8, 32))
        self.assertEqual(str(x3.double_width()), '`double_width(x3)`')
        with self.assertRaises(AssertionError):
            x3.half_width()

    def test_singleton(self):
        x = TypeVar.singleton(i32)
        self.assertEqual(str(x), '`i32`')
        self.assertEqual(x.type_set.min_int, 32)
        self.assertEqual(x.type_set.max_int, 32)
        self.assertEqual(x.type_set.min_lanes, 1)
        self.assertEqual(x.type_set.max_lanes, 1)

        x = TypeVar.singleton(i32.by(4))
        self.assertEqual(str(x), '`i32x4`')
        self.assertEqual(x.type_set.min_int, 32)
        self.assertEqual(x.type_set.max_int, 32)
        self.assertEqual(x.type_set.min_lanes, 4)
        self.assertEqual(x.type_set.max_lanes, 4)
