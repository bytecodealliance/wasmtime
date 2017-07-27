from __future__ import absolute_import
from semantics.primitives import prim_to_bv, prim_from_bv, bvsplit, bvconcat,\
    bvadd, bvult, bvzeroext
from .instructions import vsplit, vconcat, iadd, iadd_cout, icmp, bextend
from .immediates import intcc
from cdsl.xform import Rtl
from cdsl.ast import Var
from cdsl.typevar import TypeSet
from cdsl.ti import InTypeset

x = Var('x')
y = Var('y')
a = Var('a')
b = Var('b')
c_out = Var('c_out')
bvc_out = Var('bvc_out')
xhi = Var('xhi')
yhi = Var('yhi')
ahi = Var('ahi')
xlo = Var('xlo')
ylo = Var('ylo')
alo = Var('alo')
lo = Var('lo')
hi = Var('hi')
bvx = Var('bvx')
bvy = Var('bvy')
bva = Var('bva')
bva_wide = Var('bva_wide')
bvlo = Var('bvlo')
bvhi = Var('bvhi')

ScalarTS = TypeSet(lanes=(1, 1), ints=True, floats=True, bools=True)

vsplit.set_semantics(
    (lo, hi) << vsplit(x),
    Rtl(
        bvx << prim_to_bv(x),
        (bvlo, bvhi) << bvsplit(bvx),
        lo << prim_from_bv(bvlo),
        hi << prim_from_bv(bvhi)
    ))

vconcat.set_semantics(
    x << vconcat(lo, hi),
    Rtl(
        bvlo << prim_to_bv(lo),
        bvhi << prim_to_bv(hi),
        bvx << bvconcat(bvlo, bvhi),
        x << prim_from_bv(bvx)
    ))

iadd.set_semantics(
    a << iadd(x, y),
    (Rtl(bvx << prim_to_bv(x),
         bvy << prim_to_bv(y),
         bva << bvadd(bvx, bvy),
         a << prim_from_bv(bva)),
     [InTypeset(x.get_typevar(), ScalarTS)]),
    Rtl((xlo, xhi) << vsplit(x),
        (ylo, yhi) << vsplit(y),
        alo << iadd(xlo, ylo),
        ahi << iadd(xhi, yhi),
        a << vconcat(alo, ahi)))

iadd_cout.set_semantics(
    (a, c_out) << iadd_cout(x, y),
    Rtl(
        bvx << prim_to_bv(x),
        bvy << prim_to_bv(y),
        bva << bvadd(bvx, bvy),
        bvc_out << bvult(bva, bvx),
        a << prim_from_bv(bva),
        c_out << prim_from_bv(bvc_out)
    ))

bextend.set_semantics(
    a << bextend(x),
    (Rtl(
        bvx << prim_to_bv(x),
        bvy << bvzeroext(bvx),
        a << prim_from_bv(bvy)
    ), [InTypeset(x.get_typevar(), ScalarTS)]),
    Rtl((xlo, xhi) << vsplit(x),
        alo << bextend(xlo),
        ahi << bextend(xhi),
        a << vconcat(alo, ahi)))

icmp.set_semantics(
    a << icmp(intcc.ult, x, y),
    (Rtl(
        bvx << prim_to_bv(x),
        bvy << prim_to_bv(y),
        bva << bvult(bvx, bvy),
        bva_wide << bvzeroext(bva),
        a << prim_from_bv(bva_wide),
    ), [InTypeset(x.get_typevar(), ScalarTS)]),
    Rtl((xlo, xhi) << vsplit(x),
        (ylo, yhi) << vsplit(y),
        alo << icmp(intcc.ult, xlo, ylo),
        ahi << icmp(intcc.ult, xhi, yhi),
        b << vconcat(alo, ahi),
        a << bextend(b)))
