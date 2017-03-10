"""
Cretonne base instruction set.

This module defines the basic Cretonne instruction set that all targets
support.
"""
from __future__ import absolute_import
from cdsl.operands import Operand, VARIABLE_ARGS
from cdsl.typevar import TypeVar
from cdsl.instructions import Instruction, InstructionGroup
from base.types import i8, f32, f64, b1
from base.immediates import imm64, uimm8, ieee32, ieee64, immvector
from base.immediates import intcc, floatcc
from base import entities
import base.formats  # noqa

GROUP = InstructionGroup("base", "Shared base instruction set")

Int = TypeVar('Int', 'A scalar or vector integer type', ints=True, simd=True)
iB = TypeVar('iB', 'A scalar integer type', ints=True)
iAddr = TypeVar('iAddr', 'An integer address type', ints=(32, 64))
Testable = TypeVar(
        'Testable', 'A scalar boolean or integer type',
        ints=True, bools=True)
TxN = TypeVar(
        'TxN', 'A SIMD vector type',
        ints=True, floats=True, bools=True, scalars=False, simd=True)
Any = TypeVar(
        'Any', 'Any integer, float, or boolean scalar or vector type',
        ints=True, floats=True, bools=True, scalars=True, simd=True)

#
# Control flow
#
c = Operand('c', Testable, doc='Controlling value to test')
EBB = Operand('EBB', entities.ebb, doc='Destination extended basic block')
args = Operand('args', VARIABLE_ARGS, doc='EBB arguments')

jump = Instruction(
        'jump', r"""
        Jump.

        Unconditionally jump to an extended basic block, passing the specified
        EBB arguments. The number and types of arguments must match the
        destination EBB.
        """,
        ins=(EBB, args), is_terminator=True)

brz = Instruction(
        'brz', r"""
        Branch when zero.

        If ``c`` is a :type:`b1` value, take the branch when ``c`` is false. If
        ``c`` is an integer value, take the branch when ``c = 0``.
        """,
        ins=(c, EBB, args), is_branch=True)

brnz = Instruction(
        'brnz', r"""
        Branch when non-zero.

        If ``c`` is a :type:`b1` value, take the branch when ``c`` is true. If
        ``c`` is an integer value, take the branch when ``c != 0``.
        """,
        ins=(c, EBB, args), is_branch=True)

x = Operand('x', iB, doc='index into jump table')
JT = Operand('JT', entities.jump_table)
br_table = Instruction(
        'br_table', r"""
        Indirect branch via jump table.

        Use ``x`` as an unsigned index into the jump table ``JT``. If a jump
        table entry is found, branch to the corresponding EBB. If no entry was
        found fall through to the next instruction.

        Note that this branch instruction can't pass arguments to the targeted
        blocks. Split critical edges as needed to work around this.
        """,
        ins=(x, JT), is_branch=True)

trap = Instruction(
        'trap', r"""
        Terminate execution unconditionally.
        """,
        is_terminator=True, can_trap=True)

trapz = Instruction(
        'trapz', r"""
        Trap when zero.

        if ``c`` is non-zero, execution continues at the following instruction.
        """,
        ins=c, can_trap=True)

trapnz = Instruction(
        'trapnz', r"""
        Trap when non-zero.

        if ``c`` is zero, execution continues at the following instruction.
        """,
        ins=c, can_trap=True)

rvals = Operand('rvals', VARIABLE_ARGS, doc='return values')

x_return = Instruction(
        'return', r"""
        Return from the function.

        Unconditionally transfer control to the calling function, passing the
        provided return values. The list of return values must match the
        function signature's return types.
        """,
        ins=rvals, is_return=True, is_terminator=True)

raddr = Operand('raddr', iAddr, doc='Return address')

return_reg = Instruction(
        'return_reg', r"""
        Return from the function to a return address held in a register.

        Unconditionally transfer control to the calling function, passing the
        provided return values. The list of return values must match the
        function signature's return types.

        This instruction should only be used by ISA-specific epilogue lowering
        code. It is equivalent to :inst:`return`, but the return address is
        provided explicitly in a register. This style of return instruction is
        used by RISC architectures such as ARM and RISC-V. A normal
        :inst:`return` will be legalized into this instruction on these
        architectures.
        """,
        ins=(raddr, rvals), is_return=True, is_terminator=True)

FN = Operand(
        'FN',
        entities.func_ref,
        doc='function to call, declared by :inst:`function`')
args = Operand('args', VARIABLE_ARGS, doc='call arguments')

call = Instruction(
        'call', r"""
        Direct function call.

        Call a function which has been declared in the preamble. The argument
        types must match the function's signature.
        """,
        ins=(FN, args), outs=rvals, is_call=True)

SIG = Operand('SIG', entities.sig_ref, doc='function signature')
callee = Operand('callee', iAddr, doc='address of function to call')

call_indirect = Instruction(
        'call_indirect', r"""
        Indirect function call.

        Call the function pointed to by `callee` with the given arguments. The
        called function must match the specified signature.
        """,
        ins=(SIG, callee, args), outs=rvals, is_call=True)

#
# Materializing constants.
#

N = Operand('N', imm64)
a = Operand('a', Int, doc='A constant integer scalar or vector value')
iconst = Instruction(
        'iconst', r"""
        Integer constant.

        Create a scalar integer SSA value with an immediate constant value, or
        an integer vector where all the lanes have the same value.
        """,
        ins=N, outs=a)

N = Operand('N', ieee32)
a = Operand('a', f32, doc='A constant integer scalar or vector value')
f32const = Instruction(
        'f32const', r"""
        Floating point constant.

        Create a :type:`f32` SSA value with an immediate constant value, or a
        floating point vector where all the lanes have the same value.
        """,
        ins=N, outs=a)

N = Operand('N', ieee64)
a = Operand('a', f64, doc='A constant integer scalar or vector value')
f64const = Instruction(
        'f64const', r"""
        Floating point constant.

        Create a :type:`f64` SSA value with an immediate constant value, or a
        floating point vector where all the lanes have the same value.
        """,
        ins=N, outs=a)

N = Operand('N', immvector)
a = Operand('a', TxN, doc='A constant vector value')
vconst = Instruction(
        'vconst', r"""
        Vector constant (floating point or integer).

        Create a SIMD vector value where the lanes don't have to be identical.
        """,
        ins=N, outs=a)

#
# Generics.
#

c = Operand('c', Testable, doc='Controlling value to test')
x = Operand('x', Any, doc='Value to use when `c` is true')
y = Operand('y', Any, doc='Value to use when `c` is false')
a = Operand('a', Any)

select = Instruction(
        'select', r"""
        Conditional select.

        This instruction selects whole values. Use :inst:`vselect` for
        lane-wise selection.
        """,
        ins=(c, x, y), outs=a)

x = Operand('x', Any)

copy = Instruction(
        'copy', r"""
        Register-register copy.

        This instruction copies its input, preserving the value type.

        A pure SSA-form program does not need to copy values, but this
        instruction is useful for representing intermediate stages during
        instruction transformations, and the register allocator needs a way of
        representing register copies.
        """,
        ins=x, outs=a)

spill = Instruction(
        'spill', r"""
        Spill a register value to a stack slot.

        This instruction behaves exactly like :inst:`copy`, but the result
        value is assigned to a spill slot.
        """,
        ins=x, outs=a)

fill = Instruction(
        'fill', r"""
        Load a register value from a stack slot.

        This instruction behaves exactly like :inst:`copy`, but creates a new
        SSA value for the spilled input value.
        """,
        ins=x, outs=a)


#
# Vector operations
#

x = Operand('x', TxN, doc='Vector to split')
lo = Operand('lo', TxN.half_vector(), doc='Low-numbered lanes of `x`')
hi = Operand('hi', TxN.half_vector(), doc='High-numbered lanes of `x`')

vsplit = Instruction(
        'vsplit', r"""
        Split a vector into two halves.

        Split the vector `x` into two separate values, each containing half of
        the lanes from ``x``. The result may be two scalars if ``x`` only had
        two lanes.
        """,
        ins=x, outs=(lo, hi))

Any128 = TypeVar(
        'Any128', 'Any scalar or vector type with as most 128 lanes',
        ints=True, floats=True, bools=True, scalars=True, simd=(1, 128))
x = Operand('x', Any128, doc='Low-numbered lanes')
y = Operand('y', Any128, doc='High-numbered lanes')
a = Operand('a', Any128.double_vector(), doc='Concatenation of `x` and `y`')

vconcat = Instruction(
        'vconcat', r"""
        Vector concatenation.

        Return a vector formed by concatenating ``x`` and ``y``. The resulting
        vector type has twice as many lanes as each of the inputs. The lanes of
        ``x`` appear as the low-numbered lanes, and the lanes of ``y`` become
        the high-numbered lanes of ``a``.

        It is possible to form a vector by concatenating two scalars.
        """,
        ins=(x, y), outs=a)

c = Operand('c', TxN.as_bool(), doc='Controlling vector')
x = Operand('x', TxN, doc='Value to use where `c` is true')
y = Operand('y', TxN, doc='Value to use where `c` is false')
a = Operand('a', TxN)

vselect = Instruction(
        'vselect', r"""
        Vector lane select.

        Select lanes from ``x`` or ``y`` controlled by the lanes of the boolean
        vector ``c``.
        """,
        ins=(c, x, y), outs=a)

x = Operand('x', TxN.lane_of())

splat = Instruction(
        'splat', r"""
        Vector splat.

        Return a vector whose lanes are all ``x``.
        """,
        ins=x, outs=a)

x = Operand('x', TxN, doc='SIMD vector to modify')
y = Operand('y', TxN.lane_of(), doc='New lane value')
Idx = Operand('Idx', uimm8, doc='Lane index')

insertlane = Instruction(
        'insertlane', r"""
        Insert ``y`` as lane ``Idx`` in x.

        The lane index, ``Idx``, is an immediate value, not an SSA value. It
        must indicate a valid lane index for the type of ``x``.
        """,
        ins=(x, Idx, y), outs=a)

x = Operand('x', TxN)
a = Operand('a', TxN.lane_of())

extractlane = Instruction(
        'extractlane', r"""
        Extract lane ``Idx`` from ``x``.

        The lane index, ``Idx``, is an immediate value, not an SSA value. It
        must indicate a valid lane index for the type of ``x``.
        """,
        ins=(x, Idx), outs=a)

#
# Integer arithmetic
#

a = Operand('a', Int.as_bool())
Cond = Operand('Cond', intcc)
x = Operand('x', Int)
y = Operand('y', Int)

icmp = Instruction(
        'icmp', r"""
        Integer comparison.

        The condition code determines if the operands are interpreted as signed
        or unsigned integers.

        ====== ======== =========
        Signed Unsigned Condition
        ====== ======== =========
        eq     eq       Equal
        ne     ne       Not equal
        slt    ult      Less than
        sge    uge      Greater than or equal
        sgt    ugt      Greater than
        sle    ule      Less than or equal
        ====== ======== =========

        When this instruction compares integer vectors, it returns a boolean
        vector of lane-wise comparisons.
        """,
        ins=(Cond, x, y), outs=a)

a = Operand('a', Int)
x = Operand('x', Int)
y = Operand('y', Int)

iadd = Instruction(
        'iadd', r"""
        Wrapping integer addition: :math:`a := x + y \pmod{2^B}`.

        This instruction does not depend on the signed/unsigned interpretation
        of the operands.
        """,
        ins=(x, y), outs=a)

isub = Instruction(
        'isub', r"""
        Wrapping integer subtraction: :math:`a := x - y \pmod{2^B}`.

        This instruction does not depend on the signed/unsigned interpretation
        of the operands.
        """,
        ins=(x, y), outs=a)

imul = Instruction(
        'imul', r"""
        Wrapping integer multiplication: :math:`a := x y \pmod{2^B}`.

        This instruction does not depend on the signed/unsigned interpretation
        of the
        operands.

        Polymorphic over all integer types (vector and scalar).
        """,
        ins=(x, y), outs=a)

udiv = Instruction(
        'udiv', r"""
        Unsigned integer division: :math:`a := \lfloor {x \over y} \rfloor`.

        This operation traps if the divisor is zero.
        """,
        ins=(x, y), outs=a, can_trap=True)

sdiv = Instruction(
        'sdiv', r"""
        Signed integer division rounded toward zero: :math:`a := sign(xy)
        \lfloor {|x| \over |y|}\rfloor`.

        This operation traps if the divisor is zero, or if the result is not
        representable in :math:`B` bits two's complement. This only happens
        when :math:`x = -2^{B-1}, y = -1`.
        """,
        ins=(x, y), outs=a, can_trap=True)

urem = Instruction(
        'urem', """
        Unsigned integer remainder.

        This operation traps if the divisor is zero.
        """,
        ins=(x, y), outs=a, can_trap=True)

srem = Instruction(
        'srem', """
        Signed integer remainder.

        This operation traps if the divisor is zero.

        .. todo:: Integer remainder vs modulus.

            Clarify whether the result has the sign of the divisor or the
            dividend. Should we add a ``smod`` instruction for the case where
            the result has the same sign as the divisor?
        """,
        ins=(x, y), outs=a, can_trap=True)

a = Operand('a', iB)
x = Operand('x', iB)
Y = Operand('Y', imm64)

iadd_imm = Instruction(
        'iadd_imm', """
        Add immediate integer.

        Same as :inst:`iadd`, but one operand is an immediate constant.

        Polymorphic over all scalar integer types, but does not support vector
        types.
        """,
        ins=(x, Y), outs=a)

imul_imm = Instruction(
        'imul_imm', """
        Integer multiplication by immediate constant.

        Polymorphic over all scalar integer types.
        """,
        ins=(x, Y), outs=a)

udiv_imm = Instruction(
        'udiv_imm', """
        Unsigned integer division by an immediate constant.

        This instruction never traps because a divisor of zero is not allowed.
        """,
        ins=(x, Y), outs=a)

sdiv_imm = Instruction(
        'sdiv_imm', """
        Signed integer division by an immediate constant.

        This instruction never traps because a divisor of -1 or 0 is not
        allowed. """,
        ins=(x, Y), outs=a)

urem_imm = Instruction(
        'urem_imm', """
        Unsigned integer remainder with immediate divisor.

        This instruction never traps because a divisor of zero is not allowed.
        """,
        ins=(x, Y), outs=a)

srem_imm = Instruction(
        'srem_imm', """
        Signed integer remainder with immediate divisor.

        This instruction never traps because a divisor of 0 or -1 is not
        allowed. """,
        ins=(x, Y), outs=a)

irsub_imm = Instruction(
        'irsub_imm', """
        Immediate reverse wrapping subtraction: :math:`a := Y - x \pmod{2^B}`.

        Also works as integer negation when :math:`Y = 0`. Use :inst:`iadd_imm`
        with a negative immediate operand for the reverse immediate
        subtraction.

        Polymorphic over all scalar integer types, but does not support vector
        types.
        """,
        ins=(x, Y), outs=a)

#
# Integer arithmetic with carry and/or borrow.
#
a = Operand('a', iB)
x = Operand('x', iB)
y = Operand('y', iB)
c_in = Operand('c_in', b1, doc="Input carry flag")
c_out = Operand('c_out', b1, doc="Output carry flag")
b_in = Operand('b_in', b1, doc="Input borrow flag")
b_out = Operand('b_out', b1, doc="Output borrow flag")

iadd_cin = Instruction(
        'iadd_cin', r"""
        Add integers with carry in.

        Same as :inst:`iadd` with an additional carry input. Computes:

        .. math::

            a = x + y + c_{in} \pmod 2^B

        Polymorphic over all scalar integer types, but does not support vector
        types.
        """,
        ins=(x, y, c_in), outs=a)

iadd_cout = Instruction(
        'iadd_cout', r"""
        Add integers with carry out.

        Same as :inst:`iadd` with an additional carry output.

        .. math::

            a &= x + y \pmod 2^B \\
            c_{out} &= x+y >= 2^B

        Polymorphic over all scalar integer types, but does not support vector
        types.
        """,
        ins=(x, y), outs=(a, c_out))

iadd_carry = Instruction(
        'iadd_carry', r"""
        Add integers with carry in and out.

        Same as :inst:`iadd` with an additional carry input and output.

        .. math::

            a &= x + y + c_{in} \pmod 2^B \\
            c_{out} &= x + y + c_{in} >= 2^B

        Polymorphic over all scalar integer types, but does not support vector
        types.
        """,
        ins=(x, y, c_in), outs=(a, c_out))

isub_bin = Instruction(
        'isub_bin', r"""
        Subtract integers with borrow in.

        Same as :inst:`isub` with an additional borrow flag input. Computes:

        .. math::

            a = x - (y + b_{in}) \pmod 2^B

        Polymorphic over all scalar integer types, but does not support vector
        types.
        """,
        ins=(x, y, b_in), outs=a)

isub_bout = Instruction(
        'isub_bout', r"""
        Subtract integers with borrow out.

        Same as :inst:`isub` with an additional borrow flag output.

        .. math::

            a &= x - y \pmod 2^B \\
            b_{out} &= x < y

        Polymorphic over all scalar integer types, but does not support vector
        types.
        """,
        ins=(x, y), outs=(a, b_out))

isub_borrow = Instruction(
        'isub_borrow', r"""
        Subtract integers with borrow in and out.

        Same as :inst:`isub` with an additional borrow flag input and output.

        .. math::

            a &= x - (y + b_{in}) \pmod 2^B \\
            b_{out} &= x < y + b_{in}

        Polymorphic over all scalar integer types, but does not support vector
        types.
        """,
        ins=(x, y, b_in), outs=(a, b_out))

#
# Bitwise operations.
#

# TODO: Which types should permit boolean operations? Any reason to restrict?
bits = TypeVar(
        'bits', 'Any integer, float, or boolean scalar or vector type',
        ints=True, floats=True, bools=True, scalars=True, simd=True)

x = Operand('x', bits)
y = Operand('y', bits)
a = Operand('a', bits)

band = Instruction(
        'band', """
        Bitwise and.
        """,
        ins=(x, y), outs=a)

bor = Instruction(
        'bor', """
        Bitwise or.
        """,
        ins=(x, y), outs=a)

bxor = Instruction(
        'bxor', """
        Bitwise xor.
        """,
        ins=(x, y), outs=a)

bnot = Instruction(
        'bnot', """
        Bitwise not.
        """,
        ins=x, outs=a)

# Bitwise binary ops with immediate arg.
x = Operand('x', iB)
Y = Operand('Y', imm64)
a = Operand('a', iB)

band_imm = Instruction(
        'band_imm', """
        Bitwise and with immediate.
        """,
        ins=(x, Y), outs=a)

bor_imm = Instruction(
        'bor_imm', """
        Bitwise or with immediate.
        """,
        ins=(x, Y), outs=a)

bxor_imm = Instruction(
        'bxor_imm', """
        Bitwise xor with immediate.
        """,
        ins=(x, Y), outs=a)

# Shift/rotate.
x = Operand('x', Int, doc='Scalar or vector value to shift')
y = Operand('y', iB, doc='Number of bits to shift')
Y = Operand('Y', imm64)

a = Operand('a', Int)

rotl = Instruction(
        'rotl', r"""
        Rotate left.

        Rotate the bits in ``x`` by ``y`` places.
        """,
        ins=(x, y), outs=a)

rotr = Instruction(
        'rotr', r"""
        Rotate right.

        Rotate the bits in ``x`` by ``y`` places.
        """,
        ins=(x, y), outs=a)

rotl_imm = Instruction(
        'rotl_imm', r"""
        Rotate left by immediate.
        """,
        ins=(x, Y), outs=a)

rotr_imm = Instruction(
        'rotr_imm', r"""
        Rotate right by immediate.
        """,
        ins=(x, Y), outs=a)

ishl = Instruction(
        'ishl', r"""
        Integer shift left. Shift the bits in ``x`` towards the MSB by ``y``
        places. Shift in zero bits to the LSB.

        The shift amount is masked to the size of ``x``.

        When shifting a B-bits integer type, this instruction computes:

        .. math::
            s &:= y \pmod B,                \\
            a &:= x \cdot 2^s \pmod{2^B}.
        """,
        ins=(x, y), outs=a)

ushr = Instruction(
        'ushr', r"""
        Unsigned shift right. Shift bits in ``x`` towards the LSB by ``y``
        places, shifting in zero bits to the MSB. Also called a *logical
        shift*.

        The shift amount is masked to the size of the register.

        When shifting a B-bits integer type, this instruction computes:

        .. math::
            s &:= y \pmod B,                \\
            a &:= \lfloor x \cdot 2^{-s} \rfloor.
        """,
        ins=(x, y), outs=a)

sshr = Instruction(
        'sshr', r"""
        Signed shift right. Shift bits in ``x`` towards the LSB by ``y``
        places, shifting in sign bits to the MSB. Also called an *arithmetic
        shift*.

        The shift amount is masked to the size of the register.
        """,
        ins=(x, y), outs=a)

ishl_imm = Instruction(
        'ishl_imm', r"""
        Integer shift left by immediate.

        The shift amount is masked to the size of ``x``.
        """,
        ins=(x, Y), outs=a)

ushr_imm = Instruction(
        'ushr_imm', r"""
        Unsigned shift right by immediate.

        The shift amount is masked to the size of the register.
        """,
        ins=(x, Y), outs=a)

sshr_imm = Instruction(
        'sshr_imm', r"""
        Signed shift right by immediate.

        The shift amount is masked to the size of the register.
        """,
        ins=(x, Y), outs=a)

#
# Bit counting.
#

x = Operand('x', iB)
a = Operand('a', i8)

clz = Instruction(
        'clz', r"""
        Count leading zero bits.

        Starting from the MSB in ``x``, count the number of zero bits before
        reaching the first one bit. When ``x`` is zero, returns the size of x
        in bits.
        """,
        ins=x, outs=a)

cls = Instruction(
        'cls', r"""
        Count leading sign bits.

        Starting from the MSB after the sign bit in ``x``, count the number of
        consecutive bits identical to the sign bit. When ``x`` is 0 or -1,
        returns one less than the size of x in bits.
        """,
        ins=x, outs=a)

ctz = Instruction(
        'ctz', r"""
        Count trailing zeros.

        Starting from the LSB in ``x``, count the number of zero bits before
        reaching the first one bit. When ``x`` is zero, returns the size of x
        in bits.
        """,
        ins=x, outs=a)

popcnt = Instruction(
        'popcnt', r"""
        Population count

        Count the number of one bits in ``x``.
        """,
        ins=x, outs=a)

#
# Floating point.
#

Float = TypeVar(
        'Float', 'A scalar or vector floating point number',
        floats=True, simd=True)

Cond = Operand('Cond', floatcc)
x = Operand('x', Float)
y = Operand('y', Float)
a = Operand('a', Float.as_bool())

fcmp = Instruction(
        'fcmp', r"""
        Floating point comparison.

        Two IEEE 754-2008 floating point numbers, `x` and `y`, relate to each
        other in exactly one of four ways:

        == ==========================================
        UN Unordered when one or both numbers is NaN.
        EQ When :math:`x = y`. (And :math:`0.0 = -0.0`).
        LT When :math:`x < y`.
        GT When :math:`x > y`.
        == ==========================================

        The 14 :type:`floatcc` condition codes each correspond to a subset of
        the four relations, except for the empty set which would always be
        false, and the full set which would always be true.

        The condition codes are divided into 7 'ordered' conditions which don't
        include UN, and 7 unordered conditions which all include UN.

        +-------+------------+---------+------------+-------------------------+
        |Ordered             |Unordered             |Condition                |
        +=======+============+=========+============+=========================+
        |ord    |EQ | LT | GT|uno      |UN          |NaNs absent / present.   |
        +-------+------------+---------+------------+-------------------------+
        |eq     |EQ          |ueq      |UN | EQ     |Equal                    |
        +-------+------------+---------+------------+-------------------------+
        |one    |LT | GT     |ne       |UN | LT | GT|Not equal                |
        +-------+------------+---------+------------+-------------------------+
        |lt     |LT          |ult      |UN | LT     |Less than                |
        +-------+------------+---------+------------+-------------------------+
        |le     |LT | EQ     |ule      |UN | LT | EQ|Less than or equal       |
        +-------+------------+---------+------------+-------------------------+
        |gt     |GT          |ugt      |UN | GT     |Greater than             |
        +-------+------------+---------+------------+-------------------------+
        |ge     |GT | EQ     |uge      |UN | GT | EQ|Greater than or equal    |
        +-------+------------+---------+------------+-------------------------+

        The standard C comparison operators, `<, <=, >, >=`, are all ordered,
        so they are false if either operand is NaN. The C equality operator,
        `==`, is ordered, and since inequality is defined as the logical
        inverse it is *unordered*. They map to the :type:`floatcc` condition
        codes as follows:

        ==== ====== ============
        C    `Cond` Subset
        ==== ====== ============
        `==` eq     EQ
        `!=` ne     UN | LT | GT
        `<`  lt     LT
        `<=` le     LT | EQ
        `>`  gt     GT
        `>=` ge     GT | EQ
        ==== ====== ============

        This subset of condition codes also corresponds to the WebAssembly
        floating point comparisons of the same name.

        When this instruction compares floating point vectors, it returns a
        boolean vector with the results of lane-wise comparisons.
        """,
        ins=(Cond, x, y), outs=a)

x = Operand('x', Float)
y = Operand('y', Float)
z = Operand('z', Float)
a = Operand('a', Float, 'Result of applying operator to each lane')

fadd = Instruction(
        'fadd', r"""
        Floating point addition.
        """,
        ins=(x, y), outs=a)

fsub = Instruction(
        'fsub', r"""
        Floating point subtraction.
        """,
        ins=(x, y), outs=a)

fmul = Instruction(
        'fmul', r"""
        Floating point multiplication.
        """,
        ins=(x, y), outs=a)

fdiv = Instruction(
        'fdiv', r"""
        Floating point division.

        Unlike the integer division instructions :cton:inst:`sdiv` and
        :cton:inst:`udiv`, this can't trap. Division by zero is infinity or
        NaN, depending on the dividend.
        """,
        ins=(x, y), outs=a)

sqrt = Instruction(
        'sqrt', r"""
        Floating point square root.
        """,
        ins=x, outs=a)

fma = Instruction(
        'fma', r"""
        Floating point fused multiply-and-add.

        Computes :math:`a := xy+z` without any intermediate rounding of the
        product.
        """,
        ins=(x, y, z), outs=a)

a = Operand('a', Float, '``x`` with its sign bit inverted')
fneg = Instruction(
        'fneg', r"""
        Floating point negation.

        Note that this is a pure bitwise operation.
        """,
        ins=x, outs=a)

a = Operand('a', Float, '``x`` with its sign bit cleared')
fabs = Instruction(
        'fabs', r"""
        Floating point absolute value.

        Note that this is a pure bitwise operation.
        """,
        ins=x, outs=a)

a = Operand('a', Float, '``x`` with its sign bit changed to that of ``y``')
fcopysign = Instruction(
        'fcopysign', r"""
        Floating point copy sign.

        Note that this is a pure bitwise operation. The sign bit from ``y`` is
        copied to the sign bit of ``x``.
        """,
        ins=(x, y), outs=a)

a = Operand('a', Float, 'The smaller of ``x`` and ``y``')

fmin = Instruction(
        'fmin', r"""
        Floating point minimum, propagating NaNs.

        If either operand is NaN, this returns a NaN.
        """,
        ins=(x, y), outs=a)

fminnum = Instruction(
        'fminnum', r"""
        Floating point minimum, suppressing quiet NaNs.

        If either operand is a quiet NaN, the other operand is returned. If
        either operand is a signaling NaN, NaN is returned.
        """,
        ins=(x, y), outs=a)

a = Operand('a', Float, 'The larger of ``x`` and ``y``')

fmax = Instruction(
        'fmax', r"""
        Floating point maximum, propagating NaNs.

        If either operand is NaN, this returns a NaN.
        """,
        ins=(x, y), outs=a)

fmaxnum = Instruction(
        'fmaxnum', r"""
        Floating point maximum, suppressing quiet NaNs.

        If either operand is a quiet NaN, the other operand is returned. If
        either operand is a signaling NaN, NaN is returned.
        """,
        ins=(x, y), outs=a)

a = Operand('a', Float, '``x`` rounded to integral value')

ceil = Instruction(
        'ceil', r"""
        Round floating point round to integral, towards positive infinity.
        """,
        ins=x, outs=a)

floor = Instruction(
        'floor', r"""
        Round floating point round to integral, towards negative infinity.
        """,
        ins=x, outs=a)

trunc = Instruction(
        'trunc', r"""
        Round floating point round to integral, towards zero.
        """,
        ins=x, outs=a)

nearest = Instruction(
        'nearest', r"""
        Round floating point round to integral, towards nearest with ties to
        even.
        """,
        ins=x, outs=a)


#
# Conversions
#

Mem = TypeVar(
        'Mem', 'Any type that can be stored in memory',
        ints=True, floats=True, simd=True)
MemTo = TypeVar(
        'MemTo', 'Any type that can be stored in memory',
        ints=True, floats=True, simd=True)

x = Operand('x', Mem)
a = Operand('a', MemTo, 'Bits of `x` reinterpreted')

bitcast = Instruction(
        'bitcast', r"""
        Reinterpret the bits in `x` as a different type.

        The input and output types must be storable to memory and of the same
        size. A bitcast is equivalent to storing one type and loading the other
        type from the same address.
        """,
        ins=x, outs=a)

Int = TypeVar('Int', 'A scalar or vector integer type', ints=True, simd=True)
IntTo = TypeVar(
        'IntTo', 'A smaller integer type with the same number of lanes',
        ints=True, simd=True)

x = Operand('x', Int)
a = Operand('a', IntTo)

ireduce = Instruction(
        'ireduce', r"""
        Convert `x` to a smaller integer type by dropping high bits.

        Each lane in `x` is converted to a smaller integer type by discarding
        the most significant bits. This is the same as reducing modulo
        :math:`2^n`.

        The result type must have the same number of vector lanes as the input,
        and each lane must not have more bits that the input lanes. If the
        input and output types are the same, this is a no-op.
        """,
        ins=x, outs=a)


IntTo = TypeVar(
        'IntTo', 'A larger integer type with the same number of lanes',
        ints=True, simd=True)

x = Operand('x', Int)
a = Operand('a', IntTo)

uextend = Instruction(
        'uextend', r"""
        Convert `x` to a larger integer type by zero-extending.

        Each lane in `x` is converted to a larger integer type by adding
        zeroes. The result has the same numerical value as `x` when both are
        interpreted as unsigned integers.

        The result type must have the same number of vector lanes as the input,
        and each lane must not have fewer bits that the input lanes. If the
        input and output types are the same, this is a no-op.
        """,
        ins=x, outs=a)

sextend = Instruction(
        'sextend', r"""
        Convert `x` to a larger integer type by sign-extending.

        Each lane in `x` is converted to a larger integer type by replicating
        the sign bit. The result has the same numerical value as `x` when both
        are interpreted as signed integers.

        The result type must have the same number of vector lanes as the input,
        and each lane must not have fewer bits that the input lanes. If the
        input and output types are the same, this is a no-op.
        """,
        ins=x, outs=a)

FloatTo = TypeVar(
        'FloatTo', 'A scalar or vector floating point number',
        floats=True, simd=True)

x = Operand('x', Float)
a = Operand('a', FloatTo)

fpromote = Instruction(
        'fpromote', r"""
        Convert `x` to a larger floating point format.

        Each lane in `x` is converted to the destination floating point format.
        This is an exact operation.

        Since Cretonne currently only supports two floating point formats, this
        instruction always converts :type:`f32` to :type:`f64`. This may change
        in the future.

        The result type must have the same number of vector lanes as the input,
        and the result lanes must be larger than the input lanes.
        """,
        ins=x, outs=a)

fdemote = Instruction(
        'fdemote', r"""
        Convert `x` to a smaller floating point format.

        Each lane in `x` is converted to the destination floating point format
        by rounding to nearest, ties to even.

        Since Cretonne currently only supports two floating point formats, this
        instruction always converts :type:`f64` to :type:`f32`. This may change
        in the future.

        The result type must have the same number of vector lanes as the input,
        and the result lanes must be smaller than the input lanes.
        """,
        ins=x, outs=a)

x = Operand('x', Float)
a = Operand('a', IntTo)

fcvt_to_uint = Instruction(
        'fcvt_to_uint', r"""
        Convert floating point to unsigned integer.

        Each lane in `x` is converted to an unsigned integer by rounding
        towards zero. If `x` is NaN or if the unsigned integral value cannot be
        represented in the result type, this instruction traps.

        The result type must have the same number of vector lanes as the input.
        """,
        ins=x, outs=a, can_trap=True)

fcvt_to_sint = Instruction(
        'fcvt_to_sint', r"""
        Convert floating point to signed integer.

        Each lane in `x` is converted to a signed integer by rounding towards
        zero. If `x` is NaN or if the signed integral value cannot be
        represented in the result type, this instruction traps.

        The result type must have the same number of vector lanes as the input.
        """,
        ins=x, outs=a, can_trap=True)

x = Operand('x', Int)
a = Operand('a', FloatTo)

fcvt_from_uint = Instruction(
        'fcvt_from_uint', r"""
        Convert unsigned integer to floating point.

        Each lane in `x` is interpreted as an unsigned integer and converted to
        floating point using round to nearest, ties to even.

        The result type must have the same number of vector lanes as the input.
        """,
        ins=x, outs=a)

fcvt_from_sint = Instruction(
        'fcvt_from_sint', r"""
        Convert signed integer to floating point.

        Each lane in `x` is interpreted as a signed integer and converted to
        floating point using round to nearest, ties to even.

        The result type must have the same number of vector lanes as the input.
        """,
        ins=x, outs=a)

#
# Legalization helper instructions.
#

WideInt = TypeVar(
        'WideInt', 'A scalar integer type from `i16` upwards',
        ints=(16, 64))
x = Operand('x', WideInt)
lo = Operand(
        'lo', WideInt.half_width(), 'The low bits of `x`')
hi = Operand(
        'hi', WideInt.half_width(), 'The high bits of `x`')

isplit_lohi = Instruction(
        'isplit_lohi', r"""
        Split a scalar integer into low and high parts.

        Returns the low half of `x` and the high half of `x` as two independent
        values.
        """,
        ins=x, outs=(lo, hi))


NarrowInt = TypeVar(
        'NarrowInt', 'A scalar integer type up to `i32`',
        ints=(8, 32))
lo = Operand('lo', NarrowInt)
hi = Operand('hi', NarrowInt)
a = Operand(
        'a', NarrowInt.double_width(),
        doc='The concatenation of `lo` and `hi`')

iconcat_lohi = Instruction(
        'iconcat_lohi', r"""
        Concatenate low and high bits to form a larger integer type.
        """,
        ins=(lo, hi), outs=a)

GROUP.close()
