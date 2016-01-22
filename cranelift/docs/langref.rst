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
the finer details of the lexical and syntactic structure of the format.


Overall structure
=================

Cretonne compiles functions independently. A ``.cton`` IL file may contain
multiple functions, and the programmatic API can create multiple function
handles at the same time, but the functions don't share any data or reference
each other directly.

This is a simple C function that computes the average of an array of floats:

.. literalinclude:: example.c
    :language: c

Here is the same function compiled into Cretonne IL:

.. literalinclude:: example.cton
    :language: cton
    :linenos:
    :emphasize-lines: 2

The first line of a function definition provides the function *name* and
the :term:`function signature` which declares the argument and return types.
Then follows the :term:`function preample` which declares a number of entities
that can be referenced inside the function. In the example above, the preample
declares a single local variable, ``ss1``.

After the preample follows the :term:`function body` which consists of
:term:`extended basic block`\s, one of which is marked as the :term:`entry
block`. Every EBB ends with a :term:`terminator instruction`, so execution can
never fall through to the next EBB without an explicit branch.

Static single assignment form
-----------------------------

The instructions in the function body use and produce *values* in SSA form. This
means that every value is defined exactly once, and every use of a value must be
dominated by the definition.

Cretonne does not have phi instructions but uses *EBB arguments* instead. An EBB
can be defined with a list of typed arguments. Whenever control is transferred
to the EBB, values for the arguments must be provided. When entering a function,
the incoming function arguments are passed as arguments to the entry EBB.

Instructions define zero, one, or more result values. All SSA values are either
EBB arguments or instruction results.

In the example above, the loop induction variable ``i`` is represented as three
SSA values: In the entry block, ``v4`` is the initial value. In the loop block
``ebb2``, the EBB argument ``v5`` represents the value of the induction
variable during each iteration. Finally, ``v12`` is computed as the induction
variable value for the next iteration.

It can be difficult to generate correct SSA form if the program being converted
into Cretonne IL contains multiple assignments to the same variables. Such
variables can be presented to Cretonne as :term:`stack slot`\s instead. Stack
slots are accessed with the :inst:`stack_store` and :inst:`stack_load`
instructions which behave more like variable accesses in a typical programming
language. Cretonne can perform the necessary dataflow analysis to convert stack
slots to SSA form.

If all values are only used in the same EBB where they are defined, the
function is said to be in :term:`local SSA form`. It is much faster for
Cretonne to verify the correctness of a function in local SSA form since no
complicated control flow analysis is required.


Value types
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

    A 32-bit floating point type represented in the IEEE 754 *single precision*
    format. This corresponds to the :c:type:`float` type in most C
    implementations.

.. type:: f64

    A 64-bit floating point type represented in the IEEE 754 *double precision*
    format. This corresponds to the :c:type:`double` type in most C
    implementations.

SIMD vector types
-----------------

A SIMD vector type represents a vector of values from one of the scalar types
(:type:`bool`, integer, and floating point). Each scalar value in a SIMD type is
called a *lane*. The number of lanes must be a power of two in the range 2-256.

.. type:: i%Bx%N

    A SIMD vector of integers. The lane type :type:`iB` is one of the integer
    types :type:`i8` ... :type:`i64`.

    Some concrete integer vector types are :type:`i32x4`, :type:`i64x8`, and
    :type:`i16x4`.

    The size of a SIMD integer vector in memory is :math:`N B\over 8` bytes.

.. type:: f32x%N

    A SIMD vector of single precision floating point numbers.

    Some concrete :type:`f32` vector types are: :type:`f32x2`, :type:`f32x4`,
    and :type:`f32x8`.

    The size of a :type:`f32` vector in memory is :math:`4N` bytes.

.. type:: f64x%N

    A SIMD vector of double precision floating point numbers.

    Some concrete :type:`f64` vector types are: :type:`f64x2`, :type:`f64x4`,
    and :type:`f64x8`.

    The size of a :type:`f64` vector in memory is :math:`8N` bytes.

.. type:: boolx%N

    A boolean SIMD vector.

    Boolean vectors are used when comparing SIMD vectors. For example,
    comparing two :type:`i32x4` values would produce a :type:`boolx4` result.

    Like the :type:`bool` type, a boolean vector cannot be stored in memory.

Pseudo-types
------------

These are not concrete types, but convenient names uses to refer to real types
in this reference.

.. type:: iPtr

    A Pointer-sized integer.

    This is either :type:`i32`, or :type:`i64`, depending on whether the target
    platform has 32-bit or 64-bit pointers.

Control flow
============

Branches transfer control to a new EBB and provide values for the target EBB's
arguments, if it has any. Conditional branches only take the branch if their
condition is satisfied, otherwise execution continues at the following
instruction in the EBB.

.. inst:: br EBB(args...)

    Branch.

    Unconditionally branch to an extended basic block, passing the specified
    EBB arguments. The number and types of arguments must match the destination
    EBB.

    :arg EBB: Destination extended basic block.
    :arg args...: Zero or more arguments passed to EBB.
    :result: None. This is a terminator instruction.

.. inst:: brz x, EBB(args...)

    Branch when zero.

    If ``x`` is a :type:`bool` value, take the branch when ``x`` is false. If
    ``x`` is an integer value, take the branch when ``x = 0``.

    :arg iN/bool x: Value to test.
    :arg EBB: Destination extended basic block.
    :arg args...: Arguments passed to EBB.
    :result: None.

.. inst:: brnz x, EBB(args...)

    Branch when non-zero.

    If ``x`` is a :type:`bool` value, take the branch when ``x`` is true. If
    ``x`` is an integer value, take the branch when ``x != 0``.

    :arg iN/bool x: Value to test.
    :arg EBB: Destination extended basic block.
    :arg args...: Zero or more arguments passed to EBB.
    :result: None.

.. inst:: br_table x, JT

    Jump table branch.

    Use ``x`` as an unsigned index into the jump table ``JT``. If a jump table
    entry is found, branch to the corresponding EBB. If no entry was found fall
    through to the next instruction.

    Note that this branch instruction can't pass arguments to the targeted
    blocks. Split critical edges as needed to work around this.

    :arg iN x: Integer index into jump table.
    :arg JT: Jump table which was declared in the preample.
    :result: None.

.. inst:: JT = jump_table EBB0, EBB1, ..., EBBn

    Declare a jump table in the :term:`function preample`.

    This declares a jump table for use by the :inst:`br_table` indirect branch
    instruction. Entries in the table are either EBB names, or ``0`` which
    indicates an absent entry.

    The EBBs listed must belong to the current function, and they can't have
    any arguments.

    :arg EBB0: Target EBB when ``x = 0``.
    :arg EBB1: Target EBB when ``x = 1``.
    :arg EBBn: Target EBB when ``x = n``.
    :result: A jump table identifier. (Not an SSA value).

Traps stop the program because something went wrong. The exact behavior depends
on the target instruction set architecture and operating system. There are
explicit trap instructions defined below, but some instructions may also cause
traps for certain input value. For example, :inst:`udiv` traps when the divisor
is zero.

.. inst:: trap

    Terminate execution unconditionally.

    :result: None. This is a terminator instruction.

.. inst:: trapz x

    Trap when zero.

    if ``x`` is non-zero, execution continues at the following instruction.

    :arg iN/bool x: Value to test.
    :result: None.

.. inst:: trapnz x

    Trap when non-zero.

    if ``x`` is zero, execution continues at the following instruction.

    :arg iN/bool x: Value to test.
    :result: None.


Function calls
==============

A function call needs a target function and a :term:`function signature`. The
target function may be determined dynamically at runtime, but the signature
must be known when the function call is compiled. The function signature
describes how to call the function, including arguments, return values, and the
calling convention:

.. productionlist::
    signature : "(" [arglist] ")" ["->" retlist] [call_conv]
    arglist   : arg
              : arglist "," arg
    retlist   : arglist
    arg       : type
              : arg flag
    flag      : "uext" | "sext" | "inreg"
    callconv  : `string`

Arguments and return values have flags whose meaning is mostly target
dependent. They make it possible to call native functions on the target
platform. When calling other Cretonne functions, the flags are not necessary.

Functions that are called directly must be declared in the :term:`function
preample`:

.. inst:: F = function NAME signature

    Declare a function so it can be called directly.

    :arg NAME: Name of the function, passed to the linker for resolution.
    :arg signature: Function signature. See below.
    :result F: A function identifier that can be used with :inst:`call`.

.. inst:: a, b, ... = call F(args...)

    Direct function call.

    :arg F: Function identifier to call, declared by :inst:`function`.
    :arg args...: Function arguments matching the signature of F.
    :result a,b,...: Return values matching the signature of F.

.. inst:: return args...

    Return from function.

    Unconditionally transfer control to the calling function, passing the
    provided return values.

    :arg args: Return values. The list of return values must match the list of
               return value types in the function signature.
    :result: None. This is a terminator instruction.

This simple example illustrates direct function calls and signatures::

    function gcd(i32 uext, i32 uext) -> i32 uext "C" {
        f1 = function divmod(i32 uext, i32 uext) -> i32 uext, i32 uext

    entry ebb1(v1: i32, v2: i32):
        brz v2, ebb2
        v3, v4 = call f1(v1, v2)
        br ebb1(v2, v4)

    ebb2:
        return v1
    }

Indirect function calls use a signature declared in the preample.

.. inst:: SIG = signature signature

    Declare a function signature for use with indirect calls.

    :arg signature: Function signature. See :token:`signature`.
    :result SIG: A signature identifier.

.. inst:: a, b, ... = call_indirect SIG, x(args...)

    Indirect function call.

    :arg SIG: A function signature identifier declared with :inst:`signature`.
    :arg iPtr x: The address of the function to call.
    :arg args...: Function arguments matching SIG.
    :result a,b,...: Return values matching SIG.

.. todo:: Define safe indirect function calls.

    The :inst:`call_indirect` instruction is dangerous to use in a sandboxed
    environment since it is not easy to verify the callee address.
    We need a table-driven indirect call instruction, similar to
    :inst:`br_table`.



Operations
==========



Special operations
------------------

.. inst:: a = iconst n

    Integer constant.

.. inst:: a = fconst n

    Floating point constant.

.. inst:: a = vconst n

    Vector constant (floating point or integer).

.. inst:: a = select c, x, y

    Conditional select.

    :arg bool c: Controlling flag.
    :arg T x: Value to return when ``c`` is true.
    :arg T y: Value to return when ``c`` is false. Must be same type as ``x``.
    :rtype: T. Same type as ``x`` and ``y``.

    This instruction selects whole values. Use :inst:`vselect` for lane-wise
    selection.

Vector operations
-----------------

.. inst:: a  = vselect c, x, y

    Vector lane select.

    Select lanes from ``x`` or ``y`` controlled by the lanes of the boolean
    vector ``c``.

    :arg boolx%N c: Controlling flag vector.
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
------------------

.. inst:: a = icmp cond, x, y

    Integer comparison.

    :param cond: Condition code determining how ``x`` and ``y`` are compared.
    :param x, y: Integer scalar or vector values of the same type.
    :rtype: :type:`bool` or :type:`boolxN` with the same number of lanes as
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
------------------

.. inst:: a = and x, y

    Bitwise and.

    :rtype: bool, iB, iBxN, fBxN?

.. inst:: a = or x, y

    Bitwise or.

    :rtype: bool, iB, iBxN, fBxN?

.. inst:: a = xor x, y

    Bitwise xor.

    :rtype: bool, iB, iBxN, fBxN?

.. inst:: a = not x

    Bitwise not.

    :rtype: bool, iB, iBxN, fBxN?

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
-------------------------

.. inst:: a = fcmp cond, x, y

    Floating point comparison.

    :param cond: Condition code determining how ``x`` and ``y`` are compared.
    :param x, y: Floating point scalar or vector values of the same type.
    :rtype: :type:`bool` or :type:`boolxN` with the same number of lanes as
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
---------------------

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

Glossary
========

.. glossary::

    function signature
        A function signature describes how to call a function. It consists of:

        - The calling convention.
        - The number of arguments and return values. (Functions can return
          multiple values.)
        - Type and flags of each argument.
        - Type and flags of each return value.

        Not all function atributes are part of the signature. For example, a
        function that never returns could be marked as ``noreturn``, but that
        is not necessary to know when calling it, so it is just an attribute,
        and not part of the signature.

    function preample
        A list of declarations of entities that are used by the function body.
        Some of the entities that can be declared in the preample are:

        - Local variables.
        - Functions that are called directly.
        - Function signatures for indirect function calls.
        - Function flags and attributes that are not part of the signature.

    function body
        The extended basic blocks which contain all the executable code in a
        function. The function body follows the function preample.

    basic block
        A maximal sequence of instructions that can only be entered from the
        top, and that contains no branch or terminator instructions except for
        the last instruction.

    extended basic block
    EBB
        A maximal sequence of instructions that can only be entered from the
        top, and that contains no :term:`terminator instruction`\s except for
        the last one. An EBB can contain conditional branches that can fall
        through to the following instructions in the block, but only the first
        instruction in the EBB can be a branch target.

        The last instrution in an EBB must be a :term:`terminator instruction`,
        so execion cannot flow through to the next EBB in the function. (But
        there may be a branch to the next EBB.)

        Note that some textbooks define an EBB as a maximal *subtree* in the
        control flow graph where only the root can be a join node. This
        definition is not equivalent to Cretonne EBBs.

    terminator instruction
        A control flow instruction that unconditionally directs the flow of
        execution somewhere else. Execution never continues at the instruction
        following a terminator instruction.

        The basic terminator instructions are :inst:`br`, :inst:`return`, and
        :inst:`trap`. Conditional branches and instructions that trap
        conditionally are not terminator instructions.

    entry block
        The :term:`EBB` that is executed first in a function. Currently, a
        Cretonne function must have exactly one entry block. The types of the
        entry block arguments must match the types of arguments in the function
        signature.

    stack slot
        A fixed size memory allocation in the current function's activation
        frame. Also called a local variable.

    local SSA form
        A restricted version of SSA form where all values are defined and used
        in the same EBB. A function is in local SSA form iff it is in SSA form
        and:

        - No branches pass arguments to their target EBB.
        - Only the entry EBB may have arguments.

        This also implies that there are no branches to the entry EBB.

        Local SSA form is easy to generate and fast to verify. It passes data
        between EBBs by using stack slots.
