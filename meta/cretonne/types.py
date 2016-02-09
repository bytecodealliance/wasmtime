"""
The cretonne.types module predefines all the Cretonne scalar types.
"""

from . import ScalarType, IntType, FloatType

#: Boolean.
bool = ScalarType('bool', 0,
        """
        A boolean value that is either true or false.
        """)

i8  = IntType(8)  #: 8-bit int.
i16 = IntType(16) #: 16-bit int.
i32 = IntType(32) #: 32-bit int.
i64 = IntType(64) #: 64-bit int.

#: IEEE single precision.
f32 = FloatType(32,
        """
        A 32-bit floating point type represented in the IEEE 754-2008 *binary32*
        interchange format. This corresponds to the :c:type:`float` type in most
        C implementations.
        """)

#: IEEE double precision.
f64 = FloatType(64,
        """
        A 64-bit floating point type represented in the IEEE 754-2008 *binary64*
        interchange format. This corresponds to the :c:type:`double` type in
        most C implementations.
        """)
