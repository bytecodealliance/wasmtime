"""
Type variables for Parametric polymorphism.

Cretonne instructions and instruction transformations can be specified to be
polymorphic by using type variables.
"""
from __future__ import absolute_import
from collections import namedtuple
from . import value

#: A `TypeSet` represents a set of types. We don't allow arbitrary subsets of
#: types, but use a parametrized approach instead.
#: This is represented as a named tuple so it can be used as a dictionary key.
TypeSet = namedtuple(
        'TypeSet', [
            'allow_scalars',
            'allow_simd',
            'base',
            'all_ints',
            'all_floats',
            'all_bools'])


class TypeVar(object):
    """
    Type variables can be used in place of concrete types when defining
    instructions. This makes the instructions *polymorphic*.

    A type variable is restricted to vary over a subset of the value types.
    This subset is specified by a set of flags that control the permitted base
    types and whether the type variable can assume scalar or vector types, or
    both.

    :param name: Short name of type variable used in instruction descriptions.
    :param doc: Documentation string.
    :param base: Single base type or list of base types. Use this to specify an
        exact set of base types if the general categories below are not good
        enough.
    :param ints: Allow all integer base types.
    :param floats: Allow all floating point base types.
    :param bools: Allow all boolean base types.
    :param scalars: Allow type variable to assume scalar types.
    :param simd: Allow type variable to assume vector types.
    """

    def __init__(
            self, name, doc, base=None,
            ints=False, floats=False, bools=False,
            scalars=True, simd=False,
            derived_func=None):
        self.name = name
        self.__doc__ = doc
        self.base = base
        self.is_derived = isinstance(base, TypeVar)
        if self.is_derived:
            assert derived_func
            self.derived_func = derived_func
            self.name = '{}({})'.format(derived_func, base.name)
        else:
            self.type_set = TypeSet(
                    allow_scalars=scalars,
                    allow_simd=simd,
                    base=base,
                    all_ints=ints,
                    all_floats=floats,
                    all_bools=bools)

    def __str__(self):
        return "`{}`".format(self.name)

    def lane_of(self):
        """
        Return a derived type variable that is the scalar lane type of this
        type variable.

        When this type variable assumes a scalar type, the derived type will be
        the same scalar type.
        """
        return TypeVar(None, None, base=self, derived_func='LaneOf')

    def as_bool(self):
        """
        Return a derived type variable that has the same vector geometry as
        this type variable, but with boolean lanes. Scalar types map to `b1`.
        """
        return TypeVar(None, None, base=self, derived_func='AsBool')

    def operand_kind(self):
        # When a `TypeVar` object is used to describe the type of an `Operand`
        # in an instruction definition, the kind of that operand is an SSA
        # value.
        return value

    def free_typevar(self):
        if isinstance(self.base, TypeVar):
            return self.base
        else:
            return self
