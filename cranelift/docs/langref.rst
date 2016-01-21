****************************************
Cretonne Intermediate Language Reference
****************************************

.. default-domain:: cton
.. highlight:: cton

The Cretonne intermediate language has two equivalent representations: an
*in-memory data structure* that the code generator library is using, and
a *text format* which is used for test cases and debug output. Files containing
Cretonne textual IL have the ``.cton`` filename extension.

This reference uses the text format to describe IL semantics but glosses over
the details of the lexical and syntactic structure of the test format.

Overall structure
=================

Cretonne compiles functions independently. A ``.cton`` IL file may contain
multiple functions, and the programmatic API can create multiple function
handles at the same time, but the functions don't share any data or reference
each other directly.

This is a C function that computes the average of an array of floats:

.. code-block:: c

    float average(const float *array, size_t count) {
        double sum = 0;
        for (size_t i = 0; i < count; i++)
            sum += array[i];
        return sum / count;
    }

Here it is compiled into Cretonne IL::

    function average(i32, i32) -> f32 {
        ; Preamble.
        ss1 = local 8, align 4

    entry ebb1(v1: i32, v2: i32):
        v3 = fconst.f64 0.0
        stack_store v3, ss1
        brz v2, ebb3              ; Handle count == 0.
        v4 = iconst.i32 0
        br ebb2(v4)

    ebb2(v5: i32):
        ; Compute address of array element.
        v6 = imul_imm v5, 4
        v7 = iadd v1, v6
        v8 = heap_load.f32 v7     ; array[i]
        v9 = fext.f64 v8
        ; Add to accumulator in ss1.
        v10 = stack_load.f64 ss1
        v11 = fadd v9, v10
        stack_store v11, ss1
        ; Increment loop counter.
        v12 = iadd_imm v5, 1
        v13 = icmp ult v12, v2
        brnz v13, ebb2(v12)       ; Loop backedge.
        ; Compute average from sum.
        v14 = stack_load.f64 ss1
        v15 = cvt_utof.f64 v2
        v16 = fdiv v14, v15
        v17 = ftrunc.f32 v16
        return v17

    ebb3:
        v100 = fconst.f32 0x7f800000 ; Inf
        return v100
    }
        


Type system
===========


All SSA values have a type which determines the size and shape (for SIMD
vectors) of the value. Many instructions are polymorphic -- they can operate on
different types.

.. type:: bool

    A boolean value that is either true or false. Booleans can't be stored in
    memory.

Integer types
-------------

Integer values have a fixed size and can be interpreted as either signed or
unsigned. Some instructions will interpret an operand as a signed or unsigned
number, others don't care.

.. type:: i8

    A 8-bit integer value taking up 1 byte in memory.

.. type:: i16

    A 16-bit integer value taking up 2 bytes in memory.

.. type:: i32

    A 32-bit integer value taking up 4 bytes in memory.

.. type:: i64

    A 64-bit integer value taking up 8 bytes in memory.

Floating point types
--------------------

The floating point types have the IEEE semantics that are supported by most
hardware. There is no support for higher-precision types like quads or
double-double formats.

.. type:: f32

    A 32-bit floating point type represented in the IEEE 754 *Single precision*
    format. This corresponds to the :c:type:`float` type in most C
    implementations.

.. type:: f64

    A 64-bit floating point type represented in the IEEE 754 *Double precision*
    format. This corresponds to the :c:type:`double` type in most C
    implementations.

SIMD vector types
-----------------

A SIMD vector type represents a vector of values from one of the scalar types
(:type:`bool`, integer, and floating point). Each scalar value in a SIMD type is
called a *lane*. The number of lanes must be a power of two in the range 2-256.

.. type:: vNiB

    A SIMD vector of integers. The lane type :type:`iB` must be one of the
    integer types :type:`i8` ... :type:`i64`.

    Some concrete integer vector types are :type:`v4i32`, :type:`v8i64`, and
    :type:`v4i16`.

    The size of a SIMD integer vector in memory is :math:`N B\over 8` bytes.

.. type:: vNf32

    A SIMD vector of single precision floating point numbers.

    Some concrete :type:`f32` vector types are: :type:`v2f32`, :type:`v4f32`,
    and :type:`v8f32`.

    The size of a :type:`f32` vector in memory is :math:`4N` bytes.

.. type:: vNf64

    A SIMD vector of double precision floating point numbers.

    Some concrete :type:`f64` vector types are: :type:`v2f64`, :type:`v4f64`,
    and :type:`v8f64`.

    The size of a :type:`f64` vector in memory is :math:`8N` bytes.

.. type:: vNbool

    A boolean SIMD vector.

    Like the :type:`bool` type, a boolean vector cannot be stored in memory. It
    can only be used for ephemeral SSA values.

Instructions
============

Control flow instructions
-------------------------

.. inst:: br EBB(args...)

    Branch.

    Unconditionally branch to an extended basic block, passing the specified
    EBB arguments. The number and types of arguments must match the destination
    EBB.

.. inst:: brz x, EBB(args...)

    Branch when zero.
    
    If ``x`` is a :type:`bool` value, take the branch when ``x`` is false. If
    ``x`` is an integer value, take the branch when ``x = 0``.

    :param iN/bool x: Value to test.
    :param EBB: Destination extended basic block.

.. inst:: brnz x, EBB(args...)

    Branch when non-zero.
    
    If ``x`` is a :type:`bool` value, take the branch when ``x`` is true. If
    ``x`` is an integer value, take the branch when ``x != 0``.

    :param iN/bool x: Value to test.
    :param EBB: Destination extended basic block.

Special operations
==================

Most operations are easily classified as arithmetic or control flow. These
instructions are not so easily classified.

.. inst:: a = iconst n

    Integer constant.

.. inst:: a = fconst n

    Floating point constant.

.. inst:: a = vconst n

    Vector constant (floating point or integer).

.. inst:: a = select c, x, y

    Conditional select.

    :param c bool: Controlling flag.
    :param x: Value to return when ``c`` is true.
    :param y: Value to return when ``c`` is false. Must be same type as ``x``.
    :rtype: Same type as ``x`` and ``y``.

    This instruction selects whole values. Use :inst:`vselect` for
    lane-wise selection.

Vector operations
=================

.. inst:: a  = vselect c, x, y

    Vector lane select.

    Select lanes from ``x`` or ``y`` controlled by the lanes of the boolean
    vector ``c``.

    :arg vNbool c: Controlling flag vector.
    :arg x: Vector with lanes selected by the true lanes of ``c``.
              Must be a vector type with the same number of lanes as ``c``.
    :arg y: Vector with lanes selected by the false lanes of ``c``.
              Must be same type as ``x``.
    :rtype: Same type as ``x`` and ``y``.

.. inst:: a = vbuild x, y, z, ...

    Vector build.

    Build a vector value from the provided lanes.

.. inst:: a = splat x

    Vector splat.

    Return a vector whose lanes are all ``x``.

.. inst:: a = insertlane x, idx, y

    Insert ``y`` as lane ``idx`` in x.

    The lane index, ``idx``, is an immediate value, not an SSA value. It must
    indicate a valid lane index for the type of ``x``.

.. inst:: a = extractlane x, idx

    Extract lane ``idx`` from ``x``.

    The lane index, ``idx``, is an immediate value, not an SSA value. It must
    indicate a valid lane index for the type of ``x``.

Integer operations
==================

.. inst:: a = icmp cond, x, y

    Integer comparison.

    :param cond: Condition code determining how ``x`` and ``y`` are compared.
    :param x, y: Integer scalar or vector values of the same type.
    :rtype: :type:`bool` or :type:`vNbool` with the same number of lanes as
            ``x`` and ``y``.

    The condition code determines if the operands are interpreted as signed or
    unsigned integers.

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

.. inst:: a = iadd x, y

    Wrapping integer addition: :math:`a := x + y \pmod{2^B}`. This instruction
    does not depend on the signed/unsigned interpretation of the operands.

.. inst:: a = isub x, y

    Wrapping integer subtraction: :math:`a := x - y \pmod{2^B}`. This
    instruction does not depend on the signed/unsigned interpretation of the
    operands.

.. todo:: Overflow arithmetic

    Add instructions for add with carry out / carry in and so on. Enough to
    implement larger integer types efficiently. It should also be possible to
    legalize :type:`i64` arithmetic to terms of :type:`i32` operations.

.. inst:: a = ineg x

    Wrapping integer negation: :math:`a := -x \pmod{2^B}`. This instruction does
    not depend on the signed/unsigned interpretation of the operand.

.. inst:: a = imul x, y

    Wrapping integer multiplication: :math:`a := x y \pmod{2^B}`. This
    instruction does not depend on the signed/unsigned interpretation of the
    operands.

.. todo:: Larger multiplication results.

    For example, ``smulx`` which multiplies :type:`i32` operands to produce a
    :type:`i64` result. Alternatively, ``smulhi`` and ``smullo`` pairs.

.. inst:: a = udiv x, y

    Unsigned integer division: :math:`a := \lfloor {x \over y} \rfloor`. This
    operation traps if the divisor is zero.

    .. todo::
        Add a ``udiv_imm`` variant with an immediate divisor greater than 1.
        This is useful for pattern-matching divide-by-constant, and this
        instruction would be non-trapping.

.. inst:: a = sdiv x, y

    Signed integer division rounded toward zero: :math:`a := sign(xy) \lfloor
    {|x| \over |y|}\rfloor`. This operation traps if the divisor is zero, or if
    the result is not representable in :math:`B` bits two's complement. This only
    happens when :math:`x = -2^{B-1}, y = -1`.

    .. todo::
        Add a ``sdiv_imm`` variant with an immediate non-zero divisor. This is
        useful for pattern-matching divide-by-constant, and this instruction
        would be non-trapping. Don't allow divisors 0, 1, or -1.

.. inst:: a = urem x, y

    Unsigned integer remainder. This operation traps if the divisor is zero.

    .. todo::
        Add a ``urem_imm`` non-trapping variant.

.. inst:: a = srem x, y

    Signed integer remainder. This operation traps if the divisor is zero.

    .. todo::
        Clarify whether the result has the sign of the divisor or the dividend.
        Should we add a ``smod`` instruction for the case where the result has
        the same sign as the divisor?

.. todo:: Minimum / maximum.

    NEON has ``smin``, ``smax``, ``umin``, and ``umax`` instructions. We should
    replicate those for both scalar and vector integer types. Even if the
    target ISA doesn't have scalar operations, these are good pattern mtching
    targets.

.. todo:: Saturating arithmetic.

    Mostly for SIMD use, but again these are good paterns to contract.
    Something like ``usatadd``, ``usatsub``, ``ssatadd``, and ``ssatsub`` is a
    good start.

Bitwise operations
==================

.. inst:: a = and x, y

    Bitwise and.

    :rtype: bool, iB, vNiB, vNfB?

.. inst:: a = or x, y

    Bitwise or.

    :rtype: bool, iB, vNiB, vNfB?

.. inst:: a = xor x, y

    Bitwise xor.

    :rtype: bool, iB, vNiB, vNfB?

.. inst:: a = not x

    Bitwise not.

    :rtype: bool, iB, vNiB, vNfB?

.. todo:: Redundant bitwise operators.

    ARM has instructions like ``bic(x,y) = x & ~y``, ``orn(x,y) = x | ~y``, and
    ``eon(x,y) = x ^ ~y``.

.. inst:: a = rotl x, y

    Rotate left.

    Rotate the bits in ``x`` by ``y`` places.

    :param x: Integer value to be rotated.
    :param y: Number of bits to shift. Any integer type, not necessarily the
              same type as ``x``.
    :rtype: Same type as ``x``.

.. inst:: a = rotr x, y

    Rotate right.

    Rotate the bits in ``x`` by ``y`` places.

    :param x: Integer value to be rotated.
    :param y: Number of bits to shift. Any integer type, not necessarily the
              same type as ``x``.
    :rtype: Same type as ``x``.

.. inst:: a = ishl x, y

    Integer shift left. Shift the bits in ``x`` towards the MSB by ``y``
    places. Shift in zero bits to the LSB.

    The shift amount is masked to the size of ``x``.

    :param x: Integer value to be shifted.
    :param y: Number of bits to shift. Any integer type, not necessarily the
              same type as ``x``.
    :rtype: Same type as ``x``.

    When shifting a B-bits integer type, this instruction computes:

    .. math::
        s &:= y \pmod B,                \\
        a &:= x \cdot 2^s \pmod{2^B}.

    .. todo:: Add ``ishl_imm`` variant with an immediate ``y``.

.. inst:: a = ushr x, y

    Unsigned shift right. Shift bits in ``x`` towards the LSB by ``y`` places,
    shifting in zero bits to the MSB. Also called a *logical shift*.

    The shift amount is masked to the size of the register.

    :param x: Integer value to be shifted.
    :param y: Number of bits to shift. Can be any integer type, not necessarily
              the same type as ``x``.
    :rtype: Same type as ``x``.

    When shifting a B-bits integer type, this instruction computes:

    .. math::
        s &:= y \pmod B,                \\
        a &:= \lfloor x \cdot 2^{-s} \rfloor.

    .. todo:: Add ``ushr_imm`` variant with an immediate ``y``.

.. inst:: a = sshr x, y

    Signed shift right. Shift bits in ``x`` towards the LSB by ``y`` places,
    shifting in sign bits to the MSB. Also called an *arithmetic shift*.

    The shift amount is masked to the size of the register.

    :param x: Integer value to be shifted.
    :param y: Number of bits to shift. Can be any integer type, not necessarily
              the same type as ``x``.
    :rtype: Same type as ``x``.

    .. todo:: Add ``sshr_imm`` variant with an immediate ``y``.

.. inst:: a = clz x

    Count leading zero bits.

    :param x: Integer value.
    :rtype: :type:`i8`

    Starting from the MSB in ``x``, count the number of zero bits before
    reaching the first one bit. When ``x`` is zero, returns the size of x in
    bits.

.. inst:: a = cls x

    Count leading sign bits.

    :param x: Integer value.
    :rtype: :type:`i8`

    Starting from the MSB after the sign bit in ``x``, count the number of
    consecutive bits identical to the sign bit. When ``x`` is 0 or -1, returns
    one less than the size of x in bits.

.. inst:: a = ctz x

    Count trailing zeros.

    :param x: Integer value.
    :rtype: :type:`i8`

    Starting from the LSB in ``x``, count the number of zero bits before
    reaching the first one bit. When ``x`` is zero, returns the size of x in
    bits.

.. inst:: a = popcnt x

    Population count

    :param x: Integer value.
    :rtype: :type:`i8`

    Count the number of one bits in ``x``.


Floating point operations
=========================

.. inst:: a = fcmp cond, x, y

    Floating point comparison.

    :param cond: Condition code determining how ``x`` and ``y`` are compared.
    :param x, y: Floating point scalar or vector values of the same type.
    :rtype: :type:`bool` or :type:`vNbool` with the same number of lanes as
            ``x`` and ``y``.

    An 'ordered' condition code yields ``false`` if either operand is Nan.

    An 'unordered' condition code yields ``true`` if either operand is Nan.

    ======= ========= =========
    Ordered Unordered Condition
    ======= ========= =========
    ord     uno       None (ord = no NaNs, uno = some NaNs)
    oeq     ueq       Equal
    one     une       Not equal
    olt     ult       Less than
    oge     uge       Greater than or equal
    ogt     ugt       Greater than
    ole     ule       Less than or equal
    ======= ========= =========

.. inst:: fadd x,y

    Floating point addition.

.. inst:: fsub x,y

    Floating point subtraction.

.. inst:: fneg x

    Floating point negation.

    :returns: ``x`` with its sign bit inverted.

    Note that this is a pure bitwise operation.

.. inst:: fabs x

    Floating point absolute value.

    :returns: ``x`` with its sign bit cleared.

    Note that this is a pure bitwise operation.

.. inst::  a = fcopysign x, y

    Floating point copy sign.

    :returns: ``x`` with its sign changed to that of ``y``.

    Note that this is a pure bitwise operation. The sign bit from ``y`` is
    copied to the sign bit of ``x``.

.. inst:: fmul x, y
.. inst:: fdiv x, y
.. inst:: fmin x, y
.. inst:: fminnum x, y
.. inst:: fmax x, y
.. inst:: fmaxnum x, y
.. inst:: ceil x

    Round floating point round to integral, towards positive infinity.

.. inst:: floor x

    Round floating point round to integral, towards negative infinity.

.. inst:: trunc x

    Round floating point round to integral, towards zero.

.. inst:: nearest x

    Round floating point round to integral, towards nearest with ties to even.

.. inst:: sqrt x

    Floating point square root.

.. inst:: a = fma x, y, z

    Floating point fused multiply-and-add.

    Computes :math:`a := xy+z` wihtout any intermediate rounding of the
    product.

Conversion operations
=====================

.. inst:: a = bitcast x

    Reinterpret the bits in ``x`` as a different type.

    The input and output types must be storable to memory and of the same size.
    A bitcast is equivalent to storing one type and loading the other type from
    the same address.

.. inst:: a = itrunc x
.. inst:: a = uext x
.. inst:: a = sext x
.. inst:: a = ftrunc x
.. inst:: a = fext x
.. inst:: a = cvt_ftou x
.. inst:: a = cvt_ftos x
.. inst:: a = cvt_utof x
.. inst:: a = cvt_stof x

