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
