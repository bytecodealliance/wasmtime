#![allow(non_snake_case)]

use crate::cdsl::formats::FormatRegistry;
use crate::cdsl::inst::{InstructionBuilder as Inst, InstructionGroup};
use crate::cdsl::operands::{create_operand as operand, create_operand_doc as operand_doc};
use crate::cdsl::type_inference::Constraint::WiderOrEq;
use crate::cdsl::types::{LaneType, ValueType};
use crate::cdsl::typevar::{Interval, TypeSetBuilder, TypeVar};
use crate::shared::{types, OperandKinds};

pub fn define(
    format_registry: &FormatRegistry,
    immediates: &OperandKinds,
    entities: &OperandKinds,
) -> InstructionGroup {
    let mut ig = InstructionGroup::new("base", "Shared base instruction set");

    // Operand kind shorthands.
    let intcc = immediates.by_name("intcc");
    let floatcc = immediates.by_name("floatcc");
    let trapcode = immediates.by_name("trapcode");
    let uimm8 = immediates.by_name("uimm8");
    let uimm32 = immediates.by_name("uimm32");
    let imm64 = immediates.by_name("imm64");
    let offset32 = immediates.by_name("offset32");
    let memflags = immediates.by_name("memflags");
    let ieee32 = immediates.by_name("ieee32");
    let ieee64 = immediates.by_name("ieee64");
    let boolean = immediates.by_name("boolean");
    let regunit = immediates.by_name("regunit");

    let ebb = entities.by_name("ebb");
    let jump_table = entities.by_name("jump_table");
    let variable_args = entities.by_name("variable_args");
    let func_ref = entities.by_name("func_ref");
    let sig_ref = entities.by_name("sig_ref");
    let stack_slot = entities.by_name("stack_slot");
    let global_value = entities.by_name("global_value");
    let heap = entities.by_name("heap");
    let table = entities.by_name("table");

    let iflags: &TypeVar = &ValueType::Special(types::Flag::IFlags.into()).into();
    let fflags: &TypeVar = &ValueType::Special(types::Flag::FFlags.into()).into();

    let b1: &TypeVar = &ValueType::from(LaneType::from(types::Bool::B1)).into();
    let f32_: &TypeVar = &ValueType::from(LaneType::from(types::Float::F32)).into();
    let f64_: &TypeVar = &ValueType::from(LaneType::from(types::Float::F64)).into();

    // Starting definitions.
    let Int = &TypeVar::new(
        "Int",
        "A scalar or vector integer type",
        TypeSetBuilder::new()
            .ints(Interval::All)
            .simd_lanes(Interval::All)
            .finish(),
    );

    let Bool = &TypeVar::new(
        "Bool",
        "A scalar or vector boolean type",
        TypeSetBuilder::new()
            .bools(Interval::All)
            .simd_lanes(Interval::All)
            .finish(),
    );

    let iB = &TypeVar::new(
        "iB",
        "A scalar integer type",
        TypeSetBuilder::new().ints(Interval::All).finish(),
    );

    let iAddr = &TypeVar::new(
        "iAddr",
        "An integer address type",
        TypeSetBuilder::new().ints(32..64).finish(),
    );

    let Testable = &TypeVar::new(
        "Testable",
        "A scalar boolean or integer type",
        TypeSetBuilder::new()
            .ints(Interval::All)
            .bools(Interval::All)
            .finish(),
    );

    let TxN = &TypeVar::new(
        "TxN",
        "A SIMD vector type",
        TypeSetBuilder::new()
            .ints(Interval::All)
            .floats(Interval::All)
            .bools(Interval::All)
            .simd_lanes(Interval::All)
            .includes_scalars(false)
            .finish(),
    );

    let Any = &TypeVar::new(
        "Any",
        "Any integer, float, or boolean scalar or vector type",
        TypeSetBuilder::new()
            .ints(Interval::All)
            .floats(Interval::All)
            .bools(Interval::All)
            .simd_lanes(Interval::All)
            .includes_scalars(true)
            .finish(),
    );

    let Mem = &TypeVar::new(
        "Mem",
        "Any type that can be stored in memory",
        TypeSetBuilder::new()
            .ints(Interval::All)
            .floats(Interval::All)
            .simd_lanes(Interval::All)
            .finish(),
    );

    let MemTo = &TypeVar::new(
        "MemTo",
        "Any type that can be stored in memory",
        TypeSetBuilder::new()
            .ints(Interval::All)
            .floats(Interval::All)
            .simd_lanes(Interval::All)
            .finish(),
    );

    let addr = &operand("addr", iAddr);
    let c = &operand_doc("c", Testable, "Controlling value to test");
    let Cond = &operand("Cond", intcc);
    let x = &operand("x", iB);
    let y = &operand("y", iB);
    let EBB = &operand_doc("EBB", ebb, "Destination extended basic block");
    let args = &operand_doc("args", variable_args, "EBB arguments");

    ig.push(
        Inst::new(
            "jump",
            r#"
        Jump.

        Unconditionally jump to an extended basic block, passing the specified
        EBB arguments. The number and types of arguments must match the
        destination EBB.
        "#,
        )
        .operands_in(vec![EBB, args])
        .is_terminator(true)
        .is_branch(true)
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "fallthrough",
            r#"
        Fall through to the next EBB.

        This is the same as :inst:`jump`, except the destination EBB must be
        the next one in the layout.

        Jumps are turned into fall-through instructions by the branch
        relaxation pass. There is no reason to use this instruction outside
        that pass.
        "#,
        )
        .operands_in(vec![EBB, args])
        .is_terminator(true)
        .is_branch(true)
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "brz",
            r#"
        Branch when zero.

        If ``c`` is a :type:`b1` value, take the branch when ``c`` is false. If
        ``c`` is an integer value, take the branch when ``c = 0``.
        "#,
        )
        .operands_in(vec![c, EBB, args])
        .is_branch(true)
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "brnz",
            r#"
        Branch when non-zero.

        If ``c`` is a :type:`b1` value, take the branch when ``c`` is true. If
        ``c`` is an integer value, take the branch when ``c != 0``.
        "#,
        )
        .operands_in(vec![c, EBB, args])
        .is_branch(true)
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "br_icmp",
            r#"
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
        "#,
        )
        .operands_in(vec![Cond, x, y, EBB, args])
        .is_branch(true)
        .finish(format_registry),
    );

    let f = &operand("f", iflags);

    ig.push(
        Inst::new(
            "brif",
            r#"
        Branch when condition is true in integer CPU flags.
        "#,
        )
        .operands_in(vec![Cond, f, EBB, args])
        .is_branch(true)
        .finish(format_registry),
    );

    let Cond = &operand("Cond", floatcc);
    let f = &operand("f", fflags);

    ig.push(
        Inst::new(
            "brff",
            r#"
        Branch when condition is true in floating point CPU flags.
        "#,
        )
        .operands_in(vec![Cond, f, EBB, args])
        .is_branch(true)
        .finish(format_registry),
    );

    let x = &operand_doc("x", iB, "index into jump table");

    let Entry = &TypeVar::new(
        "Entry",
        "A scalar integer type",
        TypeSetBuilder::new().ints(Interval::All).finish(),
    );

    let entry = &operand_doc("entry", Entry, "entry of jump table");
    let JT = &operand("JT", jump_table);

    ig.push(
        Inst::new(
            "br_table",
            r#"
        Indirect branch via jump table.

        Use ``x`` as an unsigned index into the jump table ``JT``. If a jump
        table entry is found, branch to the corresponding EBB. If no entry was
        found or the index is out-of-bounds, branch to the given default EBB.

        Note that this branch instruction can't pass arguments to the targeted
        blocks. Split critical edges as needed to work around this.

        Do not confuse this with "tables" in WebAssembly. ``br_table`` is for
        jump tables with destinations within the current function only -- think
        of a ``match`` in Rust or a ``switch`` in C.  If you want to call a
        function in a dynamic library, that will typically use
        ``call_indirect``.
        "#,
        )
        .operands_in(vec![x, EBB, JT])
        .is_terminator(true)
        .is_branch(true)
        .finish(format_registry),
    );

    let Size = &operand_doc("Size", uimm8, "Size in bytes");

    ig.push(
        Inst::new(
            "jump_table_entry",
            r#"
    Get an entry from a jump table.

    Load a serialized ``entry`` from a jump table ``JT`` at a given index
    ``addr`` with a specific ``Size``. The retrieved entry may need to be
    decoded after loading, depending upon the jump table type used.

    Currently, the only type supported is entries which are relative to the
    base of the jump table.
    "#,
        )
        .operands_in(vec![x, addr, Size, JT])
        .operands_out(vec![entry])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "jump_table_base",
            r#"
    Get the absolute base address of a jump table.

    This is used for jump tables wherein the entries are stored relative to
    the base of jump table. In order to use these, generated code should first
    load an entry using ``jump_table_entry``, then use this instruction to add
    the relative base back to it.
    "#,
        )
        .operands_in(vec![JT])
        .operands_out(vec![addr])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "indirect_jump_table_br",
            r#"
    Branch indirectly via a jump table entry.

    Unconditionally jump via a jump table entry that was previously loaded
    with the ``jump_table_entry`` instruction.
    "#,
        )
        .operands_in(vec![addr, JT])
        .is_indirect_branch(true)
        .is_terminator(true)
        .is_branch(true)
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "debugtrap",
            r#"
    Encodes an assembly debug trap.
    "#,
        )
        .other_side_effects(true)
        .can_load(true)
        .can_store(true)
        .finish(format_registry),
    );

    let code = &operand("code", trapcode);

    ig.push(
        Inst::new(
            "trap",
            r#"
        Terminate execution unconditionally.
        "#,
        )
        .operands_in(vec![code])
        .can_trap(true)
        .is_terminator(true)
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "trapz",
            r#"
        Trap when zero.

        if ``c`` is non-zero, execution continues at the following instruction.
        "#,
        )
        .operands_in(vec![c, code])
        .can_trap(true)
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "trapnz",
            r#"
        Trap when non-zero.

        if ``c`` is zero, execution continues at the following instruction.
        "#,
        )
        .operands_in(vec![c, code])
        .can_trap(true)
        .finish(format_registry),
    );

    let Cond = &operand("Cond", intcc);
    let f = &operand("f", iflags);

    ig.push(
        Inst::new(
            "trapif",
            r#"
        Trap when condition is true in integer CPU flags.
        "#,
        )
        .operands_in(vec![Cond, f, code])
        .can_trap(true)
        .finish(format_registry),
    );

    let Cond = &operand("Cond", floatcc);
    let f = &operand("f", fflags);

    ig.push(
        Inst::new(
            "trapff",
            r#"
        Trap when condition is true in floating point CPU flags.
        "#,
        )
        .operands_in(vec![Cond, f, code])
        .can_trap(true)
        .finish(format_registry),
    );

    let rvals = &operand_doc("rvals", variable_args, "return values");

    ig.push(
        Inst::new(
            "return",
            r#"
        Return from the function.

        Unconditionally transfer control to the calling function, passing the
        provided return values. The list of return values must match the
        function signature's return types.
        "#,
        )
        .operands_in(vec![rvals])
        .is_return(true)
        .is_terminator(true)
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "fallthrough_return",
            r#"
        Return from the function by fallthrough.

        This is a specialized instruction for use where one wants to append
        a custom epilogue, which will then perform the real return. This
        instruction has no encoding.
        "#,
        )
        .operands_in(vec![rvals])
        .is_return(true)
        .is_terminator(true)
        .finish(format_registry),
    );

    let FN = &operand_doc(
        "FN",
        func_ref,
        "function to call, declared by :inst:`function`",
    );
    let args = &operand_doc("args", variable_args, "call arguments");

    ig.push(
        Inst::new(
            "call",
            r#"
        Direct function call.

        Call a function which has been declared in the preamble. The argument
        types must match the function's signature.
        "#,
        )
        .operands_in(vec![FN, args])
        .operands_out(vec![rvals])
        .is_call(true)
        .finish(format_registry),
    );

    let SIG = &operand_doc("SIG", sig_ref, "function signature");
    let callee = &operand_doc("callee", iAddr, "address of function to call");

    ig.push(
        Inst::new(
            "call_indirect",
            r#"
        Indirect function call.

        Call the function pointed to by `callee` with the given arguments. The
        called function must match the specified signature.

        Note that this is different from WebAssembly's ``call_indirect``; the
        callee is a native address, rather than a table index. For WebAssembly,
        :inst:`table_addr` and :inst:`load` are used to obtain a native address
        from a table.
        "#,
        )
        .operands_in(vec![SIG, callee, args])
        .operands_out(vec![rvals])
        .is_call(true)
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "func_addr",
            r#"
        Get the address of a function.

        Compute the absolute address of a function declared in the preamble.
        The returned address can be used as a ``callee`` argument to
        :inst:`call_indirect`. This is also a method for calling functions that
        are too far away to be addressable by a direct :inst:`call`
        instruction.
        "#,
        )
        .operands_in(vec![FN])
        .operands_out(vec![addr])
        .finish(format_registry),
    );

    let SS = &operand("SS", stack_slot);
    let Offset = &operand_doc("Offset", offset32, "Byte offset from base address");
    let x = &operand_doc("x", Mem, "Value to be stored");
    let a = &operand_doc("a", Mem, "Value loaded");
    let p = &operand("p", iAddr);
    let MemFlags = &operand("MemFlags", memflags);
    let args = &operand_doc("args", variable_args, "Address arguments");

    ig.push(
        Inst::new(
            "load",
            r#"
        Load from memory at ``p + Offset``.

        This is a polymorphic instruction that can load any value type which
        has a memory representation.
        "#,
        )
        .operands_in(vec![MemFlags, p, Offset])
        .operands_out(vec![a])
        .can_load(true)
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "load_complex",
            r#"
        Load from memory at ``sum(args) + Offset``.

        This is a polymorphic instruction that can load any value type which
        has a memory representation.
        "#,
        )
        .operands_in(vec![MemFlags, args, Offset])
        .operands_out(vec![a])
        .can_load(true)
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "store",
            r#"
        Store ``x`` to memory at ``p + Offset``.

        This is a polymorphic instruction that can store any value type with a
        memory representation.
        "#,
        )
        .operands_in(vec![MemFlags, x, p, Offset])
        .can_store(true)
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "store_complex",
            r#"
        Store ``x`` to memory at ``sum(args) + Offset``.

        This is a polymorphic instruction that can store any value type with a
        memory representation.
        "#,
        )
        .operands_in(vec![MemFlags, x, args, Offset])
        .can_store(true)
        .finish(format_registry),
    );

    let iExt8 = &TypeVar::new(
        "iExt8",
        "An integer type with more than 8 bits",
        TypeSetBuilder::new().ints(16..64).finish(),
    );
    let x = &operand("x", iExt8);
    let a = &operand("a", iExt8);

    ig.push(
        Inst::new(
            "uload8",
            r#"
        Load 8 bits from memory at ``p + Offset`` and zero-extend.

        This is equivalent to ``load.i8`` followed by ``uextend``.
        "#,
        )
        .operands_in(vec![MemFlags, p, Offset])
        .operands_out(vec![a])
        .can_load(true)
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "uload8_complex",
            r#"
        Load 8 bits from memory at ``sum(args) + Offset`` and zero-extend.

        This is equivalent to ``load.i8`` followed by ``uextend``.
        "#,
        )
        .operands_in(vec![MemFlags, args, Offset])
        .operands_out(vec![a])
        .can_load(true)
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "sload8",
            r#"
        Load 8 bits from memory at ``p + Offset`` and sign-extend.

        This is equivalent to ``load.i8`` followed by ``sextend``.
        "#,
        )
        .operands_in(vec![MemFlags, p, Offset])
        .operands_out(vec![a])
        .can_load(true)
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "sload8_complex",
            r#"
        Load 8 bits from memory at ``sum(args) + Offset`` and sign-extend.

        This is equivalent to ``load.i8`` followed by ``sextend``.
        "#,
        )
        .operands_in(vec![MemFlags, args, Offset])
        .operands_out(vec![a])
        .can_load(true)
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "istore8",
            r#"
        Store the low 8 bits of ``x`` to memory at ``p + Offset``.

        This is equivalent to ``ireduce.i8`` followed by ``store.i8``.
        "#,
        )
        .operands_in(vec![MemFlags, x, p, Offset])
        .can_store(true)
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "istore8_complex",
            r#"
        Store the low 8 bits of ``x`` to memory at ``sum(args) + Offset``.

        This is equivalent to ``ireduce.i8`` followed by ``store.i8``.
        "#,
        )
        .operands_in(vec![MemFlags, x, args, Offset])
        .can_store(true)
        .finish(format_registry),
    );

    let iExt16 = &TypeVar::new(
        "iExt16",
        "An integer type with more than 16 bits",
        TypeSetBuilder::new().ints(32..64).finish(),
    );
    let x = &operand("x", iExt16);
    let a = &operand("a", iExt16);

    ig.push(
        Inst::new(
            "uload16",
            r#"
        Load 16 bits from memory at ``p + Offset`` and zero-extend.

        This is equivalent to ``load.i16`` followed by ``uextend``.
        "#,
        )
        .operands_in(vec![MemFlags, p, Offset])
        .operands_out(vec![a])
        .can_load(true)
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "uload16_complex",
            r#"
        Load 16 bits from memory at ``sum(args) + Offset`` and zero-extend.

        This is equivalent to ``load.i16`` followed by ``uextend``.
        "#,
        )
        .operands_in(vec![MemFlags, args, Offset])
        .operands_out(vec![a])
        .can_load(true)
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "sload16",
            r#"
        Load 16 bits from memory at ``p + Offset`` and sign-extend.

        This is equivalent to ``load.i16`` followed by ``sextend``.
        "#,
        )
        .operands_in(vec![MemFlags, p, Offset])
        .operands_out(vec![a])
        .can_load(true)
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "sload16_complex",
            r#"
        Load 16 bits from memory at ``sum(args) + Offset`` and sign-extend.

        This is equivalent to ``load.i16`` followed by ``sextend``.
        "#,
        )
        .operands_in(vec![MemFlags, args, Offset])
        .operands_out(vec![a])
        .can_load(true)
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "istore16",
            r#"
        Store the low 16 bits of ``x`` to memory at ``p + Offset``.

        This is equivalent to ``ireduce.i16`` followed by ``store.i16``.
        "#,
        )
        .operands_in(vec![MemFlags, x, p, Offset])
        .can_store(true)
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "istore16_complex",
            r#"
        Store the low 16 bits of ``x`` to memory at ``sum(args) + Offset``.

        This is equivalent to ``ireduce.i16`` followed by ``store.i16``.
        "#,
        )
        .operands_in(vec![MemFlags, x, args, Offset])
        .can_store(true)
        .finish(format_registry),
    );

    let iExt32 = &TypeVar::new(
        "iExt32",
        "An integer type with more than 32 bits",
        TypeSetBuilder::new().ints(64..64).finish(),
    );
    let x = &operand("x", iExt32);
    let a = &operand("a", iExt32);

    ig.push(
        Inst::new(
            "uload32",
            r#"
        Load 32 bits from memory at ``p + Offset`` and zero-extend.

        This is equivalent to ``load.i32`` followed by ``uextend``.
        "#,
        )
        .operands_in(vec![MemFlags, p, Offset])
        .operands_out(vec![a])
        .can_load(true)
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "uload32_complex",
            r#"
        Load 32 bits from memory at ``sum(args) + Offset`` and zero-extend.

        This is equivalent to ``load.i32`` followed by ``uextend``.
        "#,
        )
        .operands_in(vec![MemFlags, args, Offset])
        .operands_out(vec![a])
        .can_load(true)
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "sload32",
            r#"
        Load 32 bits from memory at ``p + Offset`` and sign-extend.

        This is equivalent to ``load.i32`` followed by ``sextend``.
        "#,
        )
        .operands_in(vec![MemFlags, p, Offset])
        .operands_out(vec![a])
        .can_load(true)
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "sload32_complex",
            r#"
        Load 32 bits from memory at ``sum(args) + Offset`` and sign-extend.

        This is equivalent to ``load.i32`` followed by ``sextend``.
        "#,
        )
        .operands_in(vec![MemFlags, args, Offset])
        .operands_out(vec![a])
        .can_load(true)
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "istore32",
            r#"
        Store the low 32 bits of ``x`` to memory at ``p + Offset``.

        This is equivalent to ``ireduce.i32`` followed by ``store.i32``.
        "#,
        )
        .operands_in(vec![MemFlags, x, p, Offset])
        .can_store(true)
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "istore32_complex",
            r#"
        Store the low 32 bits of ``x`` to memory at ``sum(args) + Offset``.

        This is equivalent to ``ireduce.i32`` followed by ``store.i32``.
        "#,
        )
        .operands_in(vec![MemFlags, x, args, Offset])
        .can_store(true)
        .finish(format_registry),
    );

    let x = &operand_doc("x", Mem, "Value to be stored");
    let a = &operand_doc("a", Mem, "Value loaded");
    let Offset = &operand_doc("Offset", offset32, "In-bounds offset into stack slot");

    ig.push(
        Inst::new(
            "stack_load",
            r#"
        Load a value from a stack slot at the constant offset.

        This is a polymorphic instruction that can load any value type which
        has a memory representation.

        The offset is an immediate constant, not an SSA value. The memory
        access cannot go out of bounds, i.e.
        :math:`sizeof(a) + Offset <= sizeof(SS)`.
        "#,
        )
        .operands_in(vec![SS, Offset])
        .operands_out(vec![a])
        .can_load(true)
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "stack_store",
            r#"
        Store a value to a stack slot at a constant offset.

        This is a polymorphic instruction that can store any value type with a
        memory representation.

        The offset is an immediate constant, not an SSA value. The memory
        access cannot go out of bounds, i.e.
        :math:`sizeof(a) + Offset <= sizeof(SS)`.
        "#,
        )
        .operands_in(vec![x, SS, Offset])
        .can_store(true)
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "stack_addr",
            r#"
        Get the address of a stack slot.

        Compute the absolute address of a byte in a stack slot. The offset must
        refer to a byte inside the stack slot:
        :math:`0 <= Offset < sizeof(SS)`.
        "#,
        )
        .operands_in(vec![SS, Offset])
        .operands_out(vec![addr])
        .finish(format_registry),
    );

    let GV = &operand("GV", global_value);

    ig.push(
        Inst::new(
            "global_value",
            r#"
        Compute the value of global GV.
        "#,
        )
        .operands_in(vec![GV])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "symbol_value",
            r#"
        Compute the value of global GV, which is a symbolic value.
        "#,
        )
        .operands_in(vec![GV])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    let HeapOffset = &TypeVar::new(
        "HeapOffset",
        "An unsigned heap offset",
        TypeSetBuilder::new().ints(32..64).finish(),
    );

    let H = &operand("H", heap);
    let p = &operand("p", HeapOffset);
    let Size = &operand_doc("Size", uimm32, "Size in bytes");

    ig.push(
        Inst::new(
            "heap_addr",
            r#"
        Bounds check and compute absolute address of heap memory.

        Verify that the offset range ``p .. p + Size - 1`` is in bounds for the
        heap H, and generate an absolute address that is safe to dereference.

        1. If ``p + Size`` is not greater than the heap bound, return an
           absolute address corresponding to a byte offset of ``p`` from the
           heap's base address.
        2. If ``p + Size`` is greater than the heap bound, generate a trap.
        "#,
        )
        .operands_in(vec![H, p, Size])
        .operands_out(vec![addr])
        .finish(format_registry),
    );

    let TableOffset = &TypeVar::new(
        "TableOffset",
        "An unsigned table offset",
        TypeSetBuilder::new().ints(32..64).finish(),
    );
    let T = &operand("T", table);
    let p = &operand("p", TableOffset);
    let Offset = &operand_doc("Offset", offset32, "Byte offset from element address");

    ig.push(
        Inst::new(
            "table_addr",
            r#"
        Bounds check and compute absolute address of a table entry.

        Verify that the offset ``p`` is in bounds for the table T, and generate
        an absolute address that is safe to dereference.

        ``Offset`` must be less than the size of a table element.

        1. If ``p`` is not greater than the table bound, return an absolute
           address corresponding to a byte offset of ``p`` from the table's
           base address.
        2. If ``p`` is greater than the table bound, generate a trap.
        "#,
        )
        .operands_in(vec![T, p, Offset])
        .operands_out(vec![addr])
        .finish(format_registry),
    );

    let N = &operand("N", imm64);
    let a = &operand_doc("a", Int, "A constant integer scalar or vector value");

    ig.push(
        Inst::new(
            "iconst",
            r#"
        Integer constant.

        Create a scalar integer SSA value with an immediate constant value, or
        an integer vector where all the lanes have the same value.
        "#,
        )
        .operands_in(vec![N])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    let N = &operand("N", ieee32);
    let a = &operand_doc("a", f32_, "A constant f32 scalar value");

    ig.push(
        Inst::new(
            "f32const",
            r#"
        Floating point constant.

        Create a :type:`f32` SSA value with an immediate constant value.
        "#,
        )
        .operands_in(vec![N])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    let N = &operand("N", ieee64);
    let a = &operand_doc("a", f64_, "A constant f64 scalar value");

    ig.push(
        Inst::new(
            "f64const",
            r#"
        Floating point constant.

        Create a :type:`f64` SSA value with an immediate constant value.
        "#,
        )
        .operands_in(vec![N])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    let N = &operand("N", boolean);
    let a = &operand_doc("a", Bool, "A constant boolean scalar or vector value");

    ig.push(
        Inst::new(
            "bconst",
            r#"
        Boolean constant.

        Create a scalar boolean SSA value with an immediate constant value, or
        a boolean vector where all the lanes have the same value.
        "#,
        )
        .operands_in(vec![N])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "nop",
            r#"
        Just a dummy instruction

        Note: this doesn't compile to a machine code nop
        "#,
        )
        .finish(format_registry),
    );

    let c = &operand_doc("c", Testable, "Controlling value to test");
    let x = &operand_doc("x", Any, "Value to use when `c` is true");
    let y = &operand_doc("y", Any, "Value to use when `c` is false");
    let a = &operand("a", Any);

    ig.push(
        Inst::new(
            "select",
            r#"
        Conditional select.

        This instruction selects whole values. Use :inst:`vselect` for
        lane-wise selection.
        "#,
        )
        .operands_in(vec![c, x, y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    let cc = &operand_doc("cc", intcc, "Controlling condition code");
    let flags = &operand_doc("flags", iflags, "The machine's flag register");

    ig.push(
        Inst::new(
            "selectif",
            r#"
        Conditional select, dependent on integer condition codes.
        "#,
        )
        .operands_in(vec![cc, flags, x, y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    let x = &operand("x", Any);

    ig.push(
        Inst::new(
            "copy",
            r#"
        Register-register copy.

        This instruction copies its input, preserving the value type.

        A pure SSA-form program does not need to copy values, but this
        instruction is useful for representing intermediate stages during
        instruction transformations, and the register allocator needs a way of
        representing register copies.
        "#,
        )
        .operands_in(vec![x])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "spill",
            r#"
        Spill a register value to a stack slot.

        This instruction behaves exactly like :inst:`copy`, but the result
        value is assigned to a spill slot.
        "#,
        )
        .operands_in(vec![x])
        .operands_out(vec![a])
        .can_store(true)
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "fill",
            r#"
        Load a register value from a stack slot.

        This instruction behaves exactly like :inst:`copy`, but creates a new
        SSA value for the spilled input value.
        "#,
        )
        .operands_in(vec![x])
        .operands_out(vec![a])
        .can_load(true)
        .finish(format_registry),
    );

    let src = &operand("src", regunit);
    let dst = &operand("dst", regunit);

    ig.push(
        Inst::new(
            "regmove",
            r#"
        Temporarily divert ``x`` from ``src`` to ``dst``.

        This instruction moves the location of a value from one register to
        another without creating a new SSA value. It is used by the register
        allocator to temporarily rearrange register assignments in order to
        satisfy instruction constraints.

        The register diversions created by this instruction must be undone
        before the value leaves the EBB. At the entry to a new EBB, all live
        values must be in their originally assigned registers.
        "#,
        )
        .operands_in(vec![x, src, dst])
        .other_side_effects(true)
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "copy_special",
            r#"
        Copies the contents of ''src'' register to ''dst'' register.

        This instructions copies the contents of one register to another
        register without involving any SSA values. This is used for copying
        special registers, e.g. copying the stack register to the frame
        register in a function prologue.
        "#,
        )
        .operands_in(vec![src, dst])
        .other_side_effects(true)
        .finish(format_registry),
    );

    let delta = &operand("delta", Int);

    ig.push(
        Inst::new(
            "adjust_sp_down",
            r#"
    Subtracts ``delta`` offset value from the stack pointer register.

    This instruction is used to adjust the stack pointer by a dynamic amount.
    "#,
        )
        .operands_in(vec![delta])
        .other_side_effects(true)
        .finish(format_registry),
    );

    let Offset = &operand_doc("Offset", imm64, "Offset from current stack pointer");

    ig.push(
        Inst::new(
            "adjust_sp_up_imm",
            r#"
    Adds ``Offset`` immediate offset value to the stack pointer register.

    This instruction is used to adjust the stack pointer, primarily in function
    prologues and epilogues. ``Offset`` is constrained to the size of a signed
    32-bit integer.
    "#,
        )
        .operands_in(vec![Offset])
        .other_side_effects(true)
        .finish(format_registry),
    );

    let Offset = &operand_doc("Offset", imm64, "Offset from current stack pointer");

    ig.push(
        Inst::new(
            "adjust_sp_down_imm",
            r#"
    Subtracts ``Offset`` immediate offset value from the stack pointer
    register.

    This instruction is used to adjust the stack pointer, primarily in function
    prologues and epilogues. ``Offset`` is constrained to the size of a signed
    32-bit integer.
    "#,
        )
        .operands_in(vec![Offset])
        .other_side_effects(true)
        .finish(format_registry),
    );

    let f = &operand("f", iflags);

    ig.push(
        Inst::new(
            "ifcmp_sp",
            r#"
    Compare ``addr`` with the stack pointer and set the CPU flags.

    This is like :inst:`ifcmp` where ``addr`` is the LHS operand and the stack
    pointer is the RHS.
    "#,
        )
        .operands_in(vec![addr])
        .operands_out(vec![f])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "regspill",
            r#"
        Temporarily divert ``x`` from ``src`` to ``SS``.

        This instruction moves the location of a value from a register to a
        stack slot without creating a new SSA value. It is used by the register
        allocator to temporarily rearrange register assignments in order to
        satisfy instruction constraints.

        See also :inst:`regmove`.
        "#,
        )
        .operands_in(vec![x, src, SS])
        .other_side_effects(true)
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "regfill",
            r#"
        Temporarily divert ``x`` from ``SS`` to ``dst``.

        This instruction moves the location of a value from a stack slot to a
        register without creating a new SSA value. It is used by the register
        allocator to temporarily rearrange register assignments in order to
        satisfy instruction constraints.

        See also :inst:`regmove`.
        "#,
        )
        .operands_in(vec![x, SS, dst])
        .other_side_effects(true)
        .finish(format_registry),
    );

    let x = &operand_doc("x", TxN, "Vector to split");
    let lo = &operand_doc("lo", &TxN.half_vector(), "Low-numbered lanes of `x`");
    let hi = &operand_doc("hi", &TxN.half_vector(), "High-numbered lanes of `x`");

    ig.push(
        Inst::new(
            "vsplit",
            r#"
        Split a vector into two halves.

        Split the vector `x` into two separate values, each containing half of
        the lanes from ``x``. The result may be two scalars if ``x`` only had
        two lanes.
        "#,
        )
        .operands_in(vec![x])
        .operands_out(vec![lo, hi])
        .is_ghost(true)
        .finish(format_registry),
    );

    let Any128 = &TypeVar::new(
        "Any128",
        "Any scalar or vector type with as most 128 lanes",
        TypeSetBuilder::new()
            .ints(Interval::All)
            .floats(Interval::All)
            .bools(Interval::All)
            .simd_lanes(1..128)
            .includes_scalars(true)
            .finish(),
    );

    let x = &operand_doc("x", Any128, "Low-numbered lanes");
    let y = &operand_doc("y", Any128, "High-numbered lanes");
    let a = &operand_doc("a", &Any128.double_vector(), "Concatenation of `x` and `y`");

    ig.push(
        Inst::new(
            "vconcat",
            r#"
        Vector concatenation.

        Return a vector formed by concatenating ``x`` and ``y``. The resulting
        vector type has twice as many lanes as each of the inputs. The lanes of
        ``x`` appear as the low-numbered lanes, and the lanes of ``y`` become
        the high-numbered lanes of ``a``.

        It is possible to form a vector by concatenating two scalars.
        "#,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a])
        .is_ghost(true)
        .finish(format_registry),
    );

    let c = &operand_doc("c", &TxN.as_bool(), "Controlling vector");
    let x = &operand_doc("x", TxN, "Value to use where `c` is true");
    let y = &operand_doc("y", TxN, "Value to use where `c` is false");
    let a = &operand("a", TxN);

    ig.push(
        Inst::new(
            "vselect",
            r#"
        Vector lane select.

        Select lanes from ``x`` or ``y`` controlled by the lanes of the boolean
        vector ``c``.
        "#,
        )
        .operands_in(vec![c, x, y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    let x = &operand("x", &TxN.lane_of());

    ig.push(
        Inst::new(
            "splat",
            r#"
        Vector splat.

        Return a vector whose lanes are all ``x``.
        "#,
        )
        .operands_in(vec![x])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    let x = &operand_doc("x", TxN, "SIMD vector to modify");
    let y = &operand_doc("y", &TxN.lane_of(), "New lane value");
    let Idx = &operand_doc("Idx", uimm8, "Lane index");

    ig.push(
        Inst::new(
            "insertlane",
            r#"
        Insert ``y`` as lane ``Idx`` in x.

        The lane index, ``Idx``, is an immediate value, not an SSA value. It
        must indicate a valid lane index for the type of ``x``.
        "#,
        )
        .operands_in(vec![x, Idx, y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    let x = &operand("x", TxN);
    let a = &operand("a", &TxN.lane_of());

    ig.push(
        Inst::new(
            "extractlane",
            r#"
        Extract lane ``Idx`` from ``x``.

        The lane index, ``Idx``, is an immediate value, not an SSA value. It
        must indicate a valid lane index for the type of ``x``.
        "#,
        )
        .operands_in(vec![x, Idx])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    let a = &operand("a", &Int.as_bool());
    let Cond = &operand("Cond", intcc);
    let x = &operand("x", Int);
    let y = &operand("y", Int);

    ig.push(
        Inst::new(
            "icmp",
            r#"
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
        "#,
        )
        .operands_in(vec![Cond, x, y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    let a = &operand("a", b1);
    let x = &operand("x", iB);
    let Y = &operand("Y", imm64);

    ig.push(
        Inst::new(
            "icmp_imm",
            r#"
        Compare scalar integer to a constant.

        This is the same as the :inst:`icmp` instruction, except one operand is
        an immediate constant.

        This instruction can only compare scalars. Use :inst:`icmp` for
        lane-wise vector comparisons.
        "#,
        )
        .operands_in(vec![Cond, x, Y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    let f = &operand("f", iflags);
    let x = &operand("x", iB);
    let y = &operand("y", iB);

    ig.push(
        Inst::new(
            "ifcmp",
            r#"
        Compare scalar integers and return flags.

        Compare two scalar integer values and return integer CPU flags
        representing the result.
        "#,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![f])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "ifcmp_imm",
            r#"
        Compare scalar integer to a constant and return flags.

        Like :inst:`icmp_imm`, but returns integer CPU flags instead of testing
        a specific condition code.
        "#,
        )
        .operands_in(vec![x, Y])
        .operands_out(vec![f])
        .finish(format_registry),
    );

    let a = &operand("a", Int);
    let x = &operand("x", Int);
    let y = &operand("y", Int);

    ig.push(
        Inst::new(
            "iadd",
            r#"
        Wrapping integer addition: :math:`a := x + y \pmod{2^B}`.

        This instruction does not depend on the signed/unsigned interpretation
        of the operands.
        "#,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "isub",
            r#"
        Wrapping integer subtraction: :math:`a := x - y \pmod{2^B}`.

        This instruction does not depend on the signed/unsigned interpretation
        of the operands.
        "#,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "imul",
            r#"
        Wrapping integer multiplication: :math:`a := x y \pmod{2^B}`.

        This instruction does not depend on the signed/unsigned interpretation
        of the
        operands.

        Polymorphic over all integer types (vector and scalar).
        "#,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "umulhi",
            r#"
        Unsigned integer multiplication, producing the high half of a
        double-length result.

        Polymorphic over all scalar integer types, but does not support vector
        types.
        "#,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "smulhi",
            r#"
        Signed integer multiplication, producing the high half of a
        double-length result.

        Polymorphic over all scalar integer types, but does not support vector
        types.
        "#,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "udiv",
            r#"
        Unsigned integer division: :math:`a := \lfloor {x \over y} \rfloor`.

        This operation traps if the divisor is zero.
        "#,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a])
        .can_trap(true)
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "sdiv",
            r#"
        Signed integer division rounded toward zero: :math:`a := sign(xy)
        \lfloor {|x| \over |y|}\rfloor`.

        This operation traps if the divisor is zero, or if the result is not
        representable in :math:`B` bits two's complement. This only happens
        when :math:`x = -2^{B-1}, y = -1`.
        "#,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a])
        .can_trap(true)
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "urem",
            r#"
        Unsigned integer remainder.

        This operation traps if the divisor is zero.
        "#,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a])
        .can_trap(true)
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "srem",
            r#"
        Signed integer remainder. The result has the sign of the dividend.

        This operation traps if the divisor is zero.
        "#,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a])
        .can_trap(true)
        .finish(format_registry),
    );

    let a = &operand("a", iB);
    let x = &operand("x", iB);
    let Y = &operand("Y", imm64);

    ig.push(
        Inst::new(
            "iadd_imm",
            r#"
        Add immediate integer.

        Same as :inst:`iadd`, but one operand is an immediate constant.

        Polymorphic over all scalar integer types, but does not support vector
        types.
        "#,
        )
        .operands_in(vec![x, Y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "imul_imm",
            r#"
        Integer multiplication by immediate constant.

        Polymorphic over all scalar integer types, but does not support vector
        types.
        "#,
        )
        .operands_in(vec![x, Y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "udiv_imm",
            r#"
        Unsigned integer division by an immediate constant.

        This operation traps if the divisor is zero.
        "#,
        )
        .operands_in(vec![x, Y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "sdiv_imm",
            r#"
        Signed integer division by an immediate constant.

        This operation traps if the divisor is zero, or if the result is not
        representable in :math:`B` bits two's complement. This only happens
        when :math:`x = -2^{B-1}, Y = -1`.
        "#,
        )
        .operands_in(vec![x, Y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "urem_imm",
            r#"
        Unsigned integer remainder with immediate divisor.

        This operation traps if the divisor is zero.
        "#,
        )
        .operands_in(vec![x, Y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "srem_imm",
            r#"
        Signed integer remainder with immediate divisor.

        This operation traps if the divisor is zero.
        "#,
        )
        .operands_in(vec![x, Y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "irsub_imm",
            r#"
        Immediate reverse wrapping subtraction: :math:`a := Y - x \pmod{2^B}`.

        Also works as integer negation when :math:`Y = 0`. Use :inst:`iadd_imm`
        with a negative immediate operand for the reverse immediate
        subtraction.

        Polymorphic over all scalar integer types, but does not support vector
        types.
        "#,
        )
        .operands_in(vec![x, Y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    let a = &operand("a", iB);
    let x = &operand("x", iB);
    let y = &operand("y", iB);
    let c_in = &operand_doc("c_in", b1, "Input carry flag");
    let c_out = &operand_doc("c_out", b1, "Output carry flag");
    let b_in = &operand_doc("b_in", b1, "Input borrow flag");
    let b_out = &operand_doc("b_out", b1, "Output borrow flag");

    ig.push(
        Inst::new(
            "iadd_cin",
            r#"
        Add integers with carry in.

        Same as :inst:`iadd` with an additional carry input. Computes:

        .. math::

            a = x + y + c_{in} \pmod 2^B

        Polymorphic over all scalar integer types, but does not support vector
        types.
        "#,
        )
        .operands_in(vec![x, y, c_in])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "iadd_cout",
            r#"
        Add integers with carry out.

        Same as :inst:`iadd` with an additional carry output.

        .. math::

            a &= x + y \pmod 2^B \\
            c_{out} &= x+y >= 2^B

        Polymorphic over all scalar integer types, but does not support vector
        types.
        "#,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a, c_out])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "iadd_carry",
            r#"
        Add integers with carry in and out.

        Same as :inst:`iadd` with an additional carry input and output.

        .. math::

            a &= x + y + c_{in} \pmod 2^B \\
            c_{out} &= x + y + c_{in} >= 2^B

        Polymorphic over all scalar integer types, but does not support vector
        types.
        "#,
        )
        .operands_in(vec![x, y, c_in])
        .operands_out(vec![a, c_out])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "isub_bin",
            r#"
        Subtract integers with borrow in.

        Same as :inst:`isub` with an additional borrow flag input. Computes:

        .. math::

            a = x - (y + b_{in}) \pmod 2^B

        Polymorphic over all scalar integer types, but does not support vector
        types.
        "#,
        )
        .operands_in(vec![x, y, b_in])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "isub_bout",
            r#"
        Subtract integers with borrow out.

        Same as :inst:`isub` with an additional borrow flag output.

        .. math::

            a &= x - y \pmod 2^B \\
            b_{out} &= x < y

        Polymorphic over all scalar integer types, but does not support vector
        types.
        "#,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a, b_out])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "isub_borrow",
            r#"
        Subtract integers with borrow in and out.

        Same as :inst:`isub` with an additional borrow flag input and output.

        .. math::

            a &= x - (y + b_{in}) \pmod 2^B \\
            b_{out} &= x < y + b_{in}

        Polymorphic over all scalar integer types, but does not support vector
        types.
        "#,
        )
        .operands_in(vec![x, y, b_in])
        .operands_out(vec![a, b_out])
        .finish(format_registry),
    );

    let bits = &TypeVar::new(
        "bits",
        "Any integer, float, or boolean scalar or vector type",
        TypeSetBuilder::new()
            .ints(Interval::All)
            .floats(Interval::All)
            .bools(Interval::All)
            .simd_lanes(Interval::All)
            .includes_scalars(true)
            .finish(),
    );
    let x = &operand("x", bits);
    let y = &operand("y", bits);
    let a = &operand("a", bits);

    ig.push(
        Inst::new(
            "band",
            r#"
        Bitwise and.
        "#,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "bor",
            r#"
        Bitwise or.
        "#,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "bxor",
            r#"
        Bitwise xor.
        "#,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "bnot",
            r#"
        Bitwise not.
        "#,
        )
        .operands_in(vec![x])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "band_not",
            r#"
        Bitwise and not.

        Computes `x & ~y`.
        "#,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "bor_not",
            r#"
        Bitwise or not.

        Computes `x | ~y`.
        "#,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "bxor_not",
            r#"
        Bitwise xor not.

        Computes `x ^ ~y`.
        "#,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    let x = &operand("x", iB);
    let Y = &operand("Y", imm64);
    let a = &operand("a", iB);

    ig.push(
        Inst::new(
            "band_imm",
            r#"
        Bitwise and with immediate.

        Same as :inst:`band`, but one operand is an immediate constant.

        Polymorphic over all scalar integer types, but does not support vector
        types.
        "#,
        )
        .operands_in(vec![x, Y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "bor_imm",
            r#"
        Bitwise or with immediate.

        Same as :inst:`bor`, but one operand is an immediate constant.

        Polymorphic over all scalar integer types, but does not support vector
        types.
        "#,
        )
        .operands_in(vec![x, Y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "bxor_imm",
            r#"
        Bitwise xor with immediate.

        Same as :inst:`bxor`, but one operand is an immediate constant.

        Polymorphic over all scalar integer types, but does not support vector
        types.
        "#,
        )
        .operands_in(vec![x, Y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    let x = &operand_doc("x", Int, "Scalar or vector value to shift");
    let y = &operand_doc("y", iB, "Number of bits to shift");
    let Y = &operand("Y", imm64);
    let a = &operand("a", Int);

    ig.push(
        Inst::new(
            "rotl",
            r#"
        Rotate left.

        Rotate the bits in ``x`` by ``y`` places.
        "#,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "rotr",
            r#"
        Rotate right.

        Rotate the bits in ``x`` by ``y`` places.
        "#,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "rotl_imm",
            r#"
        Rotate left by immediate.
        "#,
        )
        .operands_in(vec![x, Y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "rotr_imm",
            r#"
        Rotate right by immediate.
        "#,
        )
        .operands_in(vec![x, Y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "ishl",
            r#"
        Integer shift left. Shift the bits in ``x`` towards the MSB by ``y``
        places. Shift in zero bits to the LSB.

        The shift amount is masked to the size of ``x``.

        When shifting a B-bits integer type, this instruction computes:

        .. math::
            s &:= y \pmod B,                \\
            a &:= x \cdot 2^s \pmod{2^B}.
        "#,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "ushr",
            r#"
        Unsigned shift right. Shift bits in ``x`` towards the LSB by ``y``
        places, shifting in zero bits to the MSB. Also called a *logical
        shift*.

        The shift amount is masked to the size of the register.

        When shifting a B-bits integer type, this instruction computes:

        .. math::
            s &:= y \pmod B,                \\
            a &:= \lfloor x \cdot 2^{-s} \rfloor.
        "#,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "sshr",
            r#"
        Signed shift right. Shift bits in ``x`` towards the LSB by ``y``
        places, shifting in sign bits to the MSB. Also called an *arithmetic
        shift*.

        The shift amount is masked to the size of the register.
        "#,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "ishl_imm",
            r#"
        Integer shift left by immediate.

        The shift amount is masked to the size of ``x``.
        "#,
        )
        .operands_in(vec![x, Y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "ushr_imm",
            r#"
        Unsigned shift right by immediate.

        The shift amount is masked to the size of the register.
        "#,
        )
        .operands_in(vec![x, Y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "sshr_imm",
            r#"
        Signed shift right by immediate.

        The shift amount is masked to the size of the register.
        "#,
        )
        .operands_in(vec![x, Y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    let x = &operand("x", iB);
    let a = &operand("a", iB);

    ig.push(
        Inst::new(
            "bitrev",
            r#"
        Reverse the bits of a integer.

        Reverses the bits in ``x``.
        "#,
        )
        .operands_in(vec![x])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "clz",
            r#"
        Count leading zero bits.

        Starting from the MSB in ``x``, count the number of zero bits before
        reaching the first one bit. When ``x`` is zero, returns the size of x
        in bits.
        "#,
        )
        .operands_in(vec![x])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "cls",
            r#"
        Count leading sign bits.

        Starting from the MSB after the sign bit in ``x``, count the number of
        consecutive bits identical to the sign bit. When ``x`` is 0 or -1,
        returns one less than the size of x in bits.
        "#,
        )
        .operands_in(vec![x])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "ctz",
            r#"
        Count trailing zeros.

        Starting from the LSB in ``x``, count the number of zero bits before
        reaching the first one bit. When ``x`` is zero, returns the size of x
        in bits.
        "#,
        )
        .operands_in(vec![x])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "popcnt",
            r#"
        Population count

        Count the number of one bits in ``x``.
        "#,
        )
        .operands_in(vec![x])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    let Float = &TypeVar::new(
        "Float",
        "A scalar or vector floating point number",
        TypeSetBuilder::new()
            .floats(Interval::All)
            .simd_lanes(Interval::All)
            .finish(),
    );
    let Cond = &operand("Cond", floatcc);
    let x = &operand("x", Float);
    let y = &operand("y", Float);
    let a = &operand("a", &Float.as_bool());

    ig.push(
        Inst::new(
            "fcmp",
            r#"
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
        "#,
        )
        .operands_in(vec![Cond, x, y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    let f = &operand("f", fflags);

    ig.push(
        Inst::new(
            "ffcmp",
            r#"
        Floating point comparison returning flags.

        Compares two numbers like :inst:`fcmp`, but returns floating point CPU
        flags instead of testing a specific condition.
        "#,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![f])
        .finish(format_registry),
    );

    let x = &operand("x", Float);
    let y = &operand("y", Float);
    let z = &operand("z", Float);
    let a = &operand_doc("a", Float, "Result of applying operator to each lane");

    ig.push(
        Inst::new(
            "fadd",
            r#"
        Floating point addition.
        "#,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "fsub",
            r#"
        Floating point subtraction.
        "#,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "fmul",
            r#"
        Floating point multiplication.
        "#,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "fdiv",
            r#"
        Floating point division.

        Unlike the integer division instructions :clif:inst:`sdiv` and
        :clif:inst:`udiv`, this can't trap. Division by zero is infinity or
        NaN, depending on the dividend.
        "#,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "sqrt",
            r#"
        Floating point square root.
        "#,
        )
        .operands_in(vec![x])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "fma",
            r#"
        Floating point fused multiply-and-add.

        Computes :math:`a := xy+z` without any intermediate rounding of the
        product.
        "#,
        )
        .operands_in(vec![x, y, z])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    let a = &operand_doc("a", Float, "``x`` with its sign bit inverted");

    ig.push(
        Inst::new(
            "fneg",
            r#"
        Floating point negation.

        Note that this is a pure bitwise operation.
        "#,
        )
        .operands_in(vec![x])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    let a = &operand_doc("a", Float, "``x`` with its sign bit cleared");

    ig.push(
        Inst::new(
            "fabs",
            r#"
        Floating point absolute value.

        Note that this is a pure bitwise operation.
        "#,
        )
        .operands_in(vec![x])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    let a = &operand_doc(
        "a",
        Float,
        "``x`` with its sign bit changed to that of ``y``",
    );

    ig.push(
        Inst::new(
            "fcopysign",
            r#"
        Floating point copy sign.

        Note that this is a pure bitwise operation. The sign bit from ``y`` is
        copied to the sign bit of ``x``.
        "#,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    let a = &operand_doc("a", Float, "The smaller of ``x`` and ``y``");

    ig.push(
        Inst::new(
            "fmin",
            r#"
        Floating point minimum, propagating NaNs.

        If either operand is NaN, this returns a NaN.
        "#,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    let a = &operand_doc("a", Float, "The larger of ``x`` and ``y``");

    ig.push(
        Inst::new(
            "fmax",
            r#"
        Floating point maximum, propagating NaNs.

        If either operand is NaN, this returns a NaN.
        "#,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    let a = &operand_doc("a", Float, "``x`` rounded to integral value");

    ig.push(
        Inst::new(
            "ceil",
            r#"
        Round floating point round to integral, towards positive infinity.
        "#,
        )
        .operands_in(vec![x])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "floor",
            r#"
        Round floating point round to integral, towards negative infinity.
        "#,
        )
        .operands_in(vec![x])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "trunc",
            r#"
        Round floating point round to integral, towards zero.
        "#,
        )
        .operands_in(vec![x])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "nearest",
            r#"
        Round floating point round to integral, towards nearest with ties to
        even.
        "#,
        )
        .operands_in(vec![x])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    let Cond = &operand("Cond", intcc);
    let f = &operand("f", iflags);
    let a = &operand("a", b1);

    ig.push(
        Inst::new(
            "trueif",
            r#"
        Test integer CPU flags for a specific condition.

        Check the CPU flags in ``f`` against the ``Cond`` condition code and
        return true when the condition code is satisfied.
        "#,
        )
        .operands_in(vec![Cond, f])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    let Cond = &operand("Cond", floatcc);
    let f = &operand("f", fflags);

    ig.push(
        Inst::new(
            "trueff",
            r#"
        Test floating point CPU flags for a specific condition.

        Check the CPU flags in ``f`` against the ``Cond`` condition code and
        return true when the condition code is satisfied.
        "#,
        )
        .operands_in(vec![Cond, f])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    let x = &operand("x", Mem);
    let a = &operand_doc("a", MemTo, "Bits of `x` reinterpreted");

    ig.push(
        Inst::new(
            "bitcast",
            r#"
        Reinterpret the bits in `x` as a different type.

        The input and output types must be storable to memory and of the same
        size. A bitcast is equivalent to storing one type and loading the other
        type from the same address.
        "#,
        )
        .operands_in(vec![x])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    let Bool = &TypeVar::new(
        "Bool",
        "A scalar or vector boolean type",
        TypeSetBuilder::new()
            .bools(Interval::All)
            .simd_lanes(Interval::All)
            .finish(),
    );

    let BoolTo = &TypeVar::new(
        "BoolTo",
        "A smaller boolean type with the same number of lanes",
        TypeSetBuilder::new()
            .bools(Interval::All)
            .simd_lanes(Interval::All)
            .finish(),
    );

    let x = &operand("x", Bool);
    let a = &operand("a", BoolTo);

    ig.push(
        Inst::new(
            "breduce",
            r#"
        Convert `x` to a smaller boolean type in the platform-defined way.

        The result type must have the same number of vector lanes as the input,
        and each lane must not have more bits that the input lanes. If the
        input and output types are the same, this is a no-op.
        "#,
        )
        .operands_in(vec![x])
        .operands_out(vec![a])
        .constraints(vec![WiderOrEq(Bool.clone(), BoolTo.clone())])
        .finish(format_registry),
    );

    let BoolTo = &TypeVar::new(
        "BoolTo",
        "A larger boolean type with the same number of lanes",
        TypeSetBuilder::new()
            .bools(Interval::All)
            .simd_lanes(Interval::All)
            .finish(),
    );
    let x = &operand("x", Bool);
    let a = &operand("a", BoolTo);

    ig.push(
        Inst::new(
            "bextend",
            r#"
        Convert `x` to a larger boolean type in the platform-defined way.

        The result type must have the same number of vector lanes as the input,
        and each lane must not have fewer bits that the input lanes. If the
        input and output types are the same, this is a no-op.
        "#,
        )
        .operands_in(vec![x])
        .operands_out(vec![a])
        .constraints(vec![WiderOrEq(BoolTo.clone(), Bool.clone())])
        .finish(format_registry),
    );

    let IntTo = &TypeVar::new(
        "IntTo",
        "An integer type with the same number of lanes",
        TypeSetBuilder::new()
            .ints(Interval::All)
            .simd_lanes(Interval::All)
            .finish(),
    );
    let x = &operand("x", Bool);
    let a = &operand("a", IntTo);

    ig.push(
        Inst::new(
            "bint",
            r#"
        Convert `x` to an integer.

        True maps to 1 and false maps to 0. The result type must have the same
        number of vector lanes as the input.
        "#,
        )
        .operands_in(vec![x])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "bmask",
            r#"
        Convert `x` to an integer mask.

        True maps to all 1s and false maps to all 0s. The result type must have
        the same number of vector lanes as the input.
        "#,
        )
        .operands_in(vec![x])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    let Int = &TypeVar::new(
        "Int",
        "A scalar or vector integer type",
        TypeSetBuilder::new()
            .ints(Interval::All)
            .simd_lanes(Interval::All)
            .finish(),
    );

    let IntTo = &TypeVar::new(
        "IntTo",
        "A smaller integer type with the same number of lanes",
        TypeSetBuilder::new()
            .ints(Interval::All)
            .simd_lanes(Interval::All)
            .finish(),
    );
    let x = &operand("x", Int);
    let a = &operand("a", IntTo);

    ig.push(
        Inst::new(
            "ireduce",
            r#"
        Convert `x` to a smaller integer type by dropping high bits.

        Each lane in `x` is converted to a smaller integer type by discarding
        the most significant bits. This is the same as reducing modulo
        :math:`2^n`.

        The result type must have the same number of vector lanes as the input,
        and each lane must not have more bits that the input lanes. If the
        input and output types are the same, this is a no-op.
        "#,
        )
        .operands_in(vec![x])
        .operands_out(vec![a])
        .constraints(vec![WiderOrEq(Int.clone(), IntTo.clone())])
        .finish(format_registry),
    );

    let IntTo = &TypeVar::new(
        "IntTo",
        "A larger integer type with the same number of lanes",
        TypeSetBuilder::new()
            .ints(Interval::All)
            .simd_lanes(Interval::All)
            .finish(),
    );
    let x = &operand("x", Int);
    let a = &operand("a", IntTo);

    ig.push(
        Inst::new(
            "uextend",
            r#"
        Convert `x` to a larger integer type by zero-extending.

        Each lane in `x` is converted to a larger integer type by adding
        zeroes. The result has the same numerical value as `x` when both are
        interpreted as unsigned integers.

        The result type must have the same number of vector lanes as the input,
        and each lane must not have fewer bits that the input lanes. If the
        input and output types are the same, this is a no-op.
        "#,
        )
        .operands_in(vec![x])
        .operands_out(vec![a])
        .constraints(vec![WiderOrEq(IntTo.clone(), Int.clone())])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "sextend",
            r#"
        Convert `x` to a larger integer type by sign-extending.

        Each lane in `x` is converted to a larger integer type by replicating
        the sign bit. The result has the same numerical value as `x` when both
        are interpreted as signed integers.

        The result type must have the same number of vector lanes as the input,
        and each lane must not have fewer bits that the input lanes. If the
        input and output types are the same, this is a no-op.
        "#,
        )
        .operands_in(vec![x])
        .operands_out(vec![a])
        .constraints(vec![WiderOrEq(IntTo.clone(), Int.clone())])
        .finish(format_registry),
    );

    let FloatTo = &TypeVar::new(
        "FloatTo",
        "A scalar or vector floating point number",
        TypeSetBuilder::new()
            .floats(Interval::All)
            .simd_lanes(Interval::All)
            .finish(),
    );
    let x = &operand("x", Float);
    let a = &operand("a", FloatTo);

    ig.push(
        Inst::new(
            "fpromote",
            r#"
        Convert `x` to a larger floating point format.

        Each lane in `x` is converted to the destination floating point format.
        This is an exact operation.

        Cranelift currently only supports two floating point formats
        - :type:`f32` and :type:`f64`. This may change in the future.

        The result type must have the same number of vector lanes as the input,
        and the result lanes must not have fewer bits than the input lanes. If
        the input and output types are the same, this is a no-op.
        "#,
        )
        .operands_in(vec![x])
        .operands_out(vec![a])
        .constraints(vec![WiderOrEq(FloatTo.clone(), Float.clone())])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "fdemote",
            r#"
        Convert `x` to a smaller floating point format.

        Each lane in `x` is converted to the destination floating point format
        by rounding to nearest, ties to even.

        Cranelift currently only supports two floating point formats
        - :type:`f32` and :type:`f64`. This may change in the future.

        The result type must have the same number of vector lanes as the input,
        and the result lanes must not have more bits than the input lanes. If
        the input and output types are the same, this is a no-op.
        "#,
        )
        .operands_in(vec![x])
        .operands_out(vec![a])
        .constraints(vec![WiderOrEq(Float.clone(), FloatTo.clone())])
        .finish(format_registry),
    );

    let x = &operand("x", Float);
    let a = &operand("a", IntTo);

    ig.push(
        Inst::new(
            "fcvt_to_uint",
            r#"
        Convert floating point to unsigned integer.

        Each lane in `x` is converted to an unsigned integer by rounding
        towards zero. If `x` is NaN or if the unsigned integral value cannot be
        represented in the result type, this instruction traps.

        The result type must have the same number of vector lanes as the input.
        "#,
        )
        .operands_in(vec![x])
        .operands_out(vec![a])
        .can_trap(true)
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "fcvt_to_uint_sat",
            r#"
        Convert floating point to unsigned integer as fcvt_to_uint does, but
        saturates the input instead of trapping. NaN and negative values are
        converted to 0.
        "#,
        )
        .operands_in(vec![x])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "fcvt_to_sint",
            r#"
        Convert floating point to signed integer.

        Each lane in `x` is converted to a signed integer by rounding towards
        zero. If `x` is NaN or if the signed integral value cannot be
        represented in the result type, this instruction traps.

        The result type must have the same number of vector lanes as the input.
        "#,
        )
        .operands_in(vec![x])
        .operands_out(vec![a])
        .can_trap(true)
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "fcvt_to_sint_sat",
            r#"
        Convert floating point to signed integer as fcvt_to_sint does, but
        saturates the input instead of trapping. NaN values are converted to 0.
        "#,
        )
        .operands_in(vec![x])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    let x = &operand("x", Int);
    let a = &operand("a", FloatTo);

    ig.push(
        Inst::new(
            "fcvt_from_uint",
            r#"
        Convert unsigned integer to floating point.

        Each lane in `x` is interpreted as an unsigned integer and converted to
        floating point using round to nearest, ties to even.

        The result type must have the same number of vector lanes as the input.
        "#,
        )
        .operands_in(vec![x])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    ig.push(
        Inst::new(
            "fcvt_from_sint",
            r#"
        Convert signed integer to floating point.

        Each lane in `x` is interpreted as a signed integer and converted to
        floating point using round to nearest, ties to even.

        The result type must have the same number of vector lanes as the input.
        "#,
        )
        .operands_in(vec![x])
        .operands_out(vec![a])
        .finish(format_registry),
    );

    let WideInt = &TypeVar::new(
        "WideInt",
        "An integer type with lanes from `i16` upwards",
        TypeSetBuilder::new()
            .ints(16..64)
            .simd_lanes(Interval::All)
            .finish(),
    );
    let x = &operand("x", WideInt);
    let lo = &operand_doc("lo", &WideInt.half_width(), "The low bits of `x`");
    let hi = &operand_doc("hi", &WideInt.half_width(), "The high bits of `x`");

    ig.push(
        Inst::new(
            "isplit",
            r#"
        Split an integer into low and high parts.

        Vectors of integers are split lane-wise, so the results have the same
        number of lanes as the input, but the lanes are half the size.

        Returns the low half of `x` and the high half of `x` as two independent
        values.
        "#,
        )
        .operands_in(vec![x])
        .operands_out(vec![lo, hi])
        .is_ghost(true)
        .finish(format_registry),
    );

    let NarrowInt = &TypeVar::new(
        "NarrowInt",
        "An integer type with lanes type to `i32`",
        TypeSetBuilder::new()
            .ints(8..32)
            .simd_lanes(Interval::All)
            .finish(),
    );

    let lo = &operand("lo", NarrowInt);
    let hi = &operand("hi", NarrowInt);
    let a = &operand_doc(
        "a",
        &NarrowInt.double_width(),
        "The concatenation of `lo` and `hi`",
    );

    ig.push(
        Inst::new(
            "iconcat",
            r#"
        Concatenate low and high bits to form a larger integer type.

        Vectors of integers are concatenated lane-wise such that the result has
        the same number of lanes as the inputs, but the lanes are twice the
        size.
        "#,
        )
        .operands_in(vec![lo, hi])
        .operands_out(vec![a])
        .is_ghost(true)
        .finish(format_registry),
    );

    ig
}
