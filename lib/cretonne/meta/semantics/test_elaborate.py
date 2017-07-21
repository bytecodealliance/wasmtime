from __future__ import absolute_import
from base.instructions import vselect, vsplit, vconcat, iconst, iadd, bint
from base.instructions import b1, icmp, ireduce
from base.immediates import intcc
from base.types import i64, i8, b32, i32, i16, f32
from cdsl.typevar import TypeVar
from cdsl.ast import Var
from cdsl.xform import Rtl
from unittest import TestCase
from .elaborate import cleanup_concrete_rtl, elaborate, is_rtl_concrete,\
    cleanup_semantics
from .primitives import prim_to_bv, bvsplit, prim_from_bv, bvconcat, bvadd
import base.semantics  # noqa


def concrete_rtls_eq(r1, r2):
    # type: (Rtl, Rtl) -> bool
    """
    Check whether 2 concrete Rtls are equivalent. That is:
        1) They are structurally the same (i.e. there is a substitution between
        them)
        2) Corresponding Vars between them have the same singleton type.
    """
    assert is_rtl_concrete(r1)
    assert is_rtl_concrete(r2)

    s = r1.substitution(r2, {})

    if s is None:
        return False

    for (v, v1) in s.items():
        if v.get_typevar().singleton_type() !=\
           v1.get_typevar().singleton_type():
            return False

    return True


class TestCleanupConcreteRtl(TestCase):
    """
    Test cleanup_concrete_rtl(). cleanup_concrete_rtl() should take Rtls for
    which we can infer a single concrete typing, and update the TypeVars
    in-place to singleton TVs.
    """
    def test_cleanup_concrete_rtl(self):
        # type: () -> None
        typ = i64.by(4)
        x = Var('x')
        lo = Var('lo')
        hi = Var('hi')

        x.set_typevar(TypeVar.singleton(typ))
        r = Rtl(
                (lo, hi) << vsplit(x),
        )
        r1 = cleanup_concrete_rtl(r)

        s = r.substitution(r1, {})
        assert s is not None
        assert s[x].get_typevar().singleton_type() == typ
        assert s[lo].get_typevar().singleton_type() == i64.by(2)
        assert s[hi].get_typevar().singleton_type() == i64.by(2)

    def test_cleanup_concrete_rtl_fail(self):
        # type: () -> None
        x = Var('x')
        lo = Var('lo')
        hi = Var('hi')
        r = Rtl(
                (lo, hi) << vsplit(x),
        )

        with self.assertRaises(AssertionError):
            cleanup_concrete_rtl(r)

    def test_cleanup_concrete_rtl_ireduce(self):
        # type: () -> None
        x = Var('x')
        y = Var('y')
        x.set_typevar(TypeVar.singleton(i8.by(2)))
        r = Rtl(
                y << ireduce(x),
        )

        r1 = cleanup_concrete_rtl(r)

        s = r.substitution(r1, {})
        assert s is not None
        assert s[x].get_typevar().singleton_type() == i8.by(2)
        assert s[y].get_typevar().singleton_type() == i8.by(2)

    def test_cleanup_concrete_rtl_ireduce_bad(self):
        # type: () -> None
        x = Var('x')
        y = Var('y')
        x.set_typevar(TypeVar.singleton(i16.by(1)))
        r = Rtl(
                y << ireduce(x),
        )

        with self.assertRaises(AssertionError):
            cleanup_concrete_rtl(r)

    def test_vselect_icmpimm(self):
        # type: () -> None
        x = Var('x')
        y = Var('y')
        z = Var('z')
        w = Var('w')
        v = Var('v')
        zeroes = Var('zeroes')
        imm0 = Var("imm0")

        zeroes.set_typevar(TypeVar.singleton(i32.by(4)))
        z.set_typevar(TypeVar.singleton(f32.by(4)))

        r = Rtl(
                zeroes << iconst(imm0),
                y << icmp(intcc.eq, x, zeroes),
                v << vselect(y, z, w),
        )

        r1 = cleanup_concrete_rtl(r)

        s = r.substitution(r1, {})
        assert s is not None
        assert s[zeroes].get_typevar().singleton_type() == i32.by(4)
        assert s[x].get_typevar().singleton_type() == i32.by(4)
        assert s[y].get_typevar().singleton_type() == b32.by(4)
        assert s[z].get_typevar().singleton_type() == f32.by(4)
        assert s[w].get_typevar().singleton_type() == f32.by(4)
        assert s[v].get_typevar().singleton_type() == f32.by(4)

    def test_bint(self):
        # type: () -> None
        x = Var('x')
        y = Var('y')
        z = Var('z')
        w = Var('w')
        v = Var('v')
        u = Var('u')

        x.set_typevar(TypeVar.singleton(i32.by(8)))
        z.set_typevar(TypeVar.singleton(i32.by(8)))
        # TODO: Relax this to simd=True
        v.set_typevar(TypeVar('v', '', bools=(1, 1), simd=(8, 8)))

        r = Rtl(
            z << iadd(x, y),
            w << bint(v),
            u << iadd(z, w)
        )

        r1 = cleanup_concrete_rtl(r)

        s = r.substitution(r1, {})
        assert s is not None
        assert s[x].get_typevar().singleton_type() == i32.by(8)
        assert s[y].get_typevar().singleton_type() == i32.by(8)
        assert s[z].get_typevar().singleton_type() == i32.by(8)
        assert s[w].get_typevar().singleton_type() == i32.by(8)
        assert s[u].get_typevar().singleton_type() == i32.by(8)
        assert s[v].get_typevar().singleton_type() == b1.by(8)


class TestElaborate(TestCase):
    """
    Test semantics elaboration.
    """
    def setUp(self):
        # type: () -> None
        self.v0 = Var("v0")
        self.v1 = Var("v1")
        self.v2 = Var("v2")
        self.v3 = Var("v3")
        self.v4 = Var("v4")
        self.v5 = Var("v5")
        self.v6 = Var("v6")
        self.v7 = Var("v7")
        self.v8 = Var("v8")
        self.v9 = Var("v9")
        self.imm0 = Var("imm0")
        self.IxN_nonscalar = TypeVar("IxN_nonscalar", "", ints=True,
                                     scalars=False, simd=True)
        self.TxN = TypeVar("TxN", "", ints=True, bools=True, floats=True,
                           scalars=False, simd=True)
        self.b1 = TypeVar.singleton(b1)

    def test_elaborate_vsplit(self):
        # type: () -> None
        i32.by(4)  # Make sure i32x4 exists.
        i32.by(2)  # Make sure i32x2 exists.
        r = Rtl(
                (self.v0, self.v1) << vsplit.i32x4(self.v2),
        )
        sem = elaborate(cleanup_concrete_rtl(r))
        bvx = Var('bvx')
        bvlo = Var('bvlo')
        bvhi = Var('bvhi')
        x = Var('x')
        lo = Var('lo')
        hi = Var('hi')

        assert concrete_rtls_eq(sem, cleanup_concrete_rtl(Rtl(
            bvx << prim_to_bv.i32x4(x),
            (bvlo, bvhi) << bvsplit.bv128(bvx),
            lo << prim_from_bv.i32x2.bv64(bvlo),
            hi << prim_from_bv.i32x2.bv64(bvhi))))

    def test_elaborate_vconcat(self):
        # type: () -> None
        i32.by(4)  # Make sure i32x4 exists.
        i32.by(2)  # Make sure i32x2 exists.
        r = Rtl(
                self.v0 << vconcat.i32x2(self.v1, self.v2),
        )
        sem = elaborate(cleanup_concrete_rtl(r))
        bvx = Var('bvx')
        bvlo = Var('bvlo')
        bvhi = Var('bvhi')
        x = Var('x')
        lo = Var('lo')
        hi = Var('hi')

        assert concrete_rtls_eq(sem, cleanup_concrete_rtl(Rtl(
            bvlo << prim_to_bv.i32x2(lo),
            bvhi << prim_to_bv.i32x2(hi),
            bvx << bvconcat.bv64(bvlo, bvhi),
            x << prim_from_bv.i32x4.bv128(bvx))))

    def test_elaborate_iadd_simple(self):
        # type: () -> None
        i32.by(2)  # Make sure i32x2 exists.
        x = Var('x')
        y = Var('y')
        a = Var('a')
        bvx = Var('bvx')
        bvy = Var('bvy')
        bva = Var('bva')
        r = Rtl(
                a << iadd.i32(x, y),
        )
        sem = elaborate(cleanup_concrete_rtl(r))

        assert concrete_rtls_eq(sem, cleanup_concrete_rtl(Rtl(
            bvx << prim_to_bv.i32(x),
            bvy << prim_to_bv.i32(y),
            bva << bvadd.bv32(bvx, bvy),
            a << prim_from_bv.i32.bv32(bva))))

    def test_elaborate_iadd_elaborate_1(self):
        # type: () -> None
        i32.by(2)  # Make sure i32x2 exists.
        r = Rtl(
                self.v0 << iadd.i32x2(self.v1, self.v2),
        )
        sem = cleanup_semantics(elaborate(cleanup_concrete_rtl(r)),
                                set([self.v0]))
        x = Var('x')
        y = Var('y')
        a = Var('a')
        bvx_1 = Var('bvx_1')
        bvx_2 = Var('bvx_2')
        bvx_5 = Var('bvx_5')
        bvlo_1 = Var('bvlo_1')
        bvlo_2 = Var('bvlo_2')
        bvhi_1 = Var('bvhi_1')
        bvhi_2 = Var('bvhi_2')

        bva_3 = Var('bva_3')
        bva_4 = Var('bva_4')

        assert concrete_rtls_eq(sem, cleanup_concrete_rtl(Rtl(
            bvx_1 << prim_to_bv.i32x2(x),
            (bvlo_1, bvhi_1) << bvsplit.bv64(bvx_1),
            bvx_2 << prim_to_bv.i32x2(y),
            (bvlo_2, bvhi_2) << bvsplit.bv64(bvx_2),
            bva_3 << bvadd.bv32(bvlo_1, bvlo_2),
            bva_4 << bvadd.bv32(bvhi_1, bvhi_2),
            bvx_5 << bvconcat.bv32(bva_3, bva_4),
            a << prim_from_bv.i32x2.bv64(bvx_5))))

    def test_elaborate_iadd_elaborate_2(self):
        # type: () -> None
        i8.by(4)  # Make sure i32x2 exists.
        r = Rtl(
                self.v0 << iadd.i8x4(self.v1, self.v2),
        )

        sem = cleanup_semantics(elaborate(cleanup_concrete_rtl(r)),
                                set([self.v0]))
        x = Var('x')
        y = Var('y')
        a = Var('a')
        bvx_1 = Var('bvx_1')
        bvx_2 = Var('bvx_2')
        bvx_5 = Var('bvx_5')
        bvx_10 = Var('bvx_10')
        bvx_15 = Var('bvx_15')

        bvlo_1 = Var('bvlo_1')
        bvlo_2 = Var('bvlo_2')
        bvlo_6 = Var('bvlo_6')
        bvlo_7 = Var('bvlo_7')
        bvlo_11 = Var('bvlo_11')
        bvlo_12 = Var('bvlo_12')

        bvhi_1 = Var('bvhi_1')
        bvhi_2 = Var('bvhi_2')
        bvhi_6 = Var('bvhi_6')
        bvhi_7 = Var('bvhi_7')
        bvhi_11 = Var('bvhi_11')
        bvhi_12 = Var('bvhi_12')

        bva_8 = Var('bva_8')
        bva_9 = Var('bva_9')
        bva_13 = Var('bva_13')
        bva_14 = Var('bva_14')

        assert concrete_rtls_eq(sem, cleanup_concrete_rtl(Rtl(
            bvx_1 << prim_to_bv.i8x4(x),
            (bvlo_1, bvhi_1) << bvsplit.bv32(bvx_1),
            bvx_2 << prim_to_bv.i8x4(y),
            (bvlo_2, bvhi_2) << bvsplit.bv32(bvx_2),
            (bvlo_6, bvhi_6) << bvsplit.bv16(bvlo_1),
            (bvlo_7, bvhi_7) << bvsplit.bv16(bvlo_2),
            bva_8 << bvadd.bv8(bvlo_6, bvlo_7),
            bva_9 << bvadd.bv8(bvhi_6, bvhi_7),
            bvx_10 << bvconcat.bv8(bva_8, bva_9),
            (bvlo_11, bvhi_11) << bvsplit.bv16(bvhi_1),
            (bvlo_12, bvhi_12) << bvsplit.bv16(bvhi_2),
            bva_13 << bvadd.bv8(bvlo_11, bvlo_12),
            bva_14 << bvadd.bv8(bvhi_11, bvhi_12),
            bvx_15 << bvconcat.bv8(bva_13, bva_14),
            bvx_5 << bvconcat.bv16(bvx_10, bvx_15),
            a << prim_from_bv.i8x4.bv32(bvx_5))))
