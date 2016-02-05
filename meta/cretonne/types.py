"""Predefined types."""

from . import ScalarType, IntType, FloatType

#: A boolean value.
bool = ScalarType('bool')

i8  = IntType(8)  #: 8-bit int.
i16 = IntType(16) #: 16-bit int.
i32 = IntType(32) #: 32-bit int.
i64 = IntType(64) #: 64-bit int.

f32 = FloatType(32) #: IEEE 32-bit float.
f64 = FloatType(64) #: IEEE 64-bit float

i8x16 = i8.by(16) #: Vector of 16 i8 lanes.

f32x4 = f32.by(4) #: Vector of 4 f32 lanes.
f64x2 = f64.by(2) #: Vector of 2 f64 lanes.

