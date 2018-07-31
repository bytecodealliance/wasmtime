"""
The `cranelift.immediates` module predefines all the Cranelift immediate
operand types.
"""
from __future__ import absolute_import
from cdsl.operands import ImmediateKind

#: A 64-bit immediate integer operand.
#:
#: This type of immediate integer can interact with SSA values with any
#: :py:class:`cranelift.IntType` type.
imm64 = ImmediateKind('imm64', 'A 64-bit immediate integer.')

#: An unsigned 8-bit immediate integer operand.
#:
#: This small operand is used to indicate lane indexes in SIMD vectors and
#: immediate bit counts on shift instructions.
uimm8 = ImmediateKind('uimm8', 'An 8-bit immediate unsigned integer.')

#: An unsigned 32-bit immediate integer operand.
uimm32 = ImmediateKind('uimm32', 'A 32-bit immediate unsigned integer.')

#: A 32-bit immediate signed offset.
#:
#: This is used to represent an immediate address offset in load/store
#: instructions.
offset32 = ImmediateKind(
        'offset32',
        'A 32-bit immediate signed offset.',
        default_member='offset')

#: A 32-bit immediate floating point operand.
#:
#: IEEE 754-2008 binary32 interchange format.
ieee32 = ImmediateKind('ieee32', 'A 32-bit immediate floating point number.')

#: A 64-bit immediate floating point operand.
#:
#: IEEE 754-2008 binary64 interchange format.
ieee64 = ImmediateKind('ieee64', 'A 64-bit immediate floating point number.')

#: An immediate boolean operand.
#:
#: This type of immediate boolean can interact with SSA values with any
#: :py:class:`cranelift.BoolType` type.
boolean = ImmediateKind('bool', 'An immediate boolean.',
                        rust_type='bool')

#: A condition code for comparing integer values.
#:
#: This enumerated operand kind is used for the :clif:inst:`icmp` instruction
#: and corresponds to the `condcodes::IntCC` Rust type.
intcc = ImmediateKind(
        'intcc',
        'An integer comparison condition code.',
        default_member='cond',
        rust_type='ir::condcodes::IntCC',
        values={
            'eq':  'Equal',
            'ne':  'NotEqual',
            'sge': 'SignedGreaterThanOrEqual',
            'sgt': 'SignedGreaterThan',
            'sle': 'SignedLessThanOrEqual',
            'slt': 'SignedLessThan',
            'uge': 'UnsignedGreaterThanOrEqual',
            'ugt': 'UnsignedGreaterThan',
            'ule': 'UnsignedLessThanOrEqual',
            'ult': 'UnsignedLessThan',
        })

#: A condition code for comparing floating point values.
#:
#: This enumerated operand kind is used for the :clif:inst:`fcmp` instruction
#: and corresponds to the `condcodes::FloatCC` Rust type.
floatcc = ImmediateKind(
        'floatcc',
        'A floating point comparison condition code.',
        default_member='cond',
        rust_type='ir::condcodes::FloatCC',
        values={
            'ord': 'Ordered',
            'uno': 'Unordered',
            'eq':  'Equal',
            'ne':  'NotEqual',
            'one': 'OrderedNotEqual',
            'ueq': 'UnorderedOrEqual',
            'lt':  'LessThan',
            'le':  'LessThanOrEqual',
            'gt':  'GreaterThan',
            'ge':  'GreaterThanOrEqual',
            'ult': 'UnorderedOrLessThan',
            'ule': 'UnorderedOrLessThanOrEqual',
            'ugt': 'UnorderedOrGreaterThan',
            'uge': 'UnorderedOrGreaterThanOrEqual',
        })

#: Flags for memory operations like :clif:inst:`load` and :clif:inst:`store`.
memflags = ImmediateKind(
        'memflags',
        'Memory operation flags',
        default_member='flags', rust_type='ir::MemFlags')

#: A register unit in the current target ISA.
regunit = ImmediateKind(
        'regunit',
        'A register unit in the target ISA',
        rust_type='isa::RegUnit')

#: A trap code indicating the reason for trapping.
#:
#: The Rust enum type also has a `User(u16)` variant for user-provided trap
#: codes.
trapcode = ImmediateKind(
        'trapcode',
        'A trap reason code.',
        default_member='code',
        rust_type='ir::TrapCode',
        values={
            "stk_ovf": 'StackOverflow',
            "heap_oob": 'HeapOutOfBounds',
            "int_ovf": 'IntegerOverflow',
            "int_divz": 'IntegerDivisionByZero',
        })
