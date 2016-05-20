"""
The `cretonne.immediates` module predefines all the Cretonne immediate operand
types.
"""

from . import ImmediateKind

#: A 64-bit immediate integer operand.
#:
#: This type of immediate integer can interact with SSA values with any
#: :py:class:`cretonne.IntType` type.
imm64 = ImmediateKind('imm64', 'A 64-bit immediate integer.')

#: An unsigned 8-bit immediate integer operand.
#:
#: This small operand is used to indicate lane indexes in SIMD vectors and
#: immediate bit counts on shift instructions.
uimm8 = ImmediateKind('uimm8', 'An 8-bit immediate unsigned integer.')

#: A 32-bit immediate floating point operand.
#:
#: IEEE 754-2008 binary32 interchange format.
ieee32 = ImmediateKind('ieee32', 'A 32-bit immediate floating point number.')

#: A 64-bit immediate floating point operand.
#:
#: IEEE 754-2008 binary64 interchange format.
ieee64 = ImmediateKind('ieee64', 'A 64-bit immediate floating point number.')

#: A large SIMD vector constant.
immvector = ImmediateKind('immvector', 'An immediate SIMD vector.')
