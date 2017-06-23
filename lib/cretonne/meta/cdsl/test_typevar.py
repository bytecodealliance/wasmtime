from __future__ import absolute_import
from unittest import TestCase
from doctest import DocTestSuite
from . import typevar
from .typevar import TypeSet, TypeVar
from base.types import i32, i16, b1, f64


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
        a.ints.remove(64)
        # Can't rehash after modification.
        with self.assertRaises(AssertionError):
            a in s

    def test_forward_images(self):
        a = TypeSet(lanes=(2, 8), ints=(8, 8), floats=(32, 32))
        b = TypeSet(lanes=(1, 8), ints=(8, 8), floats=(32, 32))
        self.assertEqual(a.lane_of(), TypeSet(ints=(8, 8), floats=(32, 32)))

        c = TypeSet(lanes=(2, 8))
        c.bools = set([8, 32])

        # Test case with disjoint intervals
        self.assertEqual(a.as_bool(), c)

        # For as_bool check b1 is present when 1 \in lanes
        d = TypeSet(lanes=(1, 8))
        d.bools = set([1, 8, 32])
        self.assertEqual(b.as_bool(), d)

        self.assertEqual(TypeSet(lanes=(1, 32)).half_vector(),
                         TypeSet(lanes=(1, 16)))

        self.assertEqual(TypeSet(lanes=(1, 32)).double_vector(),
                         TypeSet(lanes=(2, 64)))

        self.assertEqual(TypeSet(lanes=(128, 256)).double_vector(),
                         TypeSet(lanes=(256, 256)))

        self.assertEqual(TypeSet(ints=(8, 32)).half_width(),
                         TypeSet(ints=(8, 16)))

        self.assertEqual(TypeSet(ints=(8, 32)).double_width(),
                         TypeSet(ints=(16, 64)))

        self.assertEqual(TypeSet(ints=(32, 64)).double_width(),
                         TypeSet(ints=(64, 64)))

        # Should produce an empty ts
        self.assertEqual(TypeSet(floats=(32, 32)).half_width(),
                         TypeSet())

        self.assertEqual(TypeSet(floats=(32, 64)).half_width(),
                         TypeSet(floats=(32, 32)))

        self.assertEqual(TypeSet(floats=(32, 32)).double_width(),
                         TypeSet(floats=(64, 64)))

        self.assertEqual(TypeSet(floats=(32, 64)).double_width(),
                         TypeSet(floats=(64, 64)))

        # Bools have trickier behavior around b1 (since b2, b4 don't exist)
        self.assertEqual(TypeSet(bools=(1, 8)).half_width(),
                         TypeSet())

        t = TypeSet()
        t.bools = set([8, 16])
        self.assertEqual(TypeSet(bools=(1, 32)).half_width(), t)

        # double_width() of bools={1, 8, 16} must not include 2 or 8
        t.bools = set([16, 32])
        self.assertEqual(TypeSet(bools=(1, 16)).double_width(), t)

        self.assertEqual(TypeSet(bools=(32, 64)).double_width(),
                         TypeSet(bools=(64, 64)))

    def test_get_singleton(self):
        # Raise error when calling get_singleton() on non-singleton TS
        t = TypeSet(lanes=(1, 1), ints=(8, 8), floats=(32, 32))
        with self.assertRaises(AssertionError):
            t.get_singleton()
        t = TypeSet(lanes=(1, 2), floats=(32, 32))

        with self.assertRaises(AssertionError):
            t.get_singleton()

        self.assertEqual(TypeSet(ints=(16, 16)).get_singleton(), i16)
        self.assertEqual(TypeSet(floats=(64, 64)).get_singleton(), f64)
        self.assertEqual(TypeSet(bools=(1, 1)).get_singleton(), b1)
        self.assertEqual(TypeSet(lanes=(4, 4), ints=(32, 32)).get_singleton(),
                         i32.by(4))


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
        self.assertEqual(min(x.type_set.ints), 32)
        self.assertEqual(max(x.type_set.ints), 32)
        self.assertEqual(min(x.type_set.lanes), 1)
        self.assertEqual(max(x.type_set.lanes), 1)
        self.assertEqual(len(x.type_set.floats), 0)
        self.assertEqual(len(x.type_set.bools), 0)

        x = TypeVar.singleton(i32.by(4))
        self.assertEqual(str(x), '`i32x4`')
        self.assertEqual(min(x.type_set.ints), 32)
        self.assertEqual(max(x.type_set.ints), 32)
        self.assertEqual(min(x.type_set.lanes), 4)
        self.assertEqual(max(x.type_set.lanes), 4)
        self.assertEqual(len(x.type_set.floats), 0)
        self.assertEqual(len(x.type_set.bools), 0)
