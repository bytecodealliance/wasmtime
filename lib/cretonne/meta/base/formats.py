"""
The cretonne.formats defines all instruction formats.

Every instruction format has a corresponding `InstructionData` variant in the
Rust representation of cretonne IL, so all instruction formats must be defined
in this module.
"""
from __future__ import absolute_import
from cdsl.formats import InstructionFormat
from cdsl.operands import VALUE, VARIABLE_ARGS
from .immediates import imm64, uimm8, ieee32, ieee64, immvector, intcc, floatcc
from .entities import ebb, sig_ref, func_ref, jump_table

Nullary = InstructionFormat()

Unary = InstructionFormat(VALUE)
UnaryImm = InstructionFormat(imm64)
UnaryIeee32 = InstructionFormat(ieee32)
UnaryIeee64 = InstructionFormat(ieee64)
UnaryImmVector = InstructionFormat(immvector, boxed_storage=True)
UnarySplit = InstructionFormat(VALUE, multiple_results=True)

Binary = InstructionFormat(VALUE, VALUE)
BinaryImm = InstructionFormat(VALUE, imm64)

# Generate result + overflow flag.
BinaryOverflow = InstructionFormat(VALUE, VALUE, multiple_results=True)

# The select instructions are controlled by the second VALUE operand.
# The first VALUE operand is the controlling flag which has a derived type.
# The fma instruction has the same constraint on all inputs.
Ternary = InstructionFormat(VALUE, VALUE, VALUE, typevar_operand=1)

# Carry in *and* carry out for `iadd_carry` and friends.
TernaryOverflow = InstructionFormat(
        VALUE, VALUE, VALUE, multiple_results=True, boxed_storage=True)

InsertLane = InstructionFormat(VALUE, ('lane', uimm8), VALUE)
ExtractLane = InstructionFormat(VALUE, ('lane', uimm8))

IntCompare = InstructionFormat(intcc, VALUE, VALUE)
FloatCompare = InstructionFormat(floatcc, VALUE, VALUE)

Jump = InstructionFormat(ebb, VARIABLE_ARGS, value_list=True)
Branch = InstructionFormat(VALUE, ebb, VARIABLE_ARGS, value_list=True)
BranchTable = InstructionFormat(VALUE, jump_table)

Call = InstructionFormat(
        func_ref, VARIABLE_ARGS, multiple_results=True, value_list=True)
IndirectCall = InstructionFormat(
        sig_ref, VALUE, VARIABLE_ARGS,
        multiple_results=True, value_list=True)
Return = InstructionFormat(VARIABLE_ARGS, value_list=True)
ReturnReg = InstructionFormat(VALUE, VARIABLE_ARGS, value_list=True)

# Finally extract the names of global variables in this module.
InstructionFormat.extract_names(globals())
