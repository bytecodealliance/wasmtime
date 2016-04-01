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

Boolean types
-------------

Boolean values are either true or false. While this only requires a single bit
to represent, more bits are often used when holding a boolean value in a
register or in memory. The :type:`b1` type represents an abstract boolean
value. It can only exist as an SSA value, it can't be stored in memory or
converted to another type. The larger boolean types can be stored in memory.

.. todo:: Clarify the representation of larger boolean types.

    The multi-bit boolean types can be interpreted in different ways. We could
    declare that zero means false and non-zero means true. This may require
    unwanted normalization code in some places.

    We could specify a fixed encoding like all ones for true. This would then
    lead to undefined behavior if untrusted code uses the multibit booleans
    incorrectly.

    Something like this:

    - External code is not allowed to load/store multi-bit booleans or
      otherwise expose the representation.
    - Each target specifies the exact representation of a multi-bit boolean.

.. autoctontype:: b1
.. autoctontype:: b8
.. autoctontype:: b16
.. autoctontype:: b32
.. autoctontype:: b64

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
(boolean, integer, and floating point). Each scalar value in a SIMD type is
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

.. type:: b1x%N

    A boolean SIMD vector.

    Boolean vectors are used when comparing SIMD vectors. For example,
    comparing two :type:`i32x4` values would produce a :type:`b1x4` result.

    Like the :type:`b1` type, a boolean vector cannot be stored in memory.

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

    Either :type:`b1` or :type:`b1xN`.

.. type:: Testable

    Either :type:`b1` or :type:`iN`.

Immediate operand types
-----------------------

These types are not part of the normal SSA type system. They are used to
indicate the different kinds of immediate operands on an instruction.

.. type:: imm64

    A 64-bit immediate integer. The value of this operand is interpreted as a
    signed two's complement integer. Instruction encodings may limit the valid
    range.

    In the textual format, :type:`imm64` immediates appear as decimal or
    hexadecimal literals using the same syntax as C.

.. type:: ieee32

    A 32-bit immediate floating point number in the IEEE 754-2008 binary32
    interchange format. All bit patterns are allowed.

.. type:: ieee64

    A 64-bit immediate floating point number in the IEEE 754-2008 binary64
    interchange format. All bit patterns are allowed.

.. type:: immvector

    An immediate SIMD vector. This operand supplies all the bits of a SIMD
    type, so it can have different sizes depending on the type produced. The
    bits of the operand are interpreted as if the SIMD vector was loaded from
    memory containing the immediate.

The two IEEE floating point immediate types :type:`ieee32` and :type:`ieee64`
are displayed as hexadecimal floating point literals in the textual IL format.
Decimal floating point literals are not allowed because some computer systems
can round differently when converting to binary. The hexadecimal floating point
format is mostly the same as the one used by C99, but extended to represent all
NaN bit patterns:

Normal numbers
    Compatible with C99: ``-0x1.Tpe`` where ``T`` are the trailing
    significand bits encoded as hexadecimal, and ``e`` is the unbiased exponent
    as a decimal number. :type:`ieee32` has 23 trailing significand bits. They
    are padded with an extra LSB to produce 6 hexadecimal digits. This is not
    necessary for :type:`ieee64` which has 52 trailing significand bits
    forming 13 hexadecimal digits with no padding.

Zeros
    Positive and negative zero are displayed as ``0.0`` and ``-0.0`` respectively.

Subnormal numbers
    Compatible with C99: ``-0x0.Tpemin`` where ``T`` are the trailing
    significand bits encoded as hexadecimal, and ``emin`` is the minimum exponent
    as a decimal number.

Infinities
    Either ``-Inf`` or ``Inf``.

Quiet NaNs
    Quiet NaNs have the MSB of the trailing significand set. If the remaining
    bits of the trailing significand are all zero, the value is displayed as
    ``-NaN`` or ``NaN``. Otherwise, ``-NaN:0xT`` where ``T`` are the trailing
    significand bits encoded as hexadecimal.

Signaling NaNs
    Displayed as ``-sNaN:0xT``.

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

    If ``x`` is a :type:`b1` value, take the branch when ``x`` is false. If
    ``x`` is an integer value, take the branch when ``x = 0``.

    :arg Testable x: Value to test.
    :arg EBB: Destination extended basic block.
    :arg args...: Arguments passed to EBB.
    :result: None.

.. inst:: brnz x, EBB(args...)

    Branch when non-zero.

    If ``x`` is a :type:`b1` value, take the branch when ``x`` is true. If
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
    memory representation.

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
:inst:`band` / :inst:`band_imm`), but in general an instruction is required to
load a constant into an SSA value.

.. autoinst:: iconst
.. autoinst:: f32const
.. autoinst:: f64const
.. autoinst:: vconst

.. inst:: a = select c, x, y

    Conditional select.

    :arg b1 c: Controlling flag.
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

    :arg b1xN c: Controlling flag vector.
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

.. autoinst:: iadd
.. autoinst:: iadd_imm
.. autoinst:: isub
.. autoinst:: isub_imm

.. todo:: Integer overflow arithmetic

    Add instructions for add with carry out / carry in and so on. Enough to
    implement larger integer types efficiently. It should also be possible to
    legalize :type:`i64` arithmetic to terms of :type:`i32` operations.

.. autoinst:: imul
.. autoinst:: imul_imm

.. todo:: Larger multiplication results.

    For example, ``smulx`` which multiplies :type:`i32` operands to produce a
    :type:`i64` result. Alternatively, ``smulhi`` and ``smullo`` pairs.

.. autoinst:: udiv
.. autoinst:: udiv_imm
.. autoinst:: sdiv
.. autoinst:: sdiv_imm
.. autoinst:: urem
.. autoinst:: urem_imm
.. autoinst:: srem
.. autoinst:: srem_imm

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

The bitwise operations and operate on any value type: Integers, floating point
numbers, and booleans. When operating on integer or floating point types, the
bitwise operations are working on the binary representation of the values. When
operating on boolean values, the bitwise operations work as logical operators.

.. autoinst:: band
.. autoinst:: bor
.. autoinst:: bxor
.. autoinst:: bnot

.. todo:: Redundant bitwise operators.

    ARM has instructions like ``bic(x,y) = x & ~y``, ``orn(x,y) = x | ~y``, and
    ``eon(x,y) = x ^ ~y``.

The shift and rotate operations only work on integer types (scalar and vector).
The shift amount does not have to be the same type as the value being shifted.
Only the low `B` bits of the shift amount is significant.

When operating on an integer vector type, the shift amount is still a scalar
type, and all the lanes are shifted the same amount. The shift amount is masked
to the number of bits in a *lane*, not the full size of the vector type.

.. autoinst:: rotl
.. autoinst:: rotr
.. autoinst:: ishl
.. autoinst:: ushr
.. autoinst:: sshr

The bit-counting instructions below are scalar only.

.. autoinst:: clz
.. autoinst:: cls
.. autoinst:: ctz
.. autoinst:: popcnt

Floating point operations
-------------------------

These operations generally follow IEEE 754-2008 semantics.

.. inst:: a = fcmp Cond, x, y

    Floating point comparison.

    :arg Cond: Condition code determining how ``x`` and ``y`` are compared.
    :arg x,y: Floating point scalar or vector values of the same type.
    :rtype: :type:`b1` or :type:`b1xN` with the same number of lanes as
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
