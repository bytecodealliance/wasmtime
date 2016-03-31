"""
The cretonne.immdiates module predefines all the Cretonne immediate operand
types.
"""

from . import ImmediateType

#: A 64-bit immediate integer operand.
#:
#: This type of immediate integer can interact with SSA values with any
#: :py:class:`cretonne.IntType` type.
imm64 = ImmediateType('imm64', 'A 64-bit immediate integer.')

#: A 32-bit immediate floating point operand.
#:
#: IEEE 754-2008 binary32 interchange format.
ieee32 = ImmediateType('ieee32', 'A 32-bit immediate floating point number.')

#: A 64-bit immediate floating point operand.
#:
#: IEEE 754-2008 binary64 interchange format.
ieee64 = ImmediateType('ieee64', 'A 64-bit immediate floating point number.')

#: A large SIMD vector constant.
immvector = ImmediateType('immvector', 'An immediate SIMD vector.')
