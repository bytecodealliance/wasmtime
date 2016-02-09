"""Predefined types."""

from . import ScalarType, IntType, FloatType

bool = ScalarType('bool', 0,
        """
        A boolean value that is either true or false.
        """)

i8  = IntType(8)
i16 = IntType(16)
i32 = IntType(32)
i64 = IntType(64)

f32 = FloatType(32,
        """
        A 32-bit floating point type represented in the IEEE 754-2008 *binary32*
        interchange format. This corresponds to the :c:type:`float` type in most
        C implementations.
        """)

f64 = FloatType(64,
        """
        A 64-bit floating point type represented in the IEEE 754-2008 *binary64*
        interchange format. This corresponds to the :c:type:`double` type in
        most C implementations.
        """)

i8x16 = i8.by(16)

f32x4 = f32.by(4)
f64x2 = f64.by(2)

