import doctest
import gen_legalizer
from unittest import TestCase
from srcgen import Formatter
from gen_legalizer import get_runtime_typechecks, emit_runtime_typecheck
from base.instructions import vselect, vsplit, isplit, iconcat, vconcat, \
    iconst, b1, icmp, copy, sextend, uextend, ireduce, fdemote, fpromote # noqa
from base.legalize import narrow, expand # noqa
from base.immediates import intcc # noqa
from cdsl.typevar import TypeVar, TypeSet
from cdsl.ast import Var, Def # noqa
from cdsl.xform import Rtl, XForm # noqa
from cdsl.ti import ti_rtl, subst, TypeEnv, get_type_env # noqa
from unique_table import UniqueTable
from functools import reduce

try:
    from typing import Callable, TYPE_CHECKING, Iterable, Any # noqa
    if TYPE_CHECKING:
        CheckProducer = Callable[[UniqueTable], str]
except ImportError:
    TYPE_CHECKING = False


def load_tests(loader, tests, ignore):
    # type: (Any, Any, Any) -> Any
    tests.addTests(doctest.DocTestSuite(gen_legalizer))
    return tests


def format_check(typesets, s, *args):
    # type: (...) -> str
    def transform(x):
        # type: (Any) -> str
        if isinstance(x, TypeSet):
            return str(typesets.index[x])
        elif isinstance(x, TypeVar):
            assert not x.is_derived
            return x.name
        else:
            return str(x)

    dummy_s = s  # type: str
    args = tuple(map(lambda x:  transform(x), args))
    return dummy_s.format(*args)


def typeset_check(v, ts):
    # type: (Var, TypeSet) -> CheckProducer
    return lambda typesets: format_check(
        typesets,
        'let predicate = predicate && TYPE_SETS[{}].contains(typeof_{});\n',
        ts, v)


def equiv_check(tv1, tv2):
    # type: (str, str) -> CheckProducer
    return lambda typesets: format_check(
        typesets,
        'let predicate = predicate && match ({}, {}) {{\n'
        '    (Some(a), Some(b)) => a == b,\n'
        '    _ => false,\n'
        '}};\n', tv1, tv2)


def wider_check(tv1, tv2):
    # type: (str, str) -> CheckProducer
    return lambda typesets: format_check(
        typesets,
        'let predicate = predicate && match ({}, {}) {{\n'
        '    (Some(a), Some(b)) => a.wider_or_equal(b),\n'
        '    _ => false,\n'
        '}};\n', tv1, tv2)


def sequence(*args):
    # type: (...) -> CheckProducer
    dummy = args  # type: Iterable[CheckProducer]

    def sequenceF(typesets):
        # type: (UniqueTable) -> str
        def strconcat(acc, el):
            # type: (str, CheckProducer) -> str
            return acc + el(typesets)

        return reduce(strconcat, dummy, "")
    return sequenceF


class TestRuntimeChecks(TestCase):

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

    def check_yo_check(self, xform, expected_f):
        # type: (XForm, CheckProducer) -> None
        fmt = Formatter()
        type_sets = UniqueTable()
        for check in get_runtime_typechecks(xform):
            emit_runtime_typecheck(check, fmt, type_sets)

        # Remove comments
        got = "".join([l for l in fmt.lines if not l.strip().startswith("//")])
        expected = expected_f(type_sets)
        self.assertEqual(got, expected)

    def test_width_check(self):
        # type: () -> None
        x = XForm(Rtl(self.v0 << copy(self.v1)),
                  Rtl((self.v2, self.v3) << isplit(self.v1),
                      self.v0 << iconcat(self.v2, self.v3)))

        WideInt = TypeSet(lanes=(1, 256), ints=(16, 64))
        self.check_yo_check(x, typeset_check(self.v1, WideInt))

    def test_lanes_check(self):
        # type: () -> None
        x = XForm(Rtl(self.v0 << copy(self.v1)),
                  Rtl((self.v2, self.v3) << vsplit(self.v1),
                      self.v0 << vconcat(self.v2, self.v3)))

        WideVec = TypeSet(lanes=(2, 256), ints=(8, 64), floats=(32, 64),
                          bools=(1, 64))
        self.check_yo_check(x, typeset_check(self.v1, WideVec))

    def test_vselect_imm(self):
        # type: () -> None
        ts = TypeSet(lanes=(2, 256), ints=True, floats=True, bools=(8, 64))
        r = Rtl(
                self.v0 << iconst(self.imm0),
                self.v1 << icmp(intcc.eq, self.v2, self.v0),
                self.v5 << vselect(self.v1, self.v3, self.v4),
        )
        x = XForm(r, r)
        tv2_exp = 'Some({}).map(|t: ir::Type| t.as_bool())'\
            .format(self.v2.get_typevar().name)
        tv3_exp = 'Some({}).map(|t: ir::Type| t.as_bool())'\
            .format(self.v3.get_typevar().name)

        self.check_yo_check(
            x, sequence(typeset_check(self.v3, ts),
                        equiv_check(tv2_exp, tv3_exp)))

    def test_reduce_extend(self):
        # type: () -> None
        r = Rtl(
            self.v1 << uextend(self.v0),
            self.v2 << ireduce(self.v1),
            self.v3 << sextend(self.v2),
        )
        x = XForm(r, r)

        tv0_exp = 'Some({})'.format(self.v0.get_typevar().name)
        tv1_exp = 'Some({})'.format(self.v1.get_typevar().name)
        tv2_exp = 'Some({})'.format(self.v2.get_typevar().name)
        tv3_exp = 'Some({})'.format(self.v3.get_typevar().name)

        self.check_yo_check(
            x, sequence(wider_check(tv1_exp, tv0_exp),
                        wider_check(tv1_exp, tv2_exp),
                        wider_check(tv3_exp, tv2_exp)))

    def test_demote_promote(self):
        # type: () -> None
        r = Rtl(
            self.v1 << fpromote(self.v0),
            self.v2 << fdemote(self.v1),
            self.v3 << fpromote(self.v2),
        )
        x = XForm(r, r)

        tv0_exp = 'Some({})'.format(self.v0.get_typevar().name)
        tv1_exp = 'Some({})'.format(self.v1.get_typevar().name)
        tv2_exp = 'Some({})'.format(self.v2.get_typevar().name)
        tv3_exp = 'Some({})'.format(self.v3.get_typevar().name)

        self.check_yo_check(
            x, sequence(wider_check(tv1_exp, tv0_exp),
                        wider_check(tv1_exp, tv2_exp),
                        wider_check(tv3_exp, tv2_exp)))
