"""
The cretonne.formats defines all instruction formats.

Every instruction format has a corresponding `InstructionData` variant in the
Rust representation of cretonne IL, so all instruction formats must be defined
in this module.
"""
from __future__ import absolute_import
from cdsl.formats import InstructionFormat
from cdsl.operands import VALUE, VARIABLE_ARGS
from .immediates import imm64, uimm8, ieee32, ieee64, offset32, uoffset32
from .immediates import intcc, floatcc
from .entities import ebb, sig_ref, func_ref, jump_table, stack_slot

Nullary = InstructionFormat()

Unary = InstructionFormat(VALUE)
UnaryImm = InstructionFormat(imm64)
UnaryIeee32 = InstructionFormat(ieee32)
UnaryIeee64 = InstructionFormat(ieee64)
UnarySplit = InstructionFormat(VALUE, multiple_results=True)

Binary = InstructionFormat(VALUE, VALUE)
BinaryImm = InstructionFormat(VALUE, imm64)

# Generate result + overflow flag.
BinaryOverflow = InstructionFormat(VALUE, VALUE, multiple_results=True)

# The select instructions are controlled by the second VALUE operand.
# The first VALUE operand is the controlling flag which has a derived type.
# The fma instruction has the same constraint on all inputs.
Ternary = InstructionFormat(VALUE, VALUE, VALUE, typevar_operand=1)

# Catch-all for instructions with many outputs and inputs and no immediate
# operands.
MultiAry = InstructionFormat(VARIABLE_ARGS, multiple_results=True)

InsertLane = InstructionFormat(VALUE, ('lane', uimm8), VALUE)
ExtractLane = InstructionFormat(VALUE, ('lane', uimm8))

IntCompare = InstructionFormat(intcc, VALUE, VALUE)
IntCompareImm = InstructionFormat(intcc, VALUE, imm64)
FloatCompare = InstructionFormat(floatcc, VALUE, VALUE)

Jump = InstructionFormat(ebb, VARIABLE_ARGS)
Branch = InstructionFormat(VALUE, ebb, VARIABLE_ARGS)
BranchIcmp = InstructionFormat(intcc, VALUE, VALUE, ebb, VARIABLE_ARGS)
BranchTable = InstructionFormat(VALUE, jump_table)

Call = InstructionFormat(
        func_ref, VARIABLE_ARGS, multiple_results=True)
IndirectCall = InstructionFormat(
        sig_ref, VALUE, VARIABLE_ARGS,
        multiple_results=True)

StackLoad = InstructionFormat(stack_slot, offset32)
StackStore = InstructionFormat(VALUE, stack_slot, offset32)

# Accessing a WebAssembly heap.
# TODO: Add a reference to a `heap` declared in the preamble.
HeapLoad = InstructionFormat(VALUE, uoffset32)
HeapStore = InstructionFormat(VALUE, VALUE, uoffset32)

# Finally extract the names of global variables in this module.
InstructionFormat.extract_names(globals())
