"""
Generate sources for instruction encoding.

The tables and functions generated here support the `TargetIsa::encode()`
function which determines if a given instruction is legal, and if so, it's
`Encoding` data which consists of a *recipe* and some *encoding* bits.

The `encode` function doesn't actually generate the binary machine bits. Each
recipe has a corresponding hand-written function to do that after registers
are allocated.

This is the information available to us:

- The instruction to be encoded as an `Inst` reference.
- The data-flow graph containing the instruction, giving us access to the
  `InstructionData` representation and the types of all values involved.
- A target ISA instance with shared and ISA-specific settings for evaluating
  ISA predicates.
- The currently active CPU mode is determined by the ISA.

## Level 1 table lookup

The CPU mode provides the first table. The key is the instruction's controlling
type variable. If the instruction is not polymorphic, use `VOID` for the type
variable. The table values are level 2 tables.

## Level 2 table lookup

The level 2 table is keyed by the instruction's opcode. The table values are
*encoding lists*.

The two-level table lookup allows the level 2 tables to be much smaller with
good locality. Code in any given function usually only uses a few different
types, so many of the level 2 tables will be cold.

## Encoding lists

An encoding list is a non-empty sequence of list entries. Each entry has
one of these forms:

1. Instruction predicate, encoding recipe, and encoding bits. If the
   instruction predicate is true, use this recipe and bits.
2. ISA predicate and skip-count. If the ISA predicate is false, skip the next
   *skip-count* entries in the list. If the skip count is zero, stop
   completely.
3. Stop. End of list marker. If this is reached, the instruction does not have
   a legal encoding.

The instruction predicate is also used to distinguish between polymorphic
instructions with different types for secondary type variables.
"""
from __future__ import absolute_import
import srcgen
from collections import OrderedDict


def emit_instp(instp, fmt):
    """
    Emit code for matching an instruction predicate against an
    `InstructionData` reference called `inst`.

    The generated code is a pattern match that falls through if the instruction
    has an unexpected format. This should lead to a panic.
    """
    iform = instp.predicate_context()

    # Which fiels do we need in the InstructionData pattern match?
    if iform.boxed_storage:
        fields = 'ref data'
    else:
        # Collect the leaf predicates
        leafs = set()
        instp.predicate_leafs(leafs)
        # All the leafs are FieldPredicate instances. Here we just care about
        # the field names.
        fields = ', '.join(sorted(set(p.field.name for p in leafs)))

    with fmt.indented(
            'if let {} {{ {}, .. }} = *inst {{'
            .format(iform.name, fields), '}'):
        fmt.line('return {};'.format(instp.rust_predicate(0)))


def emit_instps(instps, fmt):
    """
    Emit a function for matching instruction predicates.
    """

    with fmt.indented(
            'fn check_instp(inst: &InstructionData, instp_idx: u16) -> bool {',
            '}'):
        with fmt.indented('match instp_idx {', '}'):
            for (instp, idx) in instps.items():
                with fmt.indented('{} => {{'.format(idx), '}'):
                    emit_instp(instp, fmt)
            fmt.line('_ => panic!("Invalid instruction predicate")')

        # The match cases will fall through if the instruction format is wrong.
        fmt.line('panic!("Bad format {}/{} for instp {}",')
        fmt.line('       InstructionFormat::from(inst),')
        fmt.line('       inst.opcode(),')
        fmt.line('       instp_idx);')


def collect_instps(cpumodes):
    # Map instp -> number
    instps = OrderedDict()
    for cpumode in cpumodes:
        for enc in cpumode.encodings:
            instp = enc.instp
            if instp and instp not in instps:
                instps[instp] = 1 + len(instps)
    return instps


class EncList(object):
    """
    List of instructions for encoding a given type + opcode pair.

    An encoding list contains a sequence of predicates and encoding recipes,
    all encoded as u16 values.

    :param inst: The instruction opcode being encoded.
    :param ty: Value of the controlling type variable, or `None`.
    """

    def __init__(self, inst, ty):
        self.inst = inst
        self.ty = ty
        # List of applicable Encoding instances.
        # These will have different predicates.
        self.encodings = []

    def name(self):
        if self.ty:
            return '{}.{}'.format(self.inst.name, self.ty.name)
        else:
            return self.inst.name


class Level2Table(object):
    """
    Level 2 table mapping instruction opcodes to `EncList` objects.

    :param ty: Controlling type variable of all entries, or `None`.
    """

    def __init__(self, ty):
        self.ty = ty
        # Maps inst -> EncList
        self.lists = OrderedDict()

    def __getitem__(self, inst):
        ls = self.lists.get(inst)
        if not ls:
            ls = EncList(inst, self.ty)
            self.lists[inst] = ls
        return ls

    def __iter__(self):
        return iter(self.lists.values())


class Level1Table(object):
    """
    Level 1 table mapping types to `Level2` objects.
    """

    def __init__(self):
        self.tables = OrderedDict()

    def __getitem__(self, ty):
        tbl = self.tables.get(ty)
        if not tbl:
            tbl = Level2Table(ty)
            self.tables[ty] = tbl
        return tbl

    def __iter__(self):
        return iter(self.tables.values())


def make_tables(cpumode):
    """
    Generate tables for `cpumode` as described above.
    """
    table = Level1Table()
    for enc in cpumode.encodings:
        ty = enc.ctrl_typevar()
        inst = enc.inst
        table[ty][inst].encodings.append(enc)
    return table


def gen_isa(cpumodes, fmt):
    # First assign numbers to relevant instruction predicates and generate the
    # check_instp() function..
    instps = collect_instps(cpumodes)
    emit_instps(instps, fmt)

    for cpumode in cpumodes:
        level1 = make_tables(cpumode)
        for level2 in level1:
            for enclist in level2:
                fmt.comment(enclist.name())
                for enc in enclist.encodings:
                    fmt.comment('{} when {}'.format(enc, enc.instp))


def generate(isas, out_dir):
    for isa in isas:
        fmt = srcgen.Formatter()
        gen_isa(isa.cpumodes, fmt)
        fmt.update_file('encoding-{}.rs'.format(isa.name), out_dir)
