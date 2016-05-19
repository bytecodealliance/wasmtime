"""
Generate sources with instruction info.
"""

import srcgen
import constant_hash
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
                "pub fn second_result_mut<'a>(&'a mut self) -> Option<&'a mut Value> {", '}'):
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
    """Generate opcode enumerations."""

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


def generate(targets, out_dir):
    groups = collect_instr_groups(targets)

    # opcodes.rs
    fmt = srcgen.Formatter()
    gen_formats(fmt)
    gen_instruction_data_impl(fmt)
    gen_opcodes(groups, fmt)
    fmt.update_file('opcodes.rs', out_dir)
