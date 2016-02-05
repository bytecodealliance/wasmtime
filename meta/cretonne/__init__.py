"""
Cretonne meta language module.

This module provides classes and functions used to describe Cretonne
instructions.
"""

# Concrete types.
#
# Instances (i8, i32, ...) are provided in the cretonne.types module.

class Type(object):
    """A concrete value type."""

    def __str__(self):
        return self.name

class ScalarType(Type):
    """
    A concrete scalar (not vector) type.

    Also tracks a unique set of :class:`VectorType` instances with this type as
    the lane type.
    """

    def __init__(self, name):
        self.name = name
        self._vectors = dict()

    def __repr__(self):
        return 'ScalarType({})'.format(self.name)

    def by(self, lanes):
        """
        Get a vector type with this type as the lane type.

        For example, ``i32.by(4)`` returns the :obj:`i32x4` type.
        """
        if lanes in self._vectors:
            return self._vectors[lanes]
        else:
            v = VectorType(self, lanes)
            self._vectors[lanes] = v
            return v

class VectorType(Type):
    """
    A concrete SIMD vector type.

    A vector type has a lane type which is an instance of :class:`ScalarType`,
    and a positive number of lanes.
    """

    def __init__(self, base, lanes):
        assert isinstance(base, ScalarType), 'SIMD lanes must be scalar types'
        self.base = base
        self.lanes = lanes
        self.name = '{}x{}'.format(base.name, lanes)

    def __repr__(self):
        return 'VectorType(base={}, lanes={})'.format(self.base.name, self.lanes)

class IntType(ScalarType):
    """A concrete scalar integer type."""

    def __init__(self, bits):
        assert bits > 0, 'IntType must have positive number of bits'
        super(IntType, self).__init__('i{:d}'.format(bits))
        self.bits = bits

    def __repr__(self):
        return 'IntType(bits={})'.format(self.bits)

class FloatType(ScalarType):
    """A concrete scalar floating point type."""

    def __init__(self, bits):
        assert bits > 0, 'FloatType must have positive number of bits'
        super(FloatType, self).__init__('f{:d}'.format(bits))
        self.bits = bits

    def __repr__(self):
        return 'FloatType(bits={})'.format(self.bits)
