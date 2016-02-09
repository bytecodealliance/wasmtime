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

.. autoctontype:: bool

Integer types
-------------

Integer values have a fixed size and can be interpreted as either signed or
unsigned. Some instructions will interpret an operand as a signed or unsigned
number, others don't care.

.. autoctontype:: i8
.. autoctontype:: i16
.. autoctontype:: i32
.. autoctontype:: i64

Floating point types
--------------------

The floating point types have the IEEE semantics that are supported by most
hardware. There is no support for higher-precision types like quads or
double-double formats.

.. autoctontype:: f32
.. autoctontype:: f64

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

Pseudo-types and type classes
-----------------------------

These are not concrete types, but convenient names uses to refer to real types
in this reference.

.. type:: iPtr

    A Pointer-sized integer.

    This is either :type:`i32`, or :type:`i64`, depending on whether the target
    platform has 32-bit or 64-bit pointers.

.. type:: iB

    Any of the scalar integer types :type:`i8` -- :type:`i64`.

.. type:: Int

    Any scalar *or vector* integer type: :type:`iB` or :type:`iBxN`.

.. type:: fB

    Either of the floating point scalar types: :type:`f32` or :type:`f64`.

.. type:: Float

    Any scalar *or vector* floating point type: :type:`fB` or :type:`fBxN`.

.. type:: %Tx%N

    Any SIMD vector type.

.. type:: Mem

    Any type that can be stored in memory: :type:`Int` or :type:`Float`.

.. type:: Logic

    Either :type:`bool` or :type:`boolxN`.

.. type:: Testable

    Either :type:`bool` or :type:`iN`.

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

    :arg Testable x: Value to test.
    :arg EBB: Destination extended basic block.
    :arg args...: Arguments passed to EBB.
    :result: None.

.. inst:: brnz x, EBB(args...)

    Branch when non-zero.

    If ``x`` is a :type:`bool` value, take the branch when ``x`` is true. If
    ``x`` is an integer value, take the branch when ``x != 0``.

    :arg Testable x: Value to test.
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

    :arg Testable x: Value to test.
    :result: None.

.. inst:: trapnz x

    Trap when non-zero.

    if ``x`` is zero, execution continues at the following instruction.

    :arg Testable x: Value to test.
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


Memory
======

Cretonne provides fully general :inst:`load` and :inst:`store` instructions for
accessing memory. However, it can be very complicated to verify the safety of
general loads and stores when compiling code for a sandboxed environment, so
Cretonne also provides more restricted memory operations that are always safe.

.. inst:: a = load p, Offset, Flags...

    Load from memory at ``p + Offset``.

    This is a polymorphic instruction that can load any value type which has a
    memory representation (i.e., everything except :type:`bool` and boolean
    vectors).

    :arg iPtr p: Base address.
    :arg Offset: Immediate signed offset.
    :flag align(N): Expected alignment of ``p + Offset``. Power of two.
    :flag aligntrap: Always trap if the memory access is misaligned.
    :result T a: Loaded value.

.. inst:: store x, p, Offset, Flags...

    Store ``x`` to memory at ``p + Offset``.

    This is a polymorphic instruction that can store any value type with a
    memory representation.

    :arg T x: Value to store.
    :arg iPtr p: Base address.
    :arg Offset: Immediate signed offset.
    :flag align(N): Expected alignment of ``p + Offset``. Power of two.
    :flag aligntrap: Always trap if the memory access is misaligned.

Loads and stores are *misaligned* if the resultant address is not a multiple of
the expected alignment. Depending on the target architecture, misaligned memory
accesses may trap, or they may work. Sometimes, operating systems catch
alignment traps and emulate the misaligned memory access.

On target architectures like x86 that don't check alignment, Cretonne expands
the aligntrap flag into a conditional trap instruction::

    v5 = load.i32 v1, 4, align(4), aligntrap
    ; Becomes:
    v10 = and_imm v1, 3
    trapnz v10
    v5 = load.i32 v1, 4


Local variables
---------------

One set of restricted memory operations access the current function's stack
frame. The stack frame is divided into fixed-size stack slots that are
allocated in the :term:`function preample`. Stack slots are not typed, they
simply represent a contiguous sequence of bytes in the stack frame.

.. inst:: SS = stack_slot Bytes, Flags...

    Allocate a stack slot in the preample.

    If no alignment is specified, Cretonne will pick an appropriate alignment
    for the stack slot based on its size and access patterns.

    :arg Bytes: Stack slot size on bytes.
    :flag align(N): Request at least N bytes alignment.
    :result SS: Stack slot index.

.. inst:: a = stack_load SS, Offset

    Load a value from a stack slot at the constant offset.

    This is a polymorphic instruction that can load any value type which has a
    memory representation.

    The offset is an immediate constant, not an SSA value. The memory access
    cannot go out of bounds, i.e. ``sizeof(a) + Offset <= sizeof(SS)``.

    :arg SS: Stack slot declared with :inst:`stack_slot`.
    :arg Offset: Immediate non-negative offset.
    :result T a: Value loaded.

.. inst:: stack_store x, SS, Offset

    Store a value to a stack slot at a constant offset.

    This is a polymorphic instruction that can store any value type with a
    memory representation.

    The offset is an immediate constant, not an SSA value. The memory access
    cannot go out of bounds, i.e. ``sizeof(a) + Offset <= sizeof(SS)``.

    :arg T x: Value to be stored.
    :arg SS: Stack slot declared with :inst:`stack_slot`.
    :arg Offset: Immediate non-negative offset.

The dedicated stack access instructions are easy ofr the compiler to reason
about because stack slots and offsets are fixed at compile time. For example,
the alignment of these stack memory accesses can be inferred from the offsets
and stack slot alignments.

It can be necessary to escape from the safety of the restricted instructions by
taking the address of a stack slot.

.. inst:: a = stack_addr SS, Offset

    Get the address of a stack slot.

    Compute the absolute address of a byte in a stack slot. The offset must
    refer to a byte inside the stack slot: ``0 <= Offset < sizeof(SS)``.

    :arg SS: Stack slot declared with :inst:`stack_slot`.
    :arg Offset: Immediate non-negative offset.
    :result iPtr a: Address.

The :inst:`stack_addr` instruction can be used to macro-expand the stack access
instructions before instruction selection::

    v1 = stack_load.f64 ss3, 16
    ; Expands to:
    v9 = stack_addr ss3, 16
    v1 = load.f64 v9

Heaps
-----

Code compiled from WebAssembly or asm.js runs in a sandbox where it can't access
all process memory. Instead, it is given a small set of memory areas to work
in, and all accesses are bounds checked. Cretonne models this through the
concept of *heaps*.

A heap is declared in the function preample and can be accessed with restricted
instructions that trap on out-of-bounds accesses. Heap addresses can be smaller
than the native pointer size, for example unsigned :type:`i32` offsets on a
64-bit architecture.

.. inst:: H = heap Name

    Declare a heap in the function preample.

    This doesn't allocate memory, it just retrieves a handle to a sandbox from
    the runtime environment.

    :arg Name: String identifying the heap in the runtime environment.
    :result H: Heap identifier.

.. inst:: a = heap_load H, p, Offset

    Load a value at the address ``p + Offset`` in the heap H.

    Trap if the heap access would be out of bounds.

    :arg H: Heap identifier created by :inst:`heap`.
    :arg iN p: Unsigned base address in heap.
    :arg Offset: Immediate signed offset.
    :flag align(N): Expected alignment of ``p + Offset``. Power of two.
    :flag aligntrap: Always trap if the memory access is misaligned.
    :result T a: Loaded value.

.. inst:: a = heap_store H, x, p, Offset

    Store a value at the address ``p + Offset`` in the heap H.

    Trap if the heap access would be out of bounds.

    :arg H: Heap indetifier created by :inst:`heap`.
    :arg T x: Value to be stored.
    :arg iN p: Unsigned base address in heap.
    :arg Offset: Immediate signed offset.
    :flag align(N): Expected alignment of ``p + Offset``. Power of two.
    :flag aligntrap: Always trap if the memory access is misaligned.

When optimizing heap accesses, Cretonne may separate the heap bounds checking
and address computations from the memory accesses.

.. inst:: a = heap_addr H, p, Size

    Bounds check and compute absolute address of heap memory.

    Verify that the address range ``p .. p + Size - 1`` is valid in the heap H,
    and trap if not.

    Convert the heap-relative address in ``p`` to a real absolute address and
    return it.

    :arg H: Heap identifier created by :inst:`heap`.
    :arg iN p: Unsigned base address in heap.
    :arg Size: Immediate unsigned byte count for range to verify.
    :result iPtr a: Absolute address corresponding to ``p``.

A small example using heaps::

    function vdup(i32, i32) {
        h1 = heap "main"

    entry ebb1(v1: i32, v2: i32):
        v3 = heap_load.i32x4 h1, v1, 0
        v4 = heap_addr h1, v2, 32      ; Shared range check for two stores.
        store v3, v4, 0
        store v3, v4, 16
        return
    }

The final expansion of the :inst:`heap_addr` range check and address conversion
depends on the runtime environment.


Operations
==========

The remaining instruction set is mostly arithmetic.

A few instructions have variants that take immediate operands (e.g.,
:inst:`and` / :inst:`and_imm`), but in general an instruction is required to
load a constant into an SSA value.

.. inst:: a = iconst N

    Integer constant.

    Create a scalar integer SSA value with an immediate constant value, or an
    integer vector where all the lanes have the same value.

    :result Int a: Constant value.

.. inst:: a = fconst N

    Floating point constant.

    Create a :type:`f32` or :type:`f64` SSA value with an immediate constant
    value, or a floating point vector where all the lanes have the same value.

    :result Float a: Constant value.

.. inst:: a = vconst N

    Vector constant (floating point or integer).

    Create a SIMD vector value where the lanes don't have to be identical.

    :result TxN a: Constant value.

.. inst:: a = select c, x, y

    Conditional select.

    :arg bool c: Controlling flag.
    :arg T x: Value to return when ``c`` is true.
    :arg T y: Value to return when ``c`` is false. Must be same type as ``x``.
    :result T a: Same type as ``x`` and ``y``.

    This instruction selects whole values. Use :inst:`vselect` for lane-wise
    selection.

Vector operations
-----------------

.. inst:: a  = vselect c, x, y

    Vector lane select.

    Select lanes from ``x`` or ``y`` controlled by the lanes of the boolean
    vector ``c``.

    :arg boolxN c: Controlling flag vector.
    :arg TxN x: Vector with lanes selected by the true lanes of ``c``.
              Must be a vector type with the same number of lanes as ``c``.
    :arg TxN y: Vector with lanes selected by the false lanes of ``c``.
              Must be same type as ``x``.
    :result TxN a: Same type as ``x`` and ``y``.

.. inst:: a = vbuild x, y, z, ...

    Vector build.

    Build a vector value from the provided lanes.

.. inst:: a = splat x

    Vector splat.

    Return a vector whose lanes are all ``x``.

    :arg T x: Scalar value to be replicated.
    :result TxN a: Vector with identical lanes.

.. inst:: a = insertlane x, Idx, y

    Insert ``y`` as lane ``Idx`` in x.

    The lane index, ``Idx``, is an immediate value, not an SSA value. It must
    indicate a valid lane index for the type of ``x``.

    :arg TxN x: Vector to modify.
    :arg Idx: Lane index smaller than N.
    :arg T y: New lane value.
    :result TxN y: Updated vector.

.. inst:: a = extractlane x, Idx

    Extract lane ``Idx`` from ``x``.

    The lane index, ``Idx``, is an immediate value, not an SSA value. It must
    indicate a valid lane index for the type of ``x``.

    :arg TxN x: Source vector
    :arg Idx: Lane index
    :result T a: Lane value.

Integer operations
------------------

.. inst:: a = icmp Cond, x, y

    Integer comparison.

    :arg Cond: Condition code determining how ``x`` and ``y`` are compared.
    :arg Int x: First value to compare.
    :arg Int y: Second value to compare.
    :result Logic a: With the same number of lanes as ``x`` and ``y``.

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

    Polymorphic over all integer types (vector and scalar).

.. inst:: a = iadd_imm x, Imm

    Add immediate integer.

    Same as :inst:`iadd`, but one operand is an immediate constant.

    :arg iN x: Dynamic addend.
    :arg Imm: Immediate addend.

    Polymorphic over all scalar integer types.

.. inst:: a = isub x, y

    Wrapping integer subtraction: :math:`a := x - y \pmod{2^B}`. This
    instruction does not depend on the signed/unsigned interpretation of the
    operands.

    Polymorphic over all integer types (vector and scalar).

.. inst:: a = isub_imm Imm, x

    Immediate subtraction.

    Also works as integer negation when :math:`Imm = 0`. Use :inst:`iadd_imm` with a
    negative immediate operand for the reverse immediate subtraction.

    :arg Imm: Immediate minuend.
    :arg iN x: Dynamic subtrahend.

    Polymorphic over all scalar integer types.

.. todo:: Integer overflow arithmetic

    Add instructions for add with carry out / carry in and so on. Enough to
    implement larger integer types efficiently. It should also be possible to
    legalize :type:`i64` arithmetic to terms of :type:`i32` operations.

.. inst:: a = imul x, y

    Wrapping integer multiplication: :math:`a := x y \pmod{2^B}`. This
    instruction does not depend on the signed/unsigned interpretation of the
    operands.

    Polymorphic over all integer types (vector and scalar).

.. inst:: a = imul_imm x, Imm

    Integer multiplication by immediate constant.

    Polymorphic over all scalar integer types.

.. todo:: Larger multiplication results.

    For example, ``smulx`` which multiplies :type:`i32` operands to produce a
    :type:`i64` result. Alternatively, ``smulhi`` and ``smullo`` pairs.

.. inst:: a = udiv x, y

    Unsigned integer division: :math:`a := \lfloor {x \over y} \rfloor`. This
    operation traps if the divisor is zero.

.. inst:: a = udiv_imm x, Imm

    Unsigned integer division by an immediate constant.

    This instruction never traps because a divisor of zero is not allowed.

.. inst:: a = sdiv x, y

    Signed integer division rounded toward zero: :math:`a := sign(xy) \lfloor
    {|x| \over |y|}\rfloor`. This operation traps if the divisor is zero, or if
    the result is not representable in :math:`B` bits two's complement. This only
    happens when :math:`x = -2^{B-1}, y = -1`.

.. inst:: a = sdiv_imm x, Imm

    Signed integer division by an immediate constant.

    This instruction never traps because a divisor of -1 or 0 is not allowed.

.. inst:: a = urem x, y

    Unsigned integer remainder.

    This operation traps if the divisor is zero.

.. inst:: a = urem_imm x, Imm

    Unsigned integer remainder with immediate divisor.

    This instruction never traps because a divisor of zero is not allowed.

.. inst:: a = srem x, y

    Signed integer remainder.

    This operation traps if the divisor is zero.

    .. todo:: Integer remainder vs modulus.

        Clarify whether the result has the sign of the divisor or the dividend.
        Should we add a ``smod`` instruction for the case where the result has
        the same sign as the divisor?

.. inst:: a = srem_imm x, Imm

    Signed integer remainder with immediate divisor.

    This instruction never traps because a divisor of 0 or -1 is not allowed.

.. todo:: Minimum / maximum.

    NEON has ``smin``, ``smax``, ``umin``, and ``umax`` instructions. We should
    replicate those for both scalar and vector integer types. Even if the
    target ISA doesn't have scalar operations, these are good pattern matching
    targets.

.. todo:: Saturating arithmetic.

    Mostly for SIMD use, but again these are good patterns for contraction.
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

    :arg T x: Integer value to be rotated.
    :arg iN y: Number of bits to shift. Any scalar integer type, not necessarily
               the same type as ``x``.
    :rtype: Same type as ``x``.

.. inst:: a = rotr x, y

    Rotate right.

    Rotate the bits in ``x`` by ``y`` places.

    :arg T x: Integer value to be rotated.
    :arg iN y: Number of bits to shift. Any scalar integer type, not necessarily
               the same type as ``x``.
    :rtype: Same type as ``x``.

.. inst:: a = ishl x, y

    Integer shift left. Shift the bits in ``x`` towards the MSB by ``y``
    places. Shift in zero bits to the LSB.

    The shift amount is masked to the size of ``x``.

    :arg T x: Integer value to be shifted.
    :arg iN y: Number of bits to shift. Any scalar integer type, not necessarily
               the same type as ``x``.
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

    :arg T x: Integer value to be shifted.
    :arg iN y: Number of bits to shift. Can be any scalar integer type, not
               necessarily the same type as ``x``.
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

    :arg T x: Integer value to be shifted.
    :arg iN y: Number of bits to shift. Can be any scalar integer type, not
               necessarily the same type as ``x``.
    :rtype: Same type as ``x``.

    .. todo:: Add ``sshr_imm`` variant with an immediate ``y``.

.. inst:: a = clz x

    Count leading zero bits.

    :arg x: Integer value.
    :rtype: :type:`i8`

    Starting from the MSB in ``x``, count the number of zero bits before
    reaching the first one bit. When ``x`` is zero, returns the size of x in
    bits.

.. inst:: a = cls x

    Count leading sign bits.

    :arg x: Integer value.
    :rtype: :type:`i8`

    Starting from the MSB after the sign bit in ``x``, count the number of
    consecutive bits identical to the sign bit. When ``x`` is 0 or -1, returns
    one less than the size of x in bits.

.. inst:: a = ctz x

    Count trailing zeros.

    :arg x: Integer value.
    :rtype: :type:`i8`

    Starting from the LSB in ``x``, count the number of zero bits before
    reaching the first one bit. When ``x`` is zero, returns the size of x in
    bits.

.. inst:: a = popcnt x

    Population count

    :arg x: Integer value.
    :rtype: :type:`i8`

    Count the number of one bits in ``x``.


Floating point operations
-------------------------

These operations generally follow IEEE 754-2008 semantics.

.. inst:: a = fcmp Cond, x, y

    Floating point comparison.

    :arg Cond: Condition code determining how ``x`` and ``y`` are compared.
    :arg x,y: Floating point scalar or vector values of the same type.
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

    :result: ``x`` with its sign bit inverted.

    Note that this is a pure bitwise operation.

.. inst:: fabs x

    Floating point absolute value.

    :result: ``x`` with its sign bit cleared.

    Note that this is a pure bitwise operation.

.. inst::  a = fcopysign x, y

    Floating point copy sign.

    :result: ``x`` with its sign changed to that of ``y``.

    Note that this is a pure bitwise operation. The sign bit from ``y`` is
    copied to the sign bit of ``x``.

.. inst:: a = fmul x, y
.. inst:: a = fdiv x, y
.. inst:: a = fmin x, y
.. inst:: a = fminnum x, y
.. inst:: a = fmax x, y
.. inst:: a = fmaxnum x, y

.. inst:: a = ceil x

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
