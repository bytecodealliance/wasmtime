"""
The cretonne.formats defines all instruction formats.

Every instruction format has a corresponding `InstructionData` variant in the
Rust representation of cretonne IL, so all instruction formats must be defined
in this module.
"""


from . import InstructionFormat, value
from immediates import imm64, ieee32, ieee64, immvector

Unary = InstructionFormat(value)
UnaryImm = InstructionFormat(imm64)
UnaryIeee32 = InstructionFormat(ieee32)
UnaryIeee64 = InstructionFormat(ieee64)
UnaryImmVector = InstructionFormat(immvector)

Binary = InstructionFormat(value, value)
BinaryImm = InstructionFormat(value, imm64)
BinaryImmRev = InstructionFormat(imm64, value)


# Finally extract the names of global variables in this module.
InstructionFormat.extract_names(globals())
