"""
Generate sources for instruction encoding.

The tables and functions generated here support the `TargetISA::encode()`
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
from constant_hash import compute_quadratic
from unique_table import UniqueSeqTable
from collections import OrderedDict, defaultdict
import math
import itertools
from cdsl.registers import RegClass, Register
from cdsl.predicates import FieldPredicate

try:
    from typing import Sequence, Set, Tuple, List, Iterable, DefaultDict, TYPE_CHECKING  # noqa
    if TYPE_CHECKING:
        from cdsl.isa import TargetISA, OperandConstraint, Encoding, CPUMode  # noqa
        from cdsl.predicates import PredNode, PredLeaf  # noqa
        from cdsl.types import ValueType  # noqa
        from cdsl.instructions import Instruction  # noqa
except ImportError:
    pass


def emit_instp(instp, fmt):
    # type: (PredNode, srcgen.Formatter) -> None
    """
    Emit code for matching an instruction predicate against an
    `InstructionData` reference called `inst`.

    The generated code is a pattern match that falls through if the instruction
    has an unexpected format. This should lead to a panic.
    """
    iform = instp.predicate_context()

    # Which fields do we need in the InstructionData pattern match?
    # Collect the leaf predicates.
    leafs = set()  # type: Set[PredLeaf]
    instp.predicate_leafs(leafs)
    # All the leafs are FieldPredicate instances. Here we just care about
    # the field names.
    fnames = set()  # type: Set[str]
    for p in leafs:
        assert isinstance(p, FieldPredicate)
        fnames.add(p.field.name)
    fields = ', '.join(sorted(fnames))

    with fmt.indented('{} => {{'.format(instp.number), '}'):
        with fmt.indented(
                'if let InstructionData::{} {{ {}, .. }} = *inst {{'
                .format(iform.name, fields), '}'):
            fmt.line('return {};'.format(instp.rust_predicate(0)))


def emit_instps(instps, fmt):
    # type: (Sequence[PredNode], srcgen.Formatter) -> None
    """
    Emit a function for matching instruction predicates.
    """

    if not instps:
        # If the ISA has no predicates, just emit a stub.
        with fmt.indented(
                'pub fn check_instp(_: &InstructionData, _: u16) ' +
                '-> bool {', '}'):
            fmt.line('unimplemented!()')
        return

    with fmt.indented(
            'pub fn check_instp(inst: &InstructionData, instp_idx: u16) ' +
            '-> bool {', '}'):
        # The matches emitted by `emit_instp` need this.
        fmt.line('use ir::instructions::InstructionFormat;')
        with fmt.indented('match instp_idx {', '}'):
            for instp in instps:
                emit_instp(instp, fmt)
            fmt.line('_ => panic!("Invalid instruction predicate")')

        # The match cases will fall through if the instruction format is wrong.
        fmt.line('panic!("Bad format {:?}/{} for instp {}",')
        fmt.line('       InstructionFormat::from(inst),')
        fmt.line('       inst.opcode(),')
        fmt.line('       instp_idx);')


# Encoding lists are represented as u16 arrays.
CODE_BITS = 16
PRED_BITS = 12
PRED_MASK = (1 << PRED_BITS) - 1

# 0..CODE_ALWAYS means: Check instruction predicate and use the next two
# entries as a (recipe, encbits) pair if true. CODE_ALWAYS is the always-true
# predicate, smaller numbers refer to instruction predicates.
CODE_ALWAYS = PRED_MASK

# Codes above CODE_ALWAYS indicate an ISA predicate to be tested.
# `x & PRED_MASK` is the ISA predicate number to test.
# `(x >> PRED_BITS)*3` is the number of u16 table entries to skip if the ISA
# predicate is false. (The factor of three corresponds to the (inst-pred,
# recipe, encbits) triples.
#
# Finally, CODE_FAIL indicates the end of the list.
CODE_FAIL = (1 << CODE_BITS) - 1


def seq_doc(enc):
    # type: (Encoding) -> Tuple[Tuple[int, int, int], str]
    """
    Return a tuple containing u16 representations of the instruction predicate
    an recipe / encbits.

    Also return a doc string.
    """
    if enc.instp:
        p = enc.instp.number
        doc = '--> {} when {}'.format(enc, enc.instp)
    else:
        p = CODE_ALWAYS
        doc = '--> {}'.format(enc)
    assert p <= CODE_ALWAYS
    return ((p, enc.recipe.number, enc.encbits), doc)


class EncList(object):
    """
    List of instructions for encoding a given type + opcode pair.

    An encoding list contains a sequence of predicates and encoding recipes,
    all encoded as u16 values.

    :param inst: The instruction opcode being encoded.
    :param ty: Value of the controlling type variable, or `None`.
    """

    def __init__(self, inst, ty):
        # type: (Instruction, ValueType) -> None
        self.inst = inst
        self.ty = ty
        # List of applicable Encoding instances.
        # These will have different predicates.
        self.encodings = []  # type: List[Encoding]

    def name(self):
        # type: () -> str
        name = self.inst.name
        if self.ty:
            name = '{}.{}'.format(name, self.ty.name)
        if self.encodings:
            name += ' ({})'.format(self.encodings[0].cpumode)
        return name

    def by_isap(self):
        # type: () -> Iterable[Tuple[PredNode, Tuple[Encoding, ...]]]
        """
        Group the encodings by ISA predicate without reordering them.

        Yield a sequence of `(isap, (encs...))` tuples where `isap` is the ISA
        predicate or `None`, and `(encs...)` is a tuple of encodings that all
        have the same ISA predicate.
        """
        maxlen = CODE_FAIL >> PRED_BITS
        for isap, groupi in itertools.groupby(
                self.encodings, lambda enc: enc.isap):
            group = tuple(groupi)
            # This probably never happens, but we can't express more than
            # maxlen encodings per isap.
            while len(group) > maxlen:
                yield (isap, group[0:maxlen])
                group = group[maxlen:]
            yield (isap, group)

    def encode(self, seq_table, doc_table, isa):
        # type: (UniqueSeqTable, DefaultDict[int, List[str]], TargetISA) -> None  # noqa
        """
        Encode this list as a sequence of u16 numbers.

        Adds the sequence to `seq_table` and records the returned offset as
        `self.offset`.

        Adds comment lines to `doc_table` keyed by seq_table offsets.
        """
        words = list()  # type: List[int]
        docs = list()  # type: List[Tuple[int, str]]

        # Group our encodings by isap.
        for isap, group in self.by_isap():
            if isap:
                # We have an ISA predicate covering `glen` encodings.
                pnum = isa.settings.predicate_number[isap]
                glen = len(group)
                doc = 'skip {}x3 unless {}'.format(glen, isap)
                docs.append((len(words), doc))
                words.append((glen << PRED_BITS) | pnum)

            for enc in group:
                seq, doc = seq_doc(enc)
                docs.append((len(words), doc))
                words.extend(seq)

        # Terminate the list.
        words.append(CODE_FAIL)

        self.offset = seq_table.add(words)

        # Add doc comments.
        doc_table[self.offset].append(
                '{:06x}: {}'.format(self.offset, self.name()))
        for pos, doc in docs:
            doc_table[self.offset + pos].append(doc)


class Level2Table(object):
    """
    Level 2 table mapping instruction opcodes to `EncList` objects.

    :param ty: Controlling type variable of all entries, or `None`.
    """

    def __init__(self, ty):
        # type: (ValueType) -> None
        self.ty = ty
        # Maps inst -> EncList
        self.lists = OrderedDict()  # type: OrderedDict[Instruction, EncList]

    def __getitem__(self, inst):
        # type: (Instruction) -> EncList
        ls = self.lists.get(inst)
        if not ls:
            ls = EncList(inst, self.ty)
            self.lists[inst] = ls
        return ls

    def enclists(self):
        # type: () -> Iterable[EncList]
        return iter(self.lists.values())

    def layout_hashtable(self, level2_hashtables, level2_doc):
        # type: (List[EncList], DefaultDict[int, List[str]]) -> None
        """
        Compute the hash table mapping opcode -> enclist.

        Append the hash table to `level2_hashtables` and record the offset.
        """
        hash_table = compute_quadratic(
                self.lists.values(),
                lambda enclist: enclist.inst.number)

        self.hash_table_offset = len(level2_hashtables)
        self.hash_table_len = len(hash_table)

        level2_doc[self.hash_table_offset].append(
                '{:06x}: {}, {} entries'.format(
                    self.hash_table_offset,
                    self.ty.name,
                    self.hash_table_len))
        level2_hashtables.extend(hash_table)


class Level1Table(object):
    """
    Level 1 table mapping types to `Level2` objects.
    """

    def __init__(self):
        # type: () -> None
        self.tables = OrderedDict()  # type: OrderedDict[ValueType, Level2Table]  # noqa

    def __getitem__(self, ty):
        # type: (ValueType) -> Level2Table
        tbl = self.tables.get(ty)
        if not tbl:
            tbl = Level2Table(ty)
            self.tables[ty] = tbl
        return tbl

    def l2tables(self):
        # type: () -> Iterable[Level2Table]
        return iter(self.tables.values())


def make_tables(cpumode):
    # type: (CPUMode) -> Level1Table
    """
    Generate tables for `cpumode` as described above.
    """
    table = Level1Table()
    for enc in cpumode.encodings:
        ty = enc.ctrl_typevar()
        inst = enc.inst
        table[ty][inst].encodings.append(enc)
    return table


def encode_enclists(level1, seq_table, doc_table, isa):
    # type: (Level1Table, UniqueSeqTable, DefaultDict[int, List[str]], TargetISA) -> None  # noqa
    """
    Compute encodings and doc comments for encoding lists in `level1`.
    """
    for level2 in level1.l2tables():
        for enclist in level2.enclists():
            enclist.encode(seq_table, doc_table, isa)


def emit_enclists(seq_table, doc_table, fmt):
    # type: (UniqueSeqTable, DefaultDict[int, List[str]], srcgen.Formatter) -> None  # noqa
    with fmt.indented(
            'pub static ENCLISTS: [u16; {}] = ['.format(len(seq_table.table)),
            '];'):
        line = ''
        for idx, entry in enumerate(seq_table.table):
            if idx in doc_table:
                if line:
                    fmt.line(line)
                    line = ''
                for doc in doc_table[idx]:
                    fmt.comment(doc)
            line += '{:#06x}, '.format(entry)
        if line:
            fmt.line(line)


def encode_level2_hashtables(level1, level2_hashtables, level2_doc):
    # type: (Level1Table, List[EncList], DefaultDict[int, List[str]]) -> None
    for level2 in level1.l2tables():
        level2.layout_hashtable(level2_hashtables, level2_doc)


def emit_level2_hashtables(level2_hashtables, offt, level2_doc, fmt):
    # type: (List[EncList], str, DefaultDict[int, List[str]], srcgen.Formatter) -> None  # noqa
    """
    Emit the big concatenation of level 2 hash tables.
    """
    with fmt.indented(
            'pub static LEVEL2: [Level2Entry<{}>; {}] = ['
            .format(offt, len(level2_hashtables)),
            '];'):
        for offset, entry in enumerate(level2_hashtables):
            if offset in level2_doc:
                for doc in level2_doc[offset]:
                    fmt.comment(doc)
            if entry:
                fmt.line(
                        'Level2Entry ' +
                        '{{ opcode: Some(Opcode::{}), offset: {:#08x} }},'
                        .format(entry.inst.camel_name, entry.offset))
            else:
                fmt.line(
                        'Level2Entry ' +
                        '{ opcode: None, offset: 0 },')


def emit_level1_hashtable(cpumode, level1, offt, fmt):
    # type: (CPUMode, Level1Table, str, srcgen.Formatter) -> None  # noqa
    """
    Emit a level 1 hash table for `cpumode`.
    """
    hash_table = compute_quadratic(
            level1.tables.values(),
            lambda level2: level2.ty.number)

    with fmt.indented(
            'pub static LEVEL1_{}: [Level1Entry<{}>; {}] = ['
            .format(cpumode.name.upper(), offt, len(hash_table)), '];'):
        for level2 in hash_table:
            if level2:
                l2l = int(math.log(level2.hash_table_len, 2))
                assert l2l > 0, "Hash table too small"
                fmt.line(
                        'Level1Entry ' +
                        '{{ ty: types::{}, log2len: {}, offset: {:#08x} }},'
                        .format(
                            level2.ty.name.upper(),
                            l2l,
                            level2.hash_table_offset))
            else:
                # Empty entry.
                fmt.line(
                        'Level1Entry ' +
                        '{ ty: types::VOID, log2len: 0, offset: 0 },')


def offset_type(length):
    # type: (int) -> str
    """
    Compute an appropriate Rust integer type to use for offsets into a table of
    the given length.
    """
    if length <= 0x10000:
        return 'u16'
    else:
        assert length <= 0x100000000, "Table too big"
        return 'u32'


def emit_recipe_names(isa, fmt):
    # type: (TargetISA, srcgen.Formatter) -> None
    """
    Emit a table of encoding recipe names keyed by recipe number.

    This is used for pretty-printing encodings.
    """
    with fmt.indented(
            'pub static RECIPE_NAMES: [&\'static str; {}] = ['
            .format(len(isa.all_recipes)), '];'):
        for r in isa.all_recipes:
            fmt.line('"{}",'.format(r.name))


def emit_recipe_constraints(isa, fmt):
    # type: (TargetISA, srcgen.Formatter) -> None
    """
    Emit a table of encoding recipe operand constraints keyed by recipe number.

    These are used by the register allocator to pick registers that can be
    properly encoded.
    """
    with fmt.indented(
            'pub static RECIPE_CONSTRAINTS: [RecipeConstraints; {}] = ['
            .format(len(isa.all_recipes)), '];'):
        for r in isa.all_recipes:
            fmt.comment(r.name)
            with fmt.indented('RecipeConstraints {', '},'):
                emit_operand_constraints(r.ins, 'ins', fmt)
                emit_operand_constraints(r.outs, 'outs', fmt)


def emit_operand_constraints(seq, field, fmt):
    # type: (Sequence[OperandConstraint], str, srcgen.Formatter) -> None
    """
    Emit a struct field initializer for an array of operand constraints.
    """
    if len(seq) == 0:
        fmt.line('{}: &[],'.format(field))
        return
    with fmt.indented('{}: &['.format(field), '],'):
        for cons in seq:
            with fmt.indented('OperandConstraint {', '},'):
                if isinstance(cons, RegClass):
                    fmt.line('kind: ConstraintKind::Reg,')
                    fmt.line('regclass: {},'.format(cons))
                elif isinstance(cons, Register):
                    fmt.line(
                            'kind: ConstraintKind::FixedReg({}),'
                            .format(cons.unit))
                    fmt.line('regclass: {},'.format(cons.regclass))
                else:
                    raise AssertionError(
                            'Unsupported constraint {}'.format(cons))


def gen_isa(isa, fmt):
    # type: (TargetISA, srcgen.Formatter) -> None
    # First assign numbers to relevant instruction predicates and generate the
    # check_instp() function..
    emit_instps(isa.all_instps, fmt)

    # Level1 tables, one per CPU mode
    level1_tables = dict()

    # Tables for enclists with comments.
    seq_table = UniqueSeqTable()
    doc_table = defaultdict(list)  # type: DefaultDict[int, List[str]]

    # Single table containing all the level2 hash tables.
    level2_hashtables = list()  # type: List[EncList]
    level2_doc = defaultdict(list)  # type: DefaultDict[int, List[str]]

    for cpumode in isa.cpumodes:
        level2_doc[len(level2_hashtables)].append(cpumode.name)
        level1 = make_tables(cpumode)
        level1_tables[cpumode] = level1
        encode_enclists(level1, seq_table, doc_table, isa)
        encode_level2_hashtables(level1, level2_hashtables, level2_doc)

    # Level 1 table encodes offsets into the level 2 table.
    level1_offt = offset_type(len(level2_hashtables))
    # Level 2 tables encodes offsets into seq_table.
    level2_offt = offset_type(len(seq_table.table))

    emit_enclists(seq_table, doc_table, fmt)
    emit_level2_hashtables(level2_hashtables, level2_offt, level2_doc, fmt)
    for cpumode in isa.cpumodes:
        emit_level1_hashtable(
                cpumode, level1_tables[cpumode], level1_offt, fmt)

    emit_recipe_names(isa, fmt)
    emit_recipe_constraints(isa, fmt)


def generate(isas, out_dir):
    # type: (Sequence[TargetISA], str) -> None
    for isa in isas:
        fmt = srcgen.Formatter()
        gen_isa(isa, fmt)
        fmt.update_file('encoding-{}.rs'.format(isa.name), out_dir)
