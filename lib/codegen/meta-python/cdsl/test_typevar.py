from __future__ import absolute_import
from unittest import TestCase
from doctest import DocTestSuite
from . import typevar
from .typevar import TypeSet, TypeVar
from base.types import i32, i16, b1, f64
from itertools import product
from functools import reduce


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

    def test_preimage(self):
        t = TypeSet(lanes=(1, 1), ints=(8, 8), floats=(32, 32))

        # LANEOF
        self.assertEqual(TypeSet(lanes=True, ints=(8, 8), floats=(32, 32)),
                         t.preimage(TypeVar.LANEOF))
        # Inverse of empty set is still empty across LANEOF
        self.assertEqual(TypeSet(),
                         TypeSet().preimage(TypeVar.LANEOF))

        # ASBOOL
        t = TypeSet(lanes=(1, 4), bools=(1, 64))
        self.assertEqual(t.preimage(TypeVar.ASBOOL),
                         TypeSet(lanes=(1, 4), ints=True, bools=True,
                                 floats=True))

        # Half/Double Vector
        t = TypeSet(lanes=(1, 1), ints=(8, 8))
        t1 = TypeSet(lanes=(256, 256), ints=(8, 8))
        self.assertEqual(t.preimage(TypeVar.DOUBLEVECTOR).size(), 0)
        self.assertEqual(t1.preimage(TypeVar.HALFVECTOR).size(), 0)

        t = TypeSet(lanes=(1, 16), ints=(8, 16), floats=(32, 32))
        t1 = TypeSet(lanes=(64, 256), bools=(1, 32))

        self.assertEqual(t.preimage(TypeVar.DOUBLEVECTOR),
                         TypeSet(lanes=(1, 8), ints=(8, 16), floats=(32, 32)))
        self.assertEqual(t1.preimage(TypeVar.HALFVECTOR),
                         TypeSet(lanes=(128, 256), bools=(1, 32)))

        # Half/Double Width
        t = TypeSet(ints=(8, 8), floats=(32, 32), bools=(1, 8))
        t1 = TypeSet(ints=(64, 64), floats=(64, 64), bools=(64, 64))
        self.assertEqual(t.preimage(TypeVar.DOUBLEWIDTH).size(), 0)
        self.assertEqual(t1.preimage(TypeVar.HALFWIDTH).size(), 0)

        t = TypeSet(lanes=(1, 16), ints=(8, 16), floats=(32, 64))
        t1 = TypeSet(lanes=(64, 256), bools=(1, 64))

        self.assertEqual(t.preimage(TypeVar.DOUBLEWIDTH),
                         TypeSet(lanes=(1, 16), ints=(8, 8), floats=(32, 32)))
        self.assertEqual(t1.preimage(TypeVar.HALFWIDTH),
                         TypeSet(lanes=(64, 256), bools=(16, 64)))


def has_non_bijective_derived_f(iterable):
    return any(not TypeVar.is_bijection(x) for x in iterable)


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

    def test_stress_constrain_types(self):
        # Get all 43 possible derived vars of length up to 2
        funcs = [TypeVar.LANEOF,
                 TypeVar.ASBOOL, TypeVar.HALFVECTOR, TypeVar.DOUBLEVECTOR,
                 TypeVar.HALFWIDTH, TypeVar.DOUBLEWIDTH]
        v = [()] + [(x,) for x in funcs] + list(product(*[funcs, funcs]))

        # For each pair of derived variables
        for (i1, i2) in product(v, v):
            # Compute the derived sets for each  starting with a full typeset
            full_ts = TypeSet(lanes=True, floats=True, ints=True, bools=True)
            ts1 = reduce(lambda ts, func:   ts.image(func), i1, full_ts)
            ts2 = reduce(lambda ts, func:   ts.image(func), i2, full_ts)

            # Compute intersection
            intersect = ts1.copy()
            intersect &= ts2

            # Propagate intersections backward
            ts1_src = reduce(lambda ts, func:   ts.preimage(func),
                             reversed(i1),
                             intersect)
            ts2_src = reduce(lambda ts, func:   ts.preimage(func),
                             reversed(i2),
                             intersect)

            # If the intersection or its propagated forms are empty, then these
            # two variables can never overlap. For example x.double_vector and
            # x.lane_of.
            if (intersect.size() == 0 or ts1_src.size() == 0 or
                    ts2_src.size() == 0):
                continue

            # Should be safe to create derived tvs from ts1_src and ts2_src
            tv1 = reduce(lambda tv, func:   TypeVar.derived(tv, func),
                         i1,
                         TypeVar.from_typeset(ts1_src))

            tv2 = reduce(lambda tv, func:   TypeVar.derived(tv, func),
                         i2,
                         TypeVar.from_typeset(ts2_src))

            # In the absence of AS_BOOL image(preimage(f)) == f so the
            # typesets of tv1 and tv2 should be exactly intersection
            assert tv1.get_typeset() == intersect or\
                has_non_bijective_derived_f(i1)

            assert tv2.get_typeset() == intersect or\
                has_non_bijective_derived_f(i2)
