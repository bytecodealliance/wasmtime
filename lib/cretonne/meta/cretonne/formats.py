"""
The cretonne.formats defines all instruction formats.

Every instruction format has a corresponding `InstructionData` variant in the
Rust representation of cretonne IL, so all instruction formats must be defined
in this module.
"""
from __future__ import absolute_import
from . import InstructionFormat, value, variable_args
from .immediates import imm64, uimm8, ieee32, ieee64, immvector, intcc, floatcc
from .entities import ebb, sig_ref, func_ref, jump_table

Nullary = InstructionFormat()

Unary = InstructionFormat(value)
UnaryImm = InstructionFormat(imm64)
UnaryIeee32 = InstructionFormat(ieee32)
UnaryIeee64 = InstructionFormat(ieee64)
UnaryImmVector = InstructionFormat(immvector, boxed_storage=True)
UnarySplit = InstructionFormat(value, multiple_results=True)

Binary = InstructionFormat(value, value)
BinaryImm = InstructionFormat(value, imm64)
BinaryImmRev = InstructionFormat(imm64, value)

# Generate result + overflow flag.
BinaryOverflow = InstructionFormat(value, value, multiple_results=True)

# The select instructions are controlled by the second value operand.
# The first value operand is the controlling flag which has a derived type.
# The fma instruction has the same constraint on all inputs.
Ternary = InstructionFormat(value, value, value, typevar_operand=1)

# Carry in *and* carry out for `iadd_carry` and friends.
TernaryOverflow = InstructionFormat(
        value, value, value, multiple_results=True, boxed_storage=True)

InsertLane = InstructionFormat(value, ('lane', uimm8), value)
ExtractLane = InstructionFormat(value, ('lane', uimm8))

IntCompare = InstructionFormat(intcc, value, value)
FloatCompare = InstructionFormat(floatcc, value, value)

Jump = InstructionFormat(ebb, variable_args, boxed_storage=True)
Branch = InstructionFormat(value, ebb, variable_args, boxed_storage=True)
BranchTable = InstructionFormat(value, jump_table)

Call = InstructionFormat(
        func_ref, variable_args, multiple_results=True, boxed_storage=True)
IndirectCall = InstructionFormat(
        sig_ref, value, variable_args,
        multiple_results=True, boxed_storage=True)
Return = InstructionFormat(variable_args, boxed_storage=True)


# Finally extract the names of global variables in this module.
InstructionFormat.extract_names(globals())
