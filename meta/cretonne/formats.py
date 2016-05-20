"""
The cretonne.formats defines all instruction formats.

Every instruction format has a corresponding `InstructionData` variant in the
Rust representation of cretonne IL, so all instruction formats must be defined
in this module.
"""


from . import InstructionFormat, value, variable_args
from immediates import imm64, ieee32, ieee64, immvector
from entities import ebb, function, jump_table

Nullary = InstructionFormat()

Unary = InstructionFormat(value)
UnaryImm = InstructionFormat(imm64)
UnaryIeee32 = InstructionFormat(ieee32)
UnaryIeee64 = InstructionFormat(ieee64)
UnaryImmVector = InstructionFormat(immvector)

Binary = InstructionFormat(value, value)
BinaryImm = InstructionFormat(value, imm64)
BinaryImmRev = InstructionFormat(imm64, value)

# Generate result + overflow flag.
BinaryOverflow = InstructionFormat(value, value, multiple_results=True)

# The select instructions are controlled by the second value operand.
# The first value operand is the controlling flag whisch has a derived type.
Select = InstructionFormat(value, value, value, typevar_operand=1)

Jump = InstructionFormat(ebb, variable_args, boxed_storage=True)
Branch = InstructionFormat(value, ebb, variable_args, boxed_storage=True)
BranchTable = InstructionFormat(value, jump_table)

Call = InstructionFormat(
        function, variable_args, multiple_results=True, boxed_storage=True)

# Finally extract the names of global variables in this module.
InstructionFormat.extract_names(globals())
