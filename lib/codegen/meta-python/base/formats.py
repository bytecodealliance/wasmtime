"""
The cranelift.formats defines all instruction formats.

Every instruction format has a corresponding `InstructionData` variant in the
Rust representation of Cranelift IR, so all instruction formats must be defined
in this module.
"""
from __future__ import absolute_import
from cdsl.formats import InstructionFormat
from cdsl.operands import VALUE, VARIABLE_ARGS
from .immediates import imm64, uimm8, uimm32, ieee32, ieee64, offset32
from .immediates import boolean, intcc, floatcc, memflags, regunit, trapcode
from . import entities
from .entities import ebb, sig_ref, func_ref, stack_slot, heap, table

Unary = InstructionFormat(VALUE)
UnaryImm = InstructionFormat(imm64)
UnaryIeee32 = InstructionFormat(ieee32)
UnaryIeee64 = InstructionFormat(ieee64)
UnaryBool = InstructionFormat(boolean)
UnaryGlobalValue = InstructionFormat(entities.global_value)

Binary = InstructionFormat(VALUE, VALUE)
BinaryImm = InstructionFormat(VALUE, imm64)

# The select instructions are controlled by the second VALUE operand.
# The first VALUE operand is the controlling flag which has a derived type.
# The fma instruction has the same constraint on all inputs.
Ternary = InstructionFormat(VALUE, VALUE, VALUE, typevar_operand=1)

# Catch-all for instructions with many outputs and inputs and no immediate
# operands.
MultiAry = InstructionFormat(VARIABLE_ARGS)

NullAry = InstructionFormat()

InsertLane = InstructionFormat(VALUE, ('lane', uimm8), VALUE)
ExtractLane = InstructionFormat(VALUE, ('lane', uimm8))

IntCompare = InstructionFormat(intcc, VALUE, VALUE)
IntCompareImm = InstructionFormat(intcc, VALUE, imm64)
IntCond = InstructionFormat(intcc, VALUE)
FloatCompare = InstructionFormat(floatcc, VALUE, VALUE)
FloatCond = InstructionFormat(floatcc, VALUE)

IntSelect = InstructionFormat(intcc, VALUE, VALUE, VALUE)

Jump = InstructionFormat(ebb, VARIABLE_ARGS)
Branch = InstructionFormat(VALUE, ebb, VARIABLE_ARGS)
BranchInt = InstructionFormat(intcc, VALUE, ebb, VARIABLE_ARGS)
BranchFloat = InstructionFormat(floatcc, VALUE, ebb, VARIABLE_ARGS)
BranchIcmp = InstructionFormat(intcc, VALUE, VALUE, ebb, VARIABLE_ARGS)
BranchTable = InstructionFormat(VALUE, ebb, entities.jump_table)
BranchTableEntry = InstructionFormat(VALUE, VALUE, uimm8, entities.jump_table)
BranchTableBase = InstructionFormat(entities.jump_table)
IndirectJump = InstructionFormat(VALUE, entities.jump_table)

Call = InstructionFormat(func_ref, VARIABLE_ARGS)
CallIndirect = InstructionFormat(sig_ref, VALUE, VARIABLE_ARGS)
FuncAddr = InstructionFormat(func_ref)

Load = InstructionFormat(memflags, VALUE, offset32)
LoadComplex = InstructionFormat(memflags, VARIABLE_ARGS, offset32)
Store = InstructionFormat(memflags, VALUE, VALUE, offset32)
StoreComplex = InstructionFormat(memflags, VALUE, VARIABLE_ARGS, offset32)

StackLoad = InstructionFormat(stack_slot, offset32)
StackStore = InstructionFormat(VALUE, stack_slot, offset32)

# Accessing a WebAssembly heap.
HeapAddr = InstructionFormat(heap, VALUE, uimm32)

# Accessing a WebAssembly table.
TableAddr = InstructionFormat(table, VALUE, offset32)

RegMove = InstructionFormat(VALUE, ('src', regunit), ('dst', regunit))
CopySpecial = InstructionFormat(('src', regunit), ('dst', regunit))
RegSpill = InstructionFormat(
        VALUE, ('src', regunit), ('dst', entities.stack_slot))
RegFill = InstructionFormat(
        VALUE, ('src', entities.stack_slot), ('dst', regunit))

Trap = InstructionFormat(trapcode)
CondTrap = InstructionFormat(VALUE, trapcode)
IntCondTrap = InstructionFormat(intcc, VALUE, trapcode)
FloatCondTrap = InstructionFormat(floatcc, VALUE, trapcode)

# Finally extract the names of global values in this module.
InstructionFormat.extract_names(globals())
