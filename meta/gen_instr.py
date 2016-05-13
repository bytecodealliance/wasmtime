"""
Generate sources with instruction info.
"""

import srcgen
import constant_hash


def collect_instr_groups(targets):
    seen = set()
    groups = []
    for t in targets:
        for g in t.instruction_groups:
            if g not in seen:
                groups.append(g)
                seen.add(g)
    return groups


def gen_opcodes(groups, out_dir):
    """Generate opcode enumerations."""
    fmt = srcgen.Formatter()

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
                # Enum variant itself.
                fmt.line(i.camel_name + ',')

    # Generate a private opcode_name function.
    with fmt.indented('fn opcode_name(opc: Opcode) -> &\'static str {', '}'):
        with fmt.indented('match opc {', '}'):
            fmt.line('Opcode::NotAnOpcode => "<not an opcode>",')
            for i in instrs:
                fmt.format('Opcode::{} => "{}",', i.camel_name, i.name)

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

    fmt.update_file('opcodes.rs', out_dir)


def generate(targets, out_dir):
    groups = collect_instr_groups(targets)
    gen_opcodes(groups, out_dir)
