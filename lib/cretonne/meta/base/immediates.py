"""
The `cretonne.immediates` module predefines all the Cretonne immediate operand
types.
"""
from __future__ import absolute_import
from cdsl.operands import ImmediateKind

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

#: A condition code for comparing integer values.
#:
#: This enumerated operand kind is used for the :cton:inst:`icmp` instruction
#: and corresponds to the `condcodes::IntCC` Rust type.
intcc = ImmediateKind(
        'intcc',
        'An integer comparison condition code.',
        default_member='cond', rust_type='IntCC')

#: A condition code for comparing floating point values.
#:
#: This enumerated operand kind is used for the :cton:inst:`fcmp` instruction
#: and corresponds to the `condcodes::FloatCC` Rust type.
floatcc = ImmediateKind(
        'floatcc',
        'A floating point comparison condition code.',
        default_member='cond', rust_type='FloatCC')
