"""
The base.types module predefines all the Cranelift scalar types.
"""
from __future__ import absolute_import
from cdsl.types import IntType, FloatType, BoolType, FlagsType

#: Abstract boolean (can't be stored in memory, use bint to convert to 0 or 1).
b1 = BoolType(1)    #: 1-bit bool.

#: Booleans used as SIMD elements (can be stored in memory, true is all-ones).
b8 = BoolType(8)    #: 8-bit bool.
b16 = BoolType(16)  #: 16-bit bool.
b32 = BoolType(32)  #: 32-bit bool.
b64 = BoolType(64)  #: 64-bit bool.

# Integers.
i8 = IntType(8)     #: 8-bit int.
i16 = IntType(16)   #: 16-bit int.
i32 = IntType(32)   #: 32-bit int.
i64 = IntType(64)   #: 64-bit int.

#: IEEE single precision.
f32 = FloatType(
        32, """
        A 32-bit floating point type represented in the IEEE 754-2008
        *binary32* interchange format. This corresponds to the :c:type:`float`
        type in most C implementations.
        """)

#: IEEE double precision.
f64 = FloatType(
        64, """
        A 64-bit floating point type represented in the IEEE 754-2008
        *binary64* interchange format. This corresponds to the :c:type:`double`
        type in most C implementations.
        """)
#: CPU flags from an integer comparison.
iflags = FlagsType(
        'iflags', """
        CPU flags representing the result of an integer comparison. These flags
        can be tested with an :type:`intcc` condition code.
        """)

#: CPU flags from a floating point comparison.
fflags = FlagsType(
        'fflags', """
        CPU flags representing the result of a floating point comparison. These
        flags can be tested with a :type:`floatcc` condition code.
        """)
