"""
Generate sources with instruction info.
"""

import srcgen

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
    fmt.line('enum Opcode {')
    fmt.indent_push()
    for g in groups:
        for i in g.instructions:
            fmt.line(i.camel_name + ',')
    fmt.indent_pop()
    fmt.line('}')
    fmt.update_file('opcodes.rs', out_dir)

def generate(targets, out_dir):
    groups = collect_instr_groups(targets)
    gen_opcodes(groups, out_dir)
