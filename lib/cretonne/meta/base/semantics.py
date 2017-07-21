from __future__ import absolute_import
from semantics.primitives import prim_to_bv, prim_from_bv, bvsplit, bvconcat,\
    bvadd
from .instructions import vsplit, vconcat, iadd
from cdsl.xform import XForm, Rtl
from cdsl.ast import Var
from cdsl.typevar import TypeSet
from cdsl.ti import InTypeset
import semantics.types # noqa

x = Var('x')
y = Var('y')
a = Var('a')
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
bvlo = Var('bvlo')
bvhi = Var('bvhi')

ScalarTS = TypeSet(lanes=(1, 1), ints=True, floats=True, bools=True)

vsplit.set_semantics(
    XForm(Rtl((lo, hi) << vsplit(x)),
          Rtl(bvx << prim_to_bv(x),
              (bvlo, bvhi) << bvsplit(bvx),
              lo << prim_from_bv(bvlo),
              hi << prim_from_bv(bvhi))))

vconcat.set_semantics(
    XForm(Rtl(x << vconcat(lo, hi)),
          Rtl(bvlo << prim_to_bv(lo),
              bvhi << prim_to_bv(hi),
              bvx << bvconcat(bvlo, bvhi),
              x << prim_from_bv(bvx))))

iadd.set_semantics([
     XForm(Rtl(a << iadd(x, y)),
           Rtl(bvx << prim_to_bv(x),
               bvy << prim_to_bv(y),
               bva << bvadd(bvx, bvy),
               a << prim_from_bv(bva)),
           constraints=[InTypeset(x.get_typevar(), ScalarTS)]),
     XForm(Rtl(a << iadd(x, y)),
           Rtl((xlo, xhi) << vsplit(x),
               (ylo, yhi) << vsplit(y),
               alo << iadd(xlo, ylo),
               ahi << iadd(xhi, yhi),
               a << vconcat(alo, ahi)))
])
