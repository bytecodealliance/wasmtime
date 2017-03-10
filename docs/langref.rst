***************************
Cretonne Language Reference
***************************

.. default-domain:: cton
.. highlight:: cton

The Cretonne intermediate language (:term:`IL`) has two equivalent
representations: an *in-memory data structure* that the code generator library
is using, and a *text format* which is used for test cases and debug output.
Files containing Cretonne textual IL have the ``.cton`` filename extension.

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
Then follows the :term:`function preamble` which declares a number of entities
that can be referenced inside the function. In the example above, the preamble
declares a single local variable, ``ss1``.

After the preamble follows the :term:`function body` which consists of
:term:`extended basic block`\s, the first of which is the :term:`entry block`.
Every EBB ends with a :term:`terminator instruction`, so execution can never
fall through to the next EBB without an explicit branch.

A ``.cton`` file consists of a sequence of independent function definitions:

.. productionlist::
    function_list : { function }
    function      : function_spec "{" preamble function_body "}"
    function_spec : "function" function_name signature
    preamble      : { preamble_decl }
    function_body : { extended_basic_block }

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
into Cretonne :term:`IL` contains multiple assignments to the same variables.
Such variables can be presented to Cretonne as :term:`stack slot`\s instead.
Stack slots are accessed with the :inst:`stack_store` and :inst:`stack_load`
instructions which behave more like variable accesses in a typical programming
language. Cretonne can perform the necessary data-flow analysis to convert stack
slots to SSA form.

.. _value-types:

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

.. type:: intcc

    An integer condition code. See the :inst:`icmp` instruction for details.

.. type:: floatcc

    A floating point condition code. See the :inst:`fcmp` instruction for details.

The two IEEE floating point immediate types :type:`ieee32` and :type:`ieee64`
are displayed as hexadecimal floating point literals in the textual :term:`IL`
format. Decimal floating point literals are not allowed because some computer
systems can round differently when converting to binary. The hexadecimal
floating point format is mostly the same as the one used by C99, but extended
to represent all NaN bit patterns:

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

.. autoinst:: jump
.. autoinst:: brz
.. autoinst:: brnz
.. autoinst:: br_table

.. inst:: JT = jump_table EBB0, EBB1, ..., EBBn

    Declare a jump table in the :term:`function preamble`.

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

.. autoinst:: trap
.. autoinst:: trapz
.. autoinst:: trapnz


Function calls
==============

A function call needs a target function and a :term:`function signature`. The
target function may be determined dynamically at runtime, but the signature
must be known when the function call is compiled. The function signature
describes how to call the function, including arguments, return values, and the
calling convention:

.. productionlist::
    signature : "(" [arglist] ")" ["->" retlist] [call_conv]
    arglist   : arg { "," arg }
    retlist   : arglist
    arg       : type { flag }
    flag      : "uext" | "sext" | "inreg"
    callconv  : `string`

Arguments and return values have flags whose meaning is mostly target
dependent. They make it possible to call native functions on the target
platform. When calling other Cretonne functions, the flags are not necessary.

Functions that are called directly must be declared in the :term:`function
preamble`:

.. inst:: FN = function NAME signature

    Declare a function so it can be called directly.

    :arg NAME: Name of the function, passed to the linker for resolution.
    :arg signature: Function signature. See below.
    :result FN: A function identifier that can be used with :inst:`call`.

.. autoinst:: call
.. autoinst:: x_return
.. autoinst:: return_reg

This simple example illustrates direct function calls and signatures::

    function gcd(i32 uext, i32 uext) -> i32 uext "C" {
        fn1 = function divmod(i32 uext, i32 uext) -> i32 uext, i32 uext

    ebb1(v1: i32, v2: i32):
        brz v2, ebb2
        v3, v4 = call fn1(v1, v2)
        br ebb1(v2, v4)

    ebb2:
        return v1
    }

Indirect function calls use a signature declared in the preamble.

.. inst:: SIG = signature signature

    Declare a function signature for use with indirect calls.

    :arg signature: Function signature. See :token:`signature`.
    :result SIG: A signature identifier.

.. autoinst:: call_indirect

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
the `aligntrap` flag into a conditional trap instruction::

    v5 = load.i32 v1, 4, align(4), aligntrap
    ; Becomes:
    v10 = and_imm v1, 3
    trapnz v10
    v5 = load.i32 v1, 4


Local variables
---------------

One set of restricted memory operations access the current function's stack
frame. The stack frame is divided into fixed-size stack slots that are
allocated in the :term:`function preamble`. Stack slots are not typed, they
simply represent a contiguous sequence of bytes in the stack frame.

.. inst:: SS = stack_slot Bytes, Flags...

    Allocate a stack slot in the preamble.

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

The dedicated stack access instructions are easy for the compiler to reason
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

A heap is declared in the function preamble and can be accessed with restricted
instructions that trap on out-of-bounds accesses. Heap addresses can be smaller
than the native pointer size, for example unsigned :type:`i32` offsets on a
64-bit architecture.

.. inst:: H = heap Name

    Declare a heap in the function preamble.

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

    :arg H: Heap identifier created by :inst:`heap`.
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

    ebb1(v1: i32, v2: i32):
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

.. autoinst:: select

Constant materialization
------------------------

.. autoinst:: iconst
.. autoinst:: f32const
.. autoinst:: f64const
.. autoinst:: vconst

Live range splitting
--------------------

Cretonne's register allocator assigns each SSA value to a register or a spill
slot on the stack for its entire live range. Since the live range of an SSA
value can be quite large, it is sometimes beneficial to split the live range
into smaller parts.

A live range is split by creating new SSA values that are copies or the
original value or each other. The copies are created by inserting :inst:`copy`,
:inst:`spill`, or :inst:`fill` instructions, depending on whether the values
are assigned to registers or stack slots.

This approach permits SSA form to be preserved throughout the register
allocation pass and beyond.

.. autoinst:: copy
.. autoinst:: spill
.. autoinst:: fill

Vector operations
-----------------

.. autoinst:: vsplit
.. autoinst:: vconcat
.. autoinst:: vselect
.. autoinst:: splat
.. autoinst:: insertlane
.. autoinst:: extractlane

Integer operations
------------------

.. autoinst:: icmp
.. autoinst:: iadd
.. autoinst:: iadd_imm
.. autoinst:: iadd_cin
.. autoinst:: iadd_cout
.. autoinst:: iadd_carry
.. autoinst:: isub
.. autoinst:: irsub_imm
.. autoinst:: isub_bin
.. autoinst:: isub_bout
.. autoinst:: isub_borrow

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
.. autoinst:: band_imm
.. autoinst:: bor
.. autoinst:: bor_imm
.. autoinst:: bxor
.. autoinst:: bxor_imm
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
.. autoinst:: rotl_imm
.. autoinst:: rotr
.. autoinst:: rotr_imm
.. autoinst:: ishl
.. autoinst:: ishl_imm
.. autoinst:: ushr
.. autoinst:: ushr_imm
.. autoinst:: sshr
.. autoinst:: sshr_imm

The bit-counting instructions below are scalar only.

.. autoinst:: clz
.. autoinst:: cls
.. autoinst:: ctz
.. autoinst:: popcnt

Floating point operations
-------------------------

These operations generally follow IEEE 754-2008 semantics.

.. autoinst:: fcmp
.. autoinst:: fadd
.. autoinst:: fsub
.. autoinst:: fmul
.. autoinst:: fdiv
.. autoinst:: sqrt
.. autoinst:: fma

Sign bit manipulations
~~~~~~~~~~~~~~~~~~~~~~

The sign manipulating instructions work as bitwise operations, so they don't
have special behavior for signaling NaN operands. The exponent and trailing
significand bits are always preserved.

.. autoinst:: fneg
.. autoinst:: fabs
.. autoinst:: fcopysign

Minimum and maximum
~~~~~~~~~~~~~~~~~~~

These instructions return the larger or smaller of their operands. They differ
in their handling of quiet NaN inputs. Note that signaling NaN operands always
cause a NaN result.

When comparing zeroes, these instructions behave as if :math:`-0.0 < 0.0`.

.. autoinst:: fmin
.. autoinst:: fminnum
.. autoinst:: fmax
.. autoinst:: fmaxnum

Rounding
~~~~~~~~

These instructions round their argument to a nearby integral value, still
represented as a floating point number.

.. autoinst:: ceil
.. autoinst:: floor
.. autoinst:: trunc
.. autoinst:: nearest

Conversion operations
---------------------

.. autoinst:: bitcast
.. autoinst:: ireduce
.. autoinst:: uextend
.. autoinst:: sextend
.. autoinst:: fpromote
.. autoinst:: fdemote
.. autoinst:: fcvt_to_uint
.. autoinst:: fcvt_to_sint
.. autoinst:: fcvt_from_uint
.. autoinst:: fcvt_from_sint

Legalization operations
-----------------------

These instructions are used as helpers when legalizing types and operations for
the target ISA.

.. autoinst:: isplit_lohi
.. autoinst:: iconcat_lohi

Base instruction group
======================

All of the shared instructions are part of the :instgroup:`base` instruction
group.

.. autoinstgroup:: base.instructions.GROUP

Target ISAs may define further instructions in their own instruction groups.

Implementation limits
=====================

Cretonne's intermediate representation imposes some limits on the size of
functions and the number of entities allowed. If these limits are exceeded, the
implementation will panic.

Number of instructions in a function
    At most :math:`2^{31} - 1`.

Number of EBBs in a function
    At most :math:`2^{31} - 1`.

    Every EBB needs at least a terminator instruction anyway.

Number of secondary values in a function
    At most :math:`2^{31} - 1`.

    Secondary values are any SSA values that are not the first result of an
    instruction.

Other entities declared in the preamble
    At most :math:`2^{32} - 1`.

    This covers things like stack slots, jump tables, external functions, and
    function signatures, etc.

Number of arguments to an EBB
    At most :math:`2^{16}`.

Number of arguments to a function
    At most :math:`2^{16}`.

    This follows from the limit on arguments to the entry EBB. Note that
    Cretonne may add a handful of ABI register arguments as function signatures
    are lowered. This is for representing things like the link register, the
    incoming frame pointer, and callee-saved registers that are saved in the
    prologue.

Size of function call arguments on the stack
    At most :math:`2^{32} - 1` bytes.

    This is probably not possible to achieve given the limit on the number of
    arguments, except by requiring extremely large offsets for stack arguments.

Glossary
========

.. glossary::

    intermediate language
    IL
        The language used to describe functions to Cretonne. This reference
        describes the syntax and semantics of the Cretonne IL. The IL has two
        forms: Textual and an in-memory intermediate representation
        (:term:`IR`).

    intermediate representation
    IR
        The in-memory representation of :term:`IL`. The data structures
        Cretonne uses to represent a program internally are called the
        intermediate representation. Cretonne's IR can be converted to text
        losslessly.

    function signature
        A function signature describes how to call a function. It consists of:

        - The calling convention.
        - The number of arguments and return values. (Functions can return
          multiple values.)
        - Type and flags of each argument.
        - Type and flags of each return value.

        Not all function attributes are part of the signature. For example, a
        function that never returns could be marked as ``noreturn``, but that
        is not necessary to know when calling it, so it is just an attribute,
        and not part of the signature.

    function preamble
        A list of declarations of entities that are used by the function body.
        Some of the entities that can be declared in the preamble are:

        - Local variables.
        - Functions that are called directly.
        - Function signatures for indirect function calls.
        - Function flags and attributes that are not part of the signature.

    function body
        The extended basic blocks which contain all the executable code in a
        function. The function body follows the function preamble.

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

        The last instruction in an EBB must be a :term:`terminator instruction`,
        so execution cannot flow through to the next EBB in the function. (But
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
        Cretonne function must have exactly one entry block which must be the
        first block in the function. The types of the entry block arguments must
        match the types of arguments in the function signature.

    stack slot
        A fixed size memory allocation in the current function's activation
        frame. Also called a local variable.
