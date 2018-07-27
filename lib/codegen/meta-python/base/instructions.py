"""
Cranelift base instruction set.

This module defines the basic Cranelift instruction set that all targets
support.
"""
from __future__ import absolute_import
from cdsl.operands import Operand, VARIABLE_ARGS
from cdsl.typevar import TypeVar
from cdsl.instructions import Instruction, InstructionGroup
from base.types import f32, f64, b1, iflags, fflags
from base.immediates import imm64, uimm8, uimm32, ieee32, ieee64, offset32
from base.immediates import boolean, intcc, floatcc, memflags, regunit
from base.immediates import trapcode
from base import entities
from cdsl.ti import WiderOrEq
import base.formats  # noqa

GROUP = InstructionGroup("base", "Shared base instruction set")

Int = TypeVar('Int', 'A scalar or vector integer type', ints=True, simd=True)
Bool = TypeVar('Bool', 'A scalar or vector boolean type',
               bools=True, simd=True)
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
Mem = TypeVar(
        'Mem', 'Any type that can be stored in memory',
        ints=True, floats=True, simd=True)
MemTo = TypeVar(
        'MemTo', 'Any type that can be stored in memory',
        ints=True, floats=True, simd=True)

addr = Operand('addr', iAddr)

#
# Control flow
#
c = Operand('c', Testable, doc='Controlling value to test')
Cond = Operand('Cond', intcc)
x = Operand('x', iB)
y = Operand('y', iB)
EBB = Operand('EBB', entities.ebb, doc='Destination extended basic block')
args = Operand('args', VARIABLE_ARGS, doc='EBB arguments')

jump = Instruction(
        'jump', r"""
        Jump.

        Unconditionally jump to an extended basic block, passing the specified
        EBB arguments. The number and types of arguments must match the
        destination EBB.
        """,
        ins=(EBB, args), is_branch=True, is_terminator=True)

fallthrough = Instruction(
        'fallthrough', r"""
        Fall through to the next EBB.

        This is the same as :inst:`jump`, except the destination EBB must be
        the next one in the layout.

        Jumps are turned into fall-through instructions by the branch
        relaxation pass. There is no reason to use this instruction outside
        that pass.
        """,
        ins=(EBB, args), is_branch=True, is_terminator=True)

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

br_icmp = Instruction(
        'br_icmp', r"""
        Compare scalar integers and branch.

        Compare ``x`` and ``y`` in the same way as the :inst:`icmp` instruction
        and take the branch if the condition is true::

            br_icmp ugt v1, v2, ebb4(v5, v6)

        is semantically equivalent to::

            v10 = icmp ugt, v1, v2
            brnz v10, ebb4(v5, v6)

        Some RISC architectures like MIPS and RISC-V provide instructions that
        implement all or some of the condition codes. The instruction can also
        be used to represent *macro-op fusion* on architectures like Intel's.
        """,
        ins=(Cond, x, y, EBB, args), is_branch=True)

f = Operand('f', iflags)

brif = Instruction(
        'brif', r"""
        Branch when condition is true in integer CPU flags.
        """,
        ins=(Cond, f, EBB, args), is_branch=True)

Cond = Operand('Cond', floatcc)
f = Operand('f', fflags)

brff = Instruction(
        'brff', r"""
        Branch when condition is true in floating point CPU flags.
        """,
        ins=(Cond, f, EBB, args), is_branch=True)

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

code = Operand('code', trapcode)
trap = Instruction(
        'trap', r"""
        Terminate execution unconditionally.
        """,
        ins=code, is_terminator=True, can_trap=True)

trapz = Instruction(
        'trapz', r"""
        Trap when zero.

        if ``c`` is non-zero, execution continues at the following instruction.
        """,
        ins=(c, code), can_trap=True)

trapnz = Instruction(
        'trapnz', r"""
        Trap when non-zero.

        if ``c`` is zero, execution continues at the following instruction.
        """,
        ins=(c, code), can_trap=True)

Cond = Operand('Cond', intcc)
f = Operand('f', iflags)

trapif = Instruction(
        'trapif', r"""
        Trap when condition is true in integer CPU flags.
        """,
        ins=(Cond, f, code), can_trap=True)

Cond = Operand('Cond', floatcc)
f = Operand('f', fflags)

trapff = Instruction(
        'trapff', r"""
        Trap when condition is true in floating point CPU flags.
        """,
        ins=(Cond, f, code), can_trap=True)

rvals = Operand('rvals', VARIABLE_ARGS, doc='return values')

x_return = Instruction(
        'return', r"""
        Return from the function.

        Unconditionally transfer control to the calling function, passing the
        provided return values. The list of return values must match the
        function signature's return types.
        """,
        ins=rvals, is_return=True, is_terminator=True)

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

        Note that this is different from WebAssembly's ``call_indirect``; the
        callee is a native address, rather than a table index. For WebAssembly,
        :inst:`table_addr` and :inst:`load` are used to obtain a native address
        from a table.
        """,
        ins=(SIG, callee, args), outs=rvals, is_call=True)

func_addr = Instruction(
        'func_addr', r"""
        Get the address of a function.

        Compute the absolute address of a function declared in the preamble.
        The returned address can be used as a ``callee`` argument to
        :inst:`call_indirect`. This is also a method for calling functions that
        are too far away to be addressable by a direct :inst:`call`
        instruction.
        """,
        ins=FN, outs=addr)

#
# Memory operations
#

SS = Operand('SS', entities.stack_slot)
Offset = Operand('Offset', offset32, 'Byte offset from base address')
x = Operand('x', Mem, doc='Value to be stored')
a = Operand('a', Mem, doc='Value loaded')
p = Operand('p', iAddr)
MemFlags = Operand('MemFlags', memflags)
args = Operand('args', VARIABLE_ARGS, doc='Address arguments')

load = Instruction(
        'load', r"""
        Load from memory at ``p + Offset``.

        This is a polymorphic instruction that can load any value type which
        has a memory representation.
        """,
        ins=(MemFlags, p, Offset), outs=a, can_load=True)

load_complex = Instruction(
        'load_complex', r"""
        Load from memory at ``sum(args) + Offset``.

        This is a polymorphic instruction that can load any value type which
        has a memory representation.
        """,
        ins=(MemFlags, args, Offset), outs=a, can_load=True)

store = Instruction(
        'store', r"""
        Store ``x`` to memory at ``p + Offset``.

        This is a polymorphic instruction that can store any value type with a
        memory representation.
        """,
        ins=(MemFlags, x, p, Offset), can_store=True)

store_complex = Instruction(
        'store_complex', r"""
        Store ``x`` to memory at ``sum(args) + Offset``.

        This is a polymorphic instruction that can store any value type with a
        memory representation.
        """,
        ins=(MemFlags, x, args, Offset), can_store=True)


iExt8 = TypeVar(
        'iExt8', 'An integer type with more than 8 bits',
        ints=(16, 64))
x = Operand('x', iExt8)
a = Operand('a', iExt8)

uload8 = Instruction(
        'uload8', r"""
        Load 8 bits from memory at ``p + Offset`` and zero-extend.

        This is equivalent to ``load.i8`` followed by ``uextend``.
        """,
        ins=(MemFlags, p, Offset), outs=a, can_load=True)

uload8_complex = Instruction(
        'uload8_complex', r"""
        Load 8 bits from memory at ``sum(args) + Offset`` and zero-extend.

        This is equivalent to ``load.i8`` followed by ``uextend``.
        """,
        ins=(MemFlags, args, Offset), outs=a, can_load=True)

sload8 = Instruction(
        'sload8', r"""
        Load 8 bits from memory at ``p + Offset`` and sign-extend.

        This is equivalent to ``load.i8`` followed by ``sextend``.
        """,
        ins=(MemFlags, p, Offset), outs=a, can_load=True)

sload8_complex = Instruction(
        'sload8_complex', r"""
        Load 8 bits from memory at ``sum(args) + Offset`` and sign-extend.

        This is equivalent to ``load.i8`` followed by ``sextend``.
        """,
        ins=(MemFlags, args, Offset), outs=a, can_load=True)

istore8 = Instruction(
        'istore8', r"""
        Store the low 8 bits of ``x`` to memory at ``p + Offset``.

        This is equivalent to ``ireduce.i8`` followed by ``store.i8``.
        """,
        ins=(MemFlags, x, p, Offset), can_store=True)

istore8_complex = Instruction(
        'istore8_complex', r"""
        Store the low 8 bits of ``x`` to memory at ``sum(args) + Offset``.

        This is equivalent to ``ireduce.i8`` followed by ``store.i8``.
        """,
        ins=(MemFlags, x, args, Offset), can_store=True)

iExt16 = TypeVar(
        'iExt16', 'An integer type with more than 16 bits',
        ints=(32, 64))
x = Operand('x', iExt16)
a = Operand('a', iExt16)

uload16 = Instruction(
        'uload16', r"""
        Load 16 bits from memory at ``p + Offset`` and zero-extend.

        This is equivalent to ``load.i16`` followed by ``uextend``.
        """,
        ins=(MemFlags, p, Offset), outs=a, can_load=True)

uload16_complex = Instruction(
        'uload16_complex', r"""
        Load 16 bits from memory at ``sum(args) + Offset`` and zero-extend.

        This is equivalent to ``load.i16`` followed by ``uextend``.
        """,
        ins=(MemFlags, args, Offset), outs=a, can_load=True)

sload16 = Instruction(
        'sload16', r"""
        Load 16 bits from memory at ``p + Offset`` and sign-extend.

        This is equivalent to ``load.i16`` followed by ``sextend``.
        """,
        ins=(MemFlags, p, Offset), outs=a, can_load=True)

sload16_complex = Instruction(
        'sload16_complex', r"""
        Load 16 bits from memory at ``sum(args) + Offset`` and sign-extend.

        This is equivalent to ``load.i16`` followed by ``sextend``.
        """,
        ins=(MemFlags, args, Offset), outs=a, can_load=True)

istore16 = Instruction(
        'istore16', r"""
        Store the low 16 bits of ``x`` to memory at ``p + Offset``.

        This is equivalent to ``ireduce.i16`` followed by ``store.i16``.
        """,
        ins=(MemFlags, x, p, Offset), can_store=True)

istore16_complex = Instruction(
        'istore16_complex', r"""
        Store the low 16 bits of ``x`` to memory at ``sum(args) + Offset``.

        This is equivalent to ``ireduce.i16`` followed by ``store.i16``.
        """,
        ins=(MemFlags, x, args, Offset), can_store=True)

iExt32 = TypeVar(
        'iExt32', 'An integer type with more than 32 bits',
        ints=(64, 64))
x = Operand('x', iExt32)
a = Operand('a', iExt32)

uload32 = Instruction(
        'uload32', r"""
        Load 32 bits from memory at ``p + Offset`` and zero-extend.

        This is equivalent to ``load.i32`` followed by ``uextend``.
        """,
        ins=(MemFlags, p, Offset), outs=a, can_load=True)

uload32_complex = Instruction(
        'uload32_complex', r"""
        Load 32 bits from memory at ``sum(args) + Offset`` and zero-extend.

        This is equivalent to ``load.i32`` followed by ``uextend``.
        """,
        ins=(MemFlags, args, Offset), outs=a, can_load=True)

sload32 = Instruction(
        'sload32', r"""
        Load 32 bits from memory at ``p + Offset`` and sign-extend.

        This is equivalent to ``load.i32`` followed by ``sextend``.
        """,
        ins=(MemFlags, p, Offset), outs=a, can_load=True)

sload32_complex = Instruction(
        'sload32_complex', r"""
        Load 32 bits from memory at ``sum(args) + Offset`` and sign-extend.

        This is equivalent to ``load.i32`` followed by ``sextend``.
        """,
        ins=(MemFlags, args, Offset), outs=a, can_load=True)

istore32 = Instruction(
        'istore32', r"""
        Store the low 32 bits of ``x`` to memory at ``p + Offset``.

        This is equivalent to ``ireduce.i32`` followed by ``store.i32``.
        """,
        ins=(MemFlags, x, p, Offset), can_store=True)

istore32_complex = Instruction(
        'istore32_complex', r"""
        Store the low 32 bits of ``x`` to memory at ``sum(args) + Offset``.

        This is equivalent to ``ireduce.i32`` followed by ``store.i32``.
        """,
        ins=(MemFlags, x, args, Offset), can_store=True)

x = Operand('x', Mem, doc='Value to be stored')
a = Operand('a', Mem, doc='Value loaded')
Offset = Operand('Offset', offset32, 'In-bounds offset into stack slot')

stack_load = Instruction(
        'stack_load', r"""
        Load a value from a stack slot at the constant offset.

        This is a polymorphic instruction that can load any value type which
        has a memory representation.

        The offset is an immediate constant, not an SSA value. The memory
        access cannot go out of bounds, i.e.
        :math:`sizeof(a) + Offset <= sizeof(SS)`.
        """,
        ins=(SS, Offset), outs=a, can_load=True)

stack_store = Instruction(
        'stack_store', r"""
        Store a value to a stack slot at a constant offset.

        This is a polymorphic instruction that can store any value type with a
        memory representation.

        The offset is an immediate constant, not an SSA value. The memory
        access cannot go out of bounds, i.e.
        :math:`sizeof(a) + Offset <= sizeof(SS)`.
        """,
        ins=(x, SS, Offset), can_store=True)

stack_addr = Instruction(
        'stack_addr', r"""
        Get the address of a stack slot.

        Compute the absolute address of a byte in a stack slot. The offset must
        refer to a byte inside the stack slot:
        :math:`0 <= Offset < sizeof(SS)`.
        """,
        ins=(SS, Offset), outs=addr)

#
# Global values.
#

GV = Operand('GV', entities.global_value)

global_value = Instruction(
        'global_value', r"""
        Compute the value of global GV.
        """,
        ins=GV, outs=addr)

# A specialized form of global_value instructions that only handles
# symbolic names.
globalsym_addr = Instruction(
        'globalsym_addr', r"""
        Compute the address of global GV, which is a symbolic name.
        """,
        ins=GV, outs=addr)

#
# WebAssembly bounds-checked heap accesses.
#

HeapOffset = TypeVar('HeapOffset', 'An unsigned heap offset', ints=(32, 64))

H = Operand('H', entities.heap)
p = Operand('p', HeapOffset)
Size = Operand('Size', uimm32, 'Size in bytes')

heap_addr = Instruction(
        'heap_addr', r"""
        Bounds check and compute absolute address of heap memory.

        Verify that the offset range ``p .. p + Size - 1`` is in bounds for the
        heap H, and generate an absolute address that is safe to dereference.

        1. If ``p + Size`` is not greater than the heap bound, return an
           absolute address corresponding to a byte offset of ``p`` from the
           heap's base address.
        2. If ``p + Size`` is greater than the heap bound, generate a trap.
        """,
        ins=(H, p, Size), outs=addr)

#
# WebAssembly bounds-checked table accesses.
#

TableOffset = TypeVar('TableOffset', 'An unsigned table offset', ints=(32, 64))

T = Operand('T', entities.table)
p = Operand('p', TableOffset)
Offset = Operand('Offset', offset32, 'Byte offset from element address')

table_addr = Instruction(
        'table_addr', r"""
        Bounds check and compute absolute address of a table entry.

        Verify that the offset ``p`` is in bounds for the table T, and generate
        an absolute address that is safe to dereference.

        ``Offset`` must be less than the size of a table element.

        1. If ``p`` is not greater than the table bound, return an absolute
           address corresponding to a byte offset of ``p`` from the table's
           base address.
        2. If ``p`` is greater than the table bound, generate a trap.
        """,
        ins=(T, p, Offset), outs=addr)


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
a = Operand('a', f32, doc='A constant f32 scalar value')
f32const = Instruction(
        'f32const', r"""
        Floating point constant.

        Create a :type:`f32` SSA value with an immediate constant value.
        """,
        ins=N, outs=a)

N = Operand('N', ieee64)
a = Operand('a', f64, doc='A constant f64 scalar value')
f64const = Instruction(
        'f64const', r"""
        Floating point constant.

        Create a :type:`f64` SSA value with an immediate constant value.
        """,
        ins=N, outs=a)

N = Operand('N', boolean)
a = Operand('a', Bool, doc='A constant boolean scalar or vector value')
bconst = Instruction(
        'bconst', r"""
        Boolean constant.

        Create a scalar boolean SSA value with an immediate constant value, or
        a boolean vector where all the lanes have the same value.
        """,
        ins=N, outs=a)

#
# Generics.
#

nop = Instruction(
        'nop', r"""
        Just a dummy instruction

        Note: this doesn't compile to a machine code nop
        """)

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

cc = Operand('cc', intcc, doc='Controlling condition code')
flags = Operand('flags', iflags, doc='The machine\'s flag register')

selectif = Instruction(
        'selectif', r"""
        Conditional select, dependent on integer condition codes.
        """,
        ins=(cc, flags, x, y), outs=a)

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
        ins=x, outs=a, can_store=True)

fill = Instruction(
        'fill', r"""
        Load a register value from a stack slot.

        This instruction behaves exactly like :inst:`copy`, but creates a new
        SSA value for the spilled input value.
        """,
        ins=x, outs=a, can_load=True)

src = Operand('src', regunit)
dst = Operand('dst', regunit)

regmove = Instruction(
        'regmove', r"""
        Temporarily divert ``x`` from ``src`` to ``dst``.

        This instruction moves the location of a value from one register to
        another without creating a new SSA value. It is used by the register
        allocator to temporarily rearrange register assignments in order to
        satisfy instruction constraints.

        The register diversions created by this instruction must be undone
        before the value leaves the EBB. At the entry to a new EBB, all live
        values must be in their originally assigned registers.
        """,
        ins=(x, src, dst),
        other_side_effects=True)

copy_special = Instruction(
        'copy_special', r"""
        Copies the contents of ''src'' register to ''dst'' register.

        This instructions copies the contents of one register to another
        register without involving any SSA values. This is used for copying
        special registers, e.g. copying the stack register to the frame
        register in a function prologue.
        """,
        ins=(src, dst),
        other_side_effects=True)

delta = Operand('delta', Int)
adjust_sp_down = Instruction(
    'adjust_sp_down', r"""
    Subtracts ``delta`` offset value from the stack pointer register.

    This instruction is used to adjust the stack pointer by a dynamic amount.
    """,
    ins=(delta,),
    other_side_effects=True)

StackOffset = Operand('Offset', imm64, 'Offset from current stack pointer')
adjust_sp_up_imm = Instruction(
    'adjust_sp_up_imm', r"""
    Adds ``Offset`` immediate offset value to the stack pointer register.

    This instruction is used to adjust the stack pointer, primarily in function
    prologues and epilogues. ``Offset`` is constrained to the size of a signed
    32-bit integer.
    """,
    ins=(StackOffset,),
    other_side_effects=True)

StackOffset = Operand('Offset', imm64, 'Offset from current stack pointer')
adjust_sp_down_imm = Instruction(
    'adjust_sp_down_imm', r"""
    Subtracts ``Offset`` immediate offset value from the stack pointer
    register.

    This instruction is used to adjust the stack pointer, primarily in function
    prologues and epilogues. ``Offset`` is constrained to the size of a signed
    32-bit integer.
    """,
    ins=(StackOffset,),
    other_side_effects=True)

f = Operand('f', iflags)

ifcmp_sp = Instruction(
    'ifcmp_sp', r"""
    Compare ``addr`` with the stack pointer and set the CPU flags.

    This is like :inst:`ifcmp` where ``addr`` is the LHS operand and the stack
    pointer is the RHS.
    """,
    ins=addr, outs=f)

regspill = Instruction(
        'regspill', r"""
        Temporarily divert ``x`` from ``src`` to ``SS``.

        This instruction moves the location of a value from a register to a
        stack slot without creating a new SSA value. It is used by the register
        allocator to temporarily rearrange register assignments in order to
        satisfy instruction constraints.

        See also :inst:`regmove`.
        """,
        ins=(x, src, SS),
        other_side_effects=True)


regfill = Instruction(
        'regfill', r"""
        Temporarily divert ``x`` from ``SS`` to ``dst``.

        This instruction moves the location of a value from a stack slot to a
        register without creating a new SSA value. It is used by the register
        allocator to temporarily rearrange register assignments in order to
        satisfy instruction constraints.

        See also :inst:`regmove`.
        """,
        ins=(x, SS, dst),
        other_side_effects=True)
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

a = Operand('a', b1)
x = Operand('x', iB)
Y = Operand('Y', imm64)

icmp_imm = Instruction(
        'icmp_imm', r"""
        Compare scalar integer to a constant.

        This is the same as the :inst:`icmp` instruction, except one operand is
        an immediate constant.

        This instruction can only compare scalars. Use :inst:`icmp` for
        lane-wise vector comparisons.
        """,
        ins=(Cond, x, Y), outs=a)

f = Operand('f', iflags)
x = Operand('x', iB)
y = Operand('y', iB)

ifcmp = Instruction(
        'ifcmp', r"""
        Compare scalar integers and return flags.

        Compare two scalar integer values and return integer CPU flags
        representing the result.
        """,
        ins=(x, y), outs=f)

ifcmp_imm = Instruction(
        'ifcmp_imm', r"""
        Compare scalar integer to a constant and return flags.

        Like :inst:`icmp_imm`, but returns integer CPU flags instead of testing
        a specific condition code.
        """,
        ins=(x, Y), outs=f)

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

umulhi = Instruction(
        'umulhi', r"""
        Unsigned integer multiplication, producing the high half of a
        double-length result.

        Polymorphic over all scalar integer types, but does not support vector
        types.
        """,
        ins=(x, y), outs=a)

smulhi = Instruction(
        'smulhi', """
        Signed integer multiplication, producing the high half of a
        double-length result.

        Polymorphic over all scalar integer types, but does not support vector
        types.
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
        Signed integer remainder. The result has the sign of the dividend.

        This operation traps if the divisor is zero.
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

        Polymorphic over all scalar integer types, but does not support vector
        types.
        """,
        ins=(x, Y), outs=a)

udiv_imm = Instruction(
        'udiv_imm', """
        Unsigned integer division by an immediate constant.

        This operation traps if the divisor is zero.
        """,
        ins=(x, Y), outs=a)

sdiv_imm = Instruction(
        'sdiv_imm', """
        Signed integer division by an immediate constant.

        This operation traps if the divisor is zero, or if the result is not
        representable in :math:`B` bits two's complement. This only happens
        when :math:`x = -2^{B-1}, Y = -1`.
        """,
        ins=(x, Y), outs=a)

urem_imm = Instruction(
        'urem_imm', """
        Unsigned integer remainder with immediate divisor.

        This operation traps if the divisor is zero.
        """,
        ins=(x, Y), outs=a)

srem_imm = Instruction(
        'srem_imm', """
        Signed integer remainder with immediate divisor.

        This operation traps if the divisor is zero.
        """,
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

band_not = Instruction(
        'band_not', """
        Bitwise and not.

        Computes `x & ~y`.
        """,
        ins=(x, y), outs=a)

bor_not = Instruction(
        'bor_not', """
        Bitwise or not.

        Computes `x | ~y`.
        """,
        ins=(x, y), outs=a)

bxor_not = Instruction(
        'bxor_not', """
        Bitwise xor not.

        Computes `x ^ ~y`.
        """,
        ins=(x, y), outs=a)

# Bitwise binary ops with immediate arg.
x = Operand('x', iB)
Y = Operand('Y', imm64)
a = Operand('a', iB)

band_imm = Instruction(
        'band_imm', """
        Bitwise and with immediate.

        Same as :inst:`band`, but one operand is an immediate constant.

        Polymorphic over all scalar integer types, but does not support vector
        types.
        """,
        ins=(x, Y), outs=a)

bor_imm = Instruction(
        'bor_imm', """
        Bitwise or with immediate.

        Same as :inst:`bor`, but one operand is an immediate constant.

        Polymorphic over all scalar integer types, but does not support vector
        types.
        """,
        ins=(x, Y), outs=a)

bxor_imm = Instruction(
        'bxor_imm', """
        Bitwise xor with immediate.

        Same as :inst:`bxor`, but one operand is an immediate constant.

        Polymorphic over all scalar integer types, but does not support vector
        types.
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
a = Operand('a', iB)

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
fB = TypeVar('fB', 'A scalar floating point number', floats=True)

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

f = Operand('f', fflags)

ffcmp = Instruction(
        'ffcmp', r"""
        Floating point comparison returning flags.

        Compares two numbers like :inst:`fcmp`, but returns floating point CPU
        flags instead of testing a specific condition.
        """,
        ins=(x, y), outs=f)

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

        Unlike the integer division instructions :clif:inst:`sdiv` and
        :clif:inst:`udiv`, this can't trap. Division by zero is infinity or
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

a = Operand('a', Float, 'The larger of ``x`` and ``y``')

fmax = Instruction(
        'fmax', r"""
        Floating point maximum, propagating NaNs.

        If either operand is NaN, this returns a NaN.
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
# CPU flag operations
#


Cond = Operand('Cond', intcc)
f = Operand('f', iflags)
a = Operand('a', b1)

trueif = Instruction(
        'trueif', r"""
        Test integer CPU flags for a specific condition.

        Check the CPU flags in ``f`` against the ``Cond`` condition code and
        return true when the condition code is satisfied.
        """,
        ins=(Cond, f), outs=a)

Cond = Operand('Cond', floatcc)
f = Operand('f', fflags)

trueff = Instruction(
        'trueff', r"""
        Test floating point CPU flags for a specific condition.

        Check the CPU flags in ``f`` against the ``Cond`` condition code and
        return true when the condition code is satisfied.
        """,
        ins=(Cond, f), outs=a)

#
# Conversions
#

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

Bool = TypeVar(
        'Bool',
        'A scalar or vector boolean type',
        bools=True, simd=True)
BoolTo = TypeVar(
        'BoolTo',
        'A smaller boolean type with the same number of lanes',
        bools=True, simd=True)

x = Operand('x', Bool)
a = Operand('a', BoolTo)

breduce = Instruction(
        'breduce', r"""
        Convert `x` to a smaller boolean type in the platform-defined way.

        The result type must have the same number of vector lanes as the input,
        and each lane must not have more bits that the input lanes. If the
        input and output types are the same, this is a no-op.
        """, ins=x, outs=a, constraints=WiderOrEq(Bool, BoolTo))

BoolTo = TypeVar(
        'BoolTo',
        'A larger boolean type with the same number of lanes',
        bools=True, simd=True)

x = Operand('x', Bool)
a = Operand('a', BoolTo)

bextend = Instruction(
        'bextend', r"""
        Convert `x` to a larger boolean type in the platform-defined way.

        The result type must have the same number of vector lanes as the input,
        and each lane must not have fewer bits that the input lanes. If the
        input and output types are the same, this is a no-op.
        """, ins=x, outs=a, constraints=WiderOrEq(BoolTo, Bool))

IntTo = TypeVar(
        'IntTo', 'An integer type with the same number of lanes',
        ints=True, simd=True)

x = Operand('x', Bool)
a = Operand('a', IntTo)

bint = Instruction(
        'bint', r"""
        Convert `x` to an integer.

        True maps to 1 and false maps to 0. The result type must have the same
        number of vector lanes as the input.
        """, ins=x, outs=a)

bmask = Instruction(
        'bmask', r"""
        Convert `x` to an integer mask.

        True maps to all 1s and false maps to all 0s. The result type must have
        the same number of vector lanes as the input.
        """, ins=x, outs=a)

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
        ins=x, outs=a, constraints=WiderOrEq(Int, IntTo))


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
        ins=x, outs=a, constraints=WiderOrEq(IntTo, Int))

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
        ins=x, outs=a, constraints=WiderOrEq(IntTo, Int))

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

        Cranelift currently only supports two floating point formats
        - :type:`f32` and :type:`f64`. This may change in the future.

        The result type must have the same number of vector lanes as the input,
        and the result lanes must not have fewer bits than the input lanes. If
        the input and output types are the same, this is a no-op.
        """,
        ins=x, outs=a, constraints=WiderOrEq(FloatTo, Float))

fdemote = Instruction(
        'fdemote', r"""
        Convert `x` to a smaller floating point format.

        Each lane in `x` is converted to the destination floating point format
        by rounding to nearest, ties to even.

        Cranelift currently only supports two floating point formats
        - :type:`f32` and :type:`f64`. This may change in the future.

        The result type must have the same number of vector lanes as the input,
        and the result lanes must not have more bits than the input lanes. If
        the input and output types are the same, this is a no-op.
        """,
        ins=x, outs=a, constraints=WiderOrEq(Float, FloatTo))

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

fcvt_to_uint_sat = Instruction(
        'fcvt_to_uint_sat', r"""
        Convert floating point to unsigned integer as fcvt_to_uint does, but
        saturates the input instead of trapping. NaN and negative values are
        converted to 0.
        """,
        ins=x, outs=a)

fcvt_to_sint = Instruction(
        'fcvt_to_sint', r"""
        Convert floating point to signed integer.

        Each lane in `x` is converted to a signed integer by rounding towards
        zero. If `x` is NaN or if the signed integral value cannot be
        represented in the result type, this instruction traps.

        The result type must have the same number of vector lanes as the input.
        """,
        ins=x, outs=a, can_trap=True)

fcvt_to_sint_sat = Instruction(
        'fcvt_to_sint_sat', r"""
        Convert floating point to signed integer as fcvt_to_sint does, but
        saturates the input instead of trapping. NaN values are converted to 0.
        """,
        ins=x, outs=a)

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
        'WideInt', 'An integer type with lanes from `i16` upwards',
        ints=(16, 64), simd=True)
x = Operand('x', WideInt)
lo = Operand(
        'lo', WideInt.half_width(), 'The low bits of `x`')
hi = Operand(
        'hi', WideInt.half_width(), 'The high bits of `x`')

isplit = Instruction(
        'isplit', r"""
        Split an integer into low and high parts.

        Vectors of integers are split lane-wise, so the results have the same
        number of lanes as the input, but the lanes are half the size.

        Returns the low half of `x` and the high half of `x` as two independent
        values.
        """,
        ins=x, outs=(lo, hi))


NarrowInt = TypeVar(
        'NarrowInt', 'An integer type with lanes type to `i32`',
        ints=(8, 32), simd=True)
lo = Operand('lo', NarrowInt)
hi = Operand('hi', NarrowInt)
a = Operand(
        'a', NarrowInt.double_width(),
        doc='The concatenation of `lo` and `hi`')

iconcat = Instruction(
        'iconcat', r"""
        Concatenate low and high bits to form a larger integer type.

        Vectors of integers are concatenated lane-wise such that the result has
        the same number of lanes as the inputs, but the lanes are twice the
        size.
        """,
        ins=(lo, hi), outs=a)

GROUP.close()
