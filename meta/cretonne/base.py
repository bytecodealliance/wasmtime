"""
Cretonne base instruction set.

This module defines the basic Cretonne instruction set that all targets support.
"""
from . import TypeVar, Operand, Instruction
from types import i8, f32, f64
from immediates import imm64, ieee32, ieee64, immvector

Int = TypeVar('Int', 'A scalar or vector integer type')
iB = TypeVar('iB', 'A scalar integer type')
TxN = TypeVar('%Tx%N', 'A SIMD vector type')

#
# Materializing constants.
#

N = Operand('N', imm64)
a = Operand('a', Int, doc='A constant integer scalar or vector value')
iconst = Instruction('iconst', r"""
    Integer constant.

    Create a scalar integer SSA value with an immediate constant value, or an
    integer vector where all the lanes have the same value.
    """,
    ins=N, outs=a)

N = Operand('N', ieee32)
a = Operand('a', f32, doc='A constant integer scalar or vector value')
f32const = Instruction('f32const', r"""
    Floating point constant.

    Create a :type:`f32` SSA value with an immediate constant value, or a
    floating point vector where all the lanes have the same value.
    """,
    ins=N, outs=a)

N = Operand('N', ieee64)
a = Operand('a', f64, doc='A constant integer scalar or vector value')
f64const = Instruction('f64const', r"""
    Floating point constant.

    Create a :type:`f64` SSA value with an immediate constant value, or a
    floating point vector where all the lanes have the same value.
    """,
    ins=N, outs=a)

N = Operand('N', immvector)
a = Operand('a', TxN, doc='A constant vector value')
vconst = Instruction('vconst', r"""
    Vector constant (floating point or integer).

    Create a SIMD vector value where the lanes don't have to be identical.
    """,
    ins=N, outs=a)

#
# Integer arithmetic
#

a = Operand('a', Int)
x = Operand('x', Int)
y = Operand('y', Int)

iadd = Instruction('iadd', r"""
    Wrapping integer addition: :math:`a := x + y \pmod{2^B}`.

    This instruction does not depend on the signed/unsigned interpretation of
    the operands.
    """,
    ins=(x,y), outs=a)

isub = Instruction('isub', r"""
    Wrapping integer subtraction: :math:`a := x - y \pmod{2^B}`.

    This instruction does not depend on the signed/unsigned interpretation of
    the operands.
    """,
    ins=(x,y), outs=a)

imul = Instruction('imul', r"""
    Wrapping integer multiplication: :math:`a := x y \pmod{2^B}`.

    This instruction does not depend on the signed/unsigned interpretation of
    the
    operands.

    Polymorphic over all integer types (vector and scalar).
    """,
    ins=(x,y), outs=a)

udiv = Instruction('udiv', r"""
    Unsigned integer division: :math:`a := \lfloor {x \over y} \rfloor`.

    This operation traps if the divisor is zero.
    """,
    ins=(x,y), outs=a)

sdiv = Instruction('sdiv', r"""
    Signed integer division rounded toward zero: :math:`a := sign(xy) \lfloor
    {|x| \over |y|}\rfloor`.

    This operation traps if the divisor is zero, or if the result is not
    representable in :math:`B` bits two's complement. This only happens when
    :math:`x = -2^{B-1}, y = -1`.
    """,
    ins=(x,y), outs=a)

urem = Instruction('urem', """
    Unsigned integer remainder.

    This operation traps if the divisor is zero.
    """,
    ins=(x,y), outs=a)

srem = Instruction('srem', """
    Signed integer remainder.

    This operation traps if the divisor is zero.

    .. todo:: Integer remainder vs modulus.

        Clarify whether the result has the sign of the divisor or the dividend.
        Should we add a ``smod`` instruction for the case where the result has
        the same sign as the divisor?
    """,
    ins=(x,y), outs=a)

a = Operand('a', iB)
x = Operand('x', iB)
Y = Operand('Y', imm64)

iadd_imm = Instruction('iadd_imm', """
    Add immediate integer.

    Same as :inst:`iadd`, but one operand is an immediate constant.

    Polymorphic over all scalar integer types, but does not support vector
    types.
    """,
    ins=(x,Y), outs=a)

imul_imm = Instruction('imul_imm', """
    Integer multiplication by immediate constant.

    Polymorphic over all scalar integer types.
    """,
    ins=(x,Y), outs=a)

udiv_imm = Instruction('udiv_imm', """
    Unsigned integer division by an immediate constant.

    This instruction never traps because a divisor of zero is not allowed.
    """,
    ins=(x,Y), outs=a)

sdiv_imm = Instruction('sdiv_imm', """
    Signed integer division by an immediate constant.

    This instruction never traps because a divisor of -1 or 0 is not allowed.
    """,
    ins=(x,Y), outs=a)

urem_imm = Instruction('urem_imm', """
    Unsigned integer remainder with immediate divisor.

    This instruction never traps because a divisor of zero is not allowed.
    """,
    ins=(x,Y), outs=a)

srem_imm = Instruction('srem_imm', """
    Signed integer remainder with immediate divisor.

    This instruction never traps because a divisor of 0 or -1 is not allowed.
    """,
    ins=(x,Y), outs=a)

# Swap x and y for isub_imm.
X = Operand('X', imm64)
y = Operand('y', iB)

isub_imm = Instruction('isub_imm', """
    Immediate wrapping subtraction: :math:`a := X - y \pmod{2^B}`.

    Also works as integer negation when :math:`X = 0`. Use :inst:`iadd_imm` with a
    negative immediate operand for the reverse immediate subtraction.

    Polymorphic over all scalar integer types, but does not support vector
    types.
    """,
    ins=(X,y), outs=a)

#
# Bitwise operations.
#

# TODO: Which types should permit boolean operations? Any reason to restrict?
bits = TypeVar('bits', 'Any integer, float, or boolean scalar or vector type')
x = Operand('x', bits)
y = Operand('y', bits)
a = Operand('a', bits)

band = Instruction('band', """
    Bitwise and.
    """,
    ins=(x,y), outs=a)

bor = Instruction('bor', """
    Bitwise or.
    """,
    ins=(x,y), outs=a)

bxor = Instruction('bxor', """
    Bitwise xor.
    """,
    ins=(x,y), outs=a)

bnot = Instruction('bnot', """
    Bitwise not.
    """,
    ins=x, outs=a)

# Shift/rotate.
x = Operand('x', Int, doc='Scalar or vector value to shift')
y = Operand('y', iB, doc='Number of bits to shift')
a = Operand('a', Int)

rotl = Instruction('rotl', r"""
    Rotate left.

    Rotate the bits in ``x`` by ``y`` places.
    """,
    ins=(x,y), outs=a)

rotr = Instruction('rotr', r"""
    Rotate right.

    Rotate the bits in ``x`` by ``y`` places.
    """,
    ins=(x,y), outs=a)

ishl = Instruction('ishl', r"""
    Integer shift left. Shift the bits in ``x`` towards the MSB by ``y``
    places. Shift in zero bits to the LSB.

    The shift amount is masked to the size of ``x``.

    When shifting a B-bits integer type, this instruction computes:

    .. math::
        s &:= y \pmod B,                \\
        a &:= x \cdot 2^s \pmod{2^B}.

    .. todo:: Add ``ishl_imm`` variant with an immediate ``y``.
    """,
    ins=(x,y), outs=a)

ushr = Instruction('ushr', r"""
    Unsigned shift right. Shift bits in ``x`` towards the LSB by ``y`` places,
    shifting in zero bits to the MSB. Also called a *logical shift*.

    The shift amount is masked to the size of the register.

    When shifting a B-bits integer type, this instruction computes:

    .. math::
        s &:= y \pmod B,                \\
        a &:= \lfloor x \cdot 2^{-s} \rfloor.

    .. todo:: Add ``ushr_imm`` variant with an immediate ``y``.
    """,
    ins=(x,y), outs=a)

sshr = Instruction('sshr', r"""
    Signed shift right. Shift bits in ``x`` towards the LSB by ``y`` places,
    shifting in sign bits to the MSB. Also called an *arithmetic shift*.

    The shift amount is masked to the size of the register.

    .. todo:: Add ``sshr_imm`` variant with an immediate ``y``.
    """,
    ins=(x,y), outs=a)

#
# Bit counting.
#

x = Operand('x', iB)
a = Operand('a', i8)

clz = Instruction('clz', r"""
    Count leading zero bits.

    Starting from the MSB in ``x``, count the number of zero bits before
    reaching the first one bit. When ``x`` is zero, returns the size of x in
    bits.
    """,
    ins=x, outs=a)

cls = Instruction('cls', r"""
    Count leading sign bits.

    Starting from the MSB after the sign bit in ``x``, count the number of
    consecutive bits identical to the sign bit. When ``x`` is 0 or -1, returns
    one less than the size of x in bits.
    """,
    ins=x, outs=a)

ctz = Instruction('ctz', r"""
    Count trailing zeros.

    Starting from the LSB in ``x``, count the number of zero bits before
    reaching the first one bit. When ``x`` is zero, returns the size of x in
    bits.
    """,
    ins=x, outs=a)

popcnt = Instruction('popcnt', r"""
    Population count

    Count the number of one bits in ``x``.
    """,
    ins=x, outs=a)
