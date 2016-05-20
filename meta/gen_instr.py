"""
Generate sources with instruction info.
"""

import srcgen
import constant_hash
from unique_table import UniqueTable, UniqueSeqTable
import cretonne


def gen_formats(fmt):
    """Generate an instruction format enumeration"""

    fmt.doc_comment('An instruction format')
    fmt.doc_comment('')
    fmt.doc_comment('Every opcode has a corresponding instruction format')
    fmt.doc_comment('which is represented by both the `InstructionFormat`')
    fmt.doc_comment('and the `InstructionData` enums.')
    fmt.line('#[derive(Copy, Clone, PartialEq, Eq, Debug)]')
    with fmt.indented('pub enum InstructionFormat {', '}'):
        for f in cretonne.InstructionFormat.all_formats:
            fmt.line(f.name + ',')
    fmt.line()

    # Emit a From<InstructionData> which also serves to verify that
    # InstructionFormat and InstructionData are in sync.
    with fmt.indented(
            "impl<'a> From<&'a InstructionData> for InstructionFormat {", '}'):
        with fmt.indented(
                "fn from(inst: &'a InstructionData) -> InstructionFormat {",
                '}'):
            with fmt.indented('match *inst {', '}'):
                for f in cretonne.InstructionFormat.all_formats:
                    fmt.line(('InstructionData::{} {{ .. }} => ' +
                              'InstructionFormat::{},')
                             .format(f.name, f.name))
    fmt.line()


def gen_instruction_data_impl(fmt):
    """
    Generate the boring parts of the InstructionData implementation.

    These methods in `impl InstructionData` can be generated automatically from
    the instruction formats:

    - `pub fn opcode(&self) -> Opcode`
    - `pub fn first_type(&self) -> Type`
    - `pub fn second_result(&self) -> Option<Value>`
    - `pub fn second_result_mut<'a>(&'a mut self) -> Option<&'a mut Value>`
    """

    # The `opcode` and `first_type` methods simply read the `opcode` and `ty`
    # members. This is really a workaround for Rust's enum types missing shared
    # members.
    with fmt.indented('impl InstructionData {', '}'):
        fmt.doc_comment('Get the opcode of this instruction.')
        with fmt.indented('pub fn opcode(&self) -> Opcode {', '}'):
            with fmt.indented('match *self {', '}'):
                for f in cretonne.InstructionFormat.all_formats:
                    fmt.line(
                            'InstructionData::{} {{ opcode, .. }} => opcode,'
                            .format(f.name))

        fmt.doc_comment('Type of the first result, or `VOID`.')
        with fmt.indented('pub fn first_type(&self) -> Type {', '}'):
            with fmt.indented('match *self {', '}'):
                for f in cretonne.InstructionFormat.all_formats:
                    fmt.line(
                            'InstructionData::{} {{ ty, .. }} => ty,'
                            .format(f.name))

        # Generate shared and mutable accessors for `second_result` which only
        # applies to instruction formats that can produce multiple results.
        # Everything else returns `None`.
        fmt.doc_comment('Second result value, if any.')
        with fmt.indented(
                'pub fn second_result(&self) -> Option<Value> {', '}'):
            with fmt.indented('match *self {', '}'):
                for f in cretonne.InstructionFormat.all_formats:
                    if not f.multiple_results:
                        # Single or no results.
                        fmt.line(
                                'InstructionData::{} {{ .. }} => None,'
                                .format(f.name))
                    elif f.boxed_storage:
                        # Multiple results, boxed storage.
                        fmt.line(
                                'InstructionData::' + f.name +
                                ' { ref data, .. }' +
                                ' => Some(data.second_result),')
                    else:
                        # Multiple results, inline storage.
                        fmt.line(
                                'InstructionData::' + f.name +
                                ' { second_result, .. }' +
                                ' => Some(second_result),')

        fmt.doc_comment('Mutable reference to second result value, if any.')
        with fmt.indented(
                "pub fn second_result_mut<'a>(&'a mut self)" +
                " -> Option<&'a mut Value> {", '}'):
            with fmt.indented('match *self {', '}'):
                for f in cretonne.InstructionFormat.all_formats:
                    if not f.multiple_results:
                        # Single or no results.
                        fmt.line(
                                'InstructionData::{} {{ .. }} => None,'
                                .format(f.name))
                    elif f.boxed_storage:
                        # Multiple results, boxed storage.
                        fmt.line(
                                'InstructionData::' + f.name +
                                ' { ref mut data, .. }' +
                                ' => Some(&mut data.second_result),')
                    else:
                        # Multiple results, inline storage.
                        fmt.line(
                                'InstructionData::' + f.name +
                                ' { ref mut second_result, .. }' +
                                ' => Some(second_result),')


def collect_instr_groups(targets):
    seen = set()
    groups = []
    for t in targets:
        for g in t.instruction_groups:
            if g not in seen:
                groups.append(g)
                seen.add(g)
    return groups


def gen_opcodes(groups, fmt):
    """
    Generate opcode enumerations.

    Return a list of all instructions.
    """

    fmt.doc_comment('An instruction opcode.')
    fmt.doc_comment('')
    fmt.doc_comment('All instructions from all supported targets are present.')
    fmt.line('#[derive(Copy, Clone, PartialEq, Eq, Debug)]')
    instrs = []
    with fmt.indented('pub enum Opcode {', '}'):
        fmt.line('NotAnOpcode,')
        for g in groups:
            for i in g.instructions:
                instrs.append(i)
                # Build a doc comment.
                prefix = ', '.join(o.name for o in i.outs)
                if prefix:
                    prefix = prefix + ' = '
                suffix = ', '.join(o.name for o in i.ins)
                fmt.doc_comment(
                        '`{}{} {}`. ({})'
                        .format(prefix, i.name, suffix, i.format.name))
                # Document polymorphism.
                if i.is_polymorphic:
                    if i.use_typevar_operand:
                        fmt.doc_comment(
                                'Type inferred from {}.'
                                .format(i.ins[i.format.typevar_operand]))
                # Enum variant itself.
                fmt.line(i.camel_name + ',')
    fmt.line()

    # Generate a private opcode_format table.
    with fmt.indented(
            'const OPCODE_FORMAT: [InstructionFormat; {}] = ['
            .format(len(instrs)),
            '];'):
        for i in instrs:
            fmt.format(
                    'InstructionFormat::{}, // {}',
                    i.format.name, i.name)
    fmt.line()

    # Generate a private opcode_name function.
    with fmt.indented('fn opcode_name(opc: Opcode) -> &\'static str {', '}'):
        with fmt.indented('match opc {', '}'):
            fmt.line('Opcode::NotAnOpcode => "<not an opcode>",')
            for i in instrs:
                fmt.format('Opcode::{} => "{}",', i.camel_name, i.name)
    fmt.line()

    # Generate an opcode hash table for looking up opcodes by name.
    hash_table = constant_hash.compute_quadratic(
            instrs,
            lambda i: constant_hash.simple_hash(i.name))
    with fmt.indented(
            'const OPCODE_HASH_TABLE: [Opcode; {}] = ['
            .format(len(hash_table)), '];'):
        for i in hash_table:
            if i is None:
                fmt.line('Opcode::NotAnOpcode,')
            else:
                fmt.format('Opcode::{},', i.camel_name)
    fmt.line()
    return instrs


def get_constraint(op, ctrl_typevar, type_sets):
    """
    Get the value type constraint for an SSA value operand, where
    `ctrl_typevar` is the controlling type variable.

    Each operand constraint is represented as a string, one of:

    - `Concrete(vt)`, where `vt` is a value type name.
    - `Free(idx)` where `idx` is an index into `type_sets`.
    - `Same`, `Lane`, `AsBool` for controlling typevar-derived constraints.
    """
    t = op.typ
    assert t.operand_kind() is cretonne.value

    # A concrete value type.
    if isinstance(t, cretonne.ValueType):
        return 'Concrete({})'.format(t.rust_name())

    if t.free_typevar() is not ctrl_typevar:
        assert not t.is_derived
        return 'Free({})'.format(type_sets.add(t.type_set))

    if t.is_derived:
        assert t.base is ctrl_typevar, "Not derived directly from ctrl_typevar"
        return t.derived_func

    assert t is ctrl_typevar
    return 'Same'


def gen_type_constraints(fmt, instrs):
    """
    Generate value type constraints for all instructions.

    - Emit a compact constant table of ValueTypeSet objects.
    - Emit a compact constant table of OperandConstraint objects.
    - Emit an opcode-indexed table of instruction constraints.

    """

    # Table of TypeSet instances.
    type_sets = UniqueTable()

    # Table of operand constraint sequences (as tuples). Each operand
    # constraint is represented as a string, one of:
    # - `Concrete(vt)`, where `vt` is a value type name.
    # - `Free(idx)` where `idx` isan index into `type_sets`.
    # - `Same`, `Lane`, `AsBool` for controlling typevar-derived constraints.
    operand_seqs = UniqueSeqTable()

    # Preload table with constraints for typical binops.
    operand_seqs.add(['Same'] * 3)

    # TypeSet indexes are encoded in 3 bits, with `111` reserved.
    typeset_limit = 7

    fmt.comment('Table of opcode constraints.')
    with fmt.indented(
            'const OPCODE_CONSTRAINTS : [OpcodeConstraints; {}] = ['
            .format(len(instrs)), '];'):
        for i in instrs:
            # Collect constraints for the value results, not including
            # `variable_args` results which are always special cased.
            constraints = list()
            ctrl_typevar = None
            ctrl_typeset = typeset_limit
            if i.is_polymorphic:
                ctrl_typevar = i.ctrl_typevar
                ctrl_typeset = type_sets.add(ctrl_typevar.type_set)
            for idx in i.value_results:
                constraints.append(
                        get_constraint(i.outs[idx], ctrl_typevar, type_sets))
            for idx in i.format.value_operands:
                constraints.append(
                        get_constraint(i.ins[idx], ctrl_typevar, type_sets))
            offset = operand_seqs.add(constraints)
            fixed_results = len(i.value_results)
            use_typevar_operand = i.is_polymorphic and i.use_typevar_operand
            fmt.comment(
                    '{}: fixed_results={}, use_typevar_operand={}'
                    .format(i.camel_name, fixed_results, use_typevar_operand))
            fmt.comment('Constraints={}'.format(constraints))
            if i.is_polymorphic:
                fmt.comment(
                        'Polymorphic over {}'.format(ctrl_typevar.type_set))
            # Compute the bit field encoding, c.f. instructions.rs.
            assert fixed_results < 8, "Bit field encoding too tight"
            bits = (offset << 8) | (ctrl_typeset << 4) | fixed_results
            if use_typevar_operand:
                bits |= 8
            assert bits < 0x10000, "Constraint table too large for bit field"
            fmt.line('OpcodeConstraints({:#06x}),'.format(bits))

    fmt.comment('Table of value type sets.')
    assert len(type_sets.table) <= typeset_limit, "Too many type sets"
    with fmt.indented(
            'const TYPE_SETS : [ValueTypeSet; {}] = ['
            .format(len(type_sets.table)), '];'):
        for ts in type_sets.table:
            with fmt.indented('ValueTypeSet {', '},'):
                if ts.base:
                    fmt.line('base: {},'.format(ts.base.rust_name()))
                else:
                    fmt.line('base: types::VOID,')
                for field in ts._fields:
                    if field == 'base':
                        continue
                    fmt.line('{}: {},'.format(
                        field, str(getattr(ts, field)).lower()))

    fmt.comment('Table of operand constraint sequences.')
    with fmt.indented(
            'const OPERAND_CONSTRAINTS : [OperandConstraint; {}] = ['
            .format(len(operand_seqs.table)), '];'):
        for c in operand_seqs.table:
            fmt.line('OperandConstraint::{},'.format(c))


def generate(targets, out_dir):
    groups = collect_instr_groups(targets)

    # opcodes.rs
    fmt = srcgen.Formatter()
    gen_formats(fmt)
    gen_instruction_data_impl(fmt)
    instrs = gen_opcodes(groups, fmt)
    gen_type_constraints(fmt, instrs)
    fmt.update_file('opcodes.rs', out_dir)
