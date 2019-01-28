"""
Supplementary instruction definitions for x86.

This module defines additional instructions that are useful only to the x86
target ISA.
"""

from base.types import iflags
from cdsl.operands import Operand
from cdsl.typevar import TypeVar
from cdsl.instructions import Instruction, InstructionGroup


GROUP = InstructionGroup("x86", "x86-specific instruction set")

iWord = TypeVar('iWord', 'A scalar integer machine word', ints=(32, 64))

nlo = Operand('nlo', iWord, doc='Low part of numerator')
nhi = Operand('nhi', iWord, doc='High part of numerator')
d = Operand('d', iWord, doc='Denominator')
q = Operand('q', iWord, doc='Quotient')
r = Operand('r', iWord, doc='Remainder')

udivmodx = Instruction(
        'x86_udivmodx', r"""
        Extended unsigned division.

        Concatenate the bits in `nhi` and `nlo` to form the numerator.
        Interpret the bits as an unsigned number and divide by the unsigned
        denominator `d`. Trap when `d` is zero or if the quotient is larger
        than the range of the output.

        Return both quotient and remainder.
        """,
        ins=(nlo, nhi, d), outs=(q, r), can_trap=True)

sdivmodx = Instruction(
        'x86_sdivmodx', r"""
        Extended signed division.

        Concatenate the bits in `nhi` and `nlo` to form the numerator.
        Interpret the bits as a signed number and divide by the signed
        denominator `d`. Trap when `d` is zero or if the quotient is outside
        the range of the output.

        Return both quotient and remainder.
        """,
        ins=(nlo, nhi, d), outs=(q, r), can_trap=True)

argL = Operand('argL', iWord)
argR = Operand('argR', iWord)
resLo = Operand('resLo', iWord)
resHi = Operand('resHi', iWord)

umulx = Instruction(
        'x86_umulx', r"""
        Unsigned integer multiplication, producing a double-length result.

        Polymorphic over all scalar integer types, but does not support vector
        types.
        """,
        ins=(argL, argR), outs=(resLo, resHi))

smulx = Instruction(
        'x86_smulx', r"""
        Signed integer multiplication, producing a double-length result.

        Polymorphic over all scalar integer types, but does not support vector
        types.
        """,
        ins=(argL, argR), outs=(resLo, resHi))

Float = TypeVar(
        'Float', 'A scalar or vector floating point number',
        floats=True, simd=True)
IntTo = TypeVar(
        'IntTo', 'An integer type with the same number of lanes',
        ints=(32, 64), simd=True)

x = Operand('x', Float)
a = Operand('a', IntTo)

cvtt2si = Instruction(
        'x86_cvtt2si', r"""
        Convert with truncation floating point to signed integer.

        The source floating point operand is converted to a signed integer by
        rounding towards zero. If the result can't be represented in the output
        type, returns the smallest signed value the output type can represent.

        This instruction does not trap.
        """,
        ins=x, outs=a)

x = Operand('x', Float)
a = Operand('a', Float)
y = Operand('y', Float)

fmin = Instruction(
        'x86_fmin', r"""
        Floating point minimum with x86 semantics.

        This is equivalent to the C ternary operator `x < y ? x : y` which
        differs from :inst:`fmin` when either operand is NaN or when comparing
        +0.0 to -0.0.

        When the two operands don't compare as LT, `y` is returned unchanged,
        even if it is a signalling NaN.
        """,
        ins=(x, y), outs=a)

fmax = Instruction(
        'x86_fmax', r"""
        Floating point maximum with x86 semantics.

        This is equivalent to the C ternary operator `x > y ? x : y` which
        differs from :inst:`fmax` when either operand is NaN or when comparing
        +0.0 to -0.0.

        When the two operands don't compare as GT, `y` is returned unchanged,
        even if it is a signalling NaN.
        """,
        ins=(x, y), outs=a)


x = Operand('x', iWord)

push = Instruction(
    'x86_push', r"""
    Pushes a value onto the stack.

    Decrements the stack pointer and stores the specified value on to the top.

    This is polymorphic in i32 and i64. However, it is only implemented for i64
    in 64-bit mode, and only for i32 in 32-bit mode.
    """,
    ins=x, can_store=True, other_side_effects=True)

pop = Instruction(
    'x86_pop', r"""
    Pops a value from the stack.

    Loads a value from the top of the stack and then increments the stack
    pointer.

    This is polymorphic in i32 and i64. However, it is only implemented for i64
    in 64-bit mode, and only for i32 in 32-bit mode.
    """,
    outs=x, can_load=True, other_side_effects=True)

y = Operand('y', iWord)
rflags = Operand('rflags', iflags)

bsr = Instruction(
    'x86_bsr', r"""
    Bit Scan Reverse -- returns the bit-index of the most significant 1
    in the word. Result is undefined if the argument is zero. However, it
    sets the Z flag depending on the argument, so it is at least easy to
    detect and handle that case.

    This is polymorphic in i32 and i64. It is implemented for both i64 and
    i32 in 64-bit mode, and only for i32 in 32-bit mode.
    """,
    ins=x, outs=(y, rflags))

bsf = Instruction(
    'x86_bsf', r"""
    Bit Scan Forwards -- returns the bit-index of the least significant 1
    in the word. Is otherwise identical to 'bsr', just above.
    """,
    ins=x, outs=(y, rflags))

GROUP.close()
