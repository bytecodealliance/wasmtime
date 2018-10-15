"""
Generate sources for instruction encoding.

The tables and functions generated here support the `TargetISA::encode()`
function which determines if a given instruction is legal, and if so, it's
`Encoding` data which consists of a *recipe* and some *encoding* bits.

The `encode` function doesn't actually generate the binary machine bits. Each
recipe has a corresponding hand-written function to do that after registers
are allocated.

This is the information available to us:

- The instruction to be encoded as an `InstructionData` reference.
- The controlling type variable.
- The data-flow graph giving us access to the types of all values involved.
  This is needed for testing any secondary type variables.
- A `PredicateView` reference for the ISA-specific settings for evaluating ISA
  predicates.
- The currently active CPU mode is determined by the ISA.

## Level 1 table lookup

The CPU mode provides the first table. The key is the instruction's controlling
type variable. If the instruction is not polymorphic, use `INVALID` for the
type variable. The table values are level 2 tables.

## Level 2 table lookup

The level 2 table is keyed by the instruction's opcode. The table values are
*encoding lists*.

The two-level table lookup allows the level 2 tables to be much smaller with
good locality. Code in any given function usually only uses a few different
types, so many of the level 2 tables will be cold.

## Encoding lists

An encoding list is a non-empty sequence of list entries. Each entry has
one of these forms:

1. Recipe + bits. Use this encoding if the recipe predicate is satisfied.
2. Recipe + bits, final entry. Use this encoding if the recipe predicate is
   satisfied. Otherwise, stop with the default legalization code.
3. Stop with legalization code.
4. Predicate + skip count. Test predicate and skip N entries if it is false.
4. Predicate + stop. Test predicate and stop with the default legalization code
   if it is false.

The instruction predicate is also used to distinguish between polymorphic
instructions with different types for secondary type variables.
"""
from __future__ import absolute_import
import srcgen
from constant_hash import compute_quadratic
from unique_table import UniqueSeqTable
from collections import OrderedDict, defaultdict
import math
from itertools import groupby
from cdsl.registers import RegClass, Register, Stack
from cdsl.predicates import FieldPredicate, TypePredicate
from cdsl.settings import SettingGroup
from cdsl.formats import instruction_context, InstructionFormat

try:
    from typing import Sequence, Set, Tuple, List, Dict, Iterable, DefaultDict, TYPE_CHECKING  # noqa
    if TYPE_CHECKING:
        from cdsl.isa import TargetISA, OperandConstraint, Encoding, CPUMode, EncRecipe, RecipePred  # noqa
        from cdsl.predicates import PredNode, PredLeaf  # noqa
        from cdsl.types import ValueType  # noqa
        from cdsl.instructions import Instruction  # noqa
        from cdsl.xform import XFormGroup  # noqa
except ImportError:
    pass


def emit_instp(instp, fmt, has_func=False):
    # type: (PredNode, srcgen.Formatter, bool) -> None
    """
    Emit code for matching an instruction predicate against an
    `InstructionData` reference called `inst`.

    The generated code is an `if let` pattern match that falls through if the
    instruction has an unexpected format. This should lead to a panic.
    """
    iform = instp.predicate_context()

    # Deal with pure type check predicates which apply to any instruction.
    if iform == instruction_context:
        fmt.line('let args = inst.arguments(&func.dfg.value_lists);')
        fmt.line(instp.rust_predicate(0))
        return

    assert isinstance(iform, InstructionFormat)

    # Which fields do we need in the InstructionData pattern match?
    has_type_check = False
    # Collect the leaf predicates.
    leafs = set()  # type: Set[PredLeaf]
    instp.predicate_leafs(leafs)
    # All the leafs are FieldPredicate or TypePredicate instances. Here we just
    # care about the field names.
    fnames = set()  # type: Set[str]
    for p in leafs:
        if isinstance(p, FieldPredicate):
            fnames.add(p.field.rust_destructuring_name())
        else:
            assert isinstance(p, TypePredicate)
            has_type_check = True
    fields = ', '.join(sorted(fnames))

    with fmt.indented(
            'if let ir::InstructionData::{} {{ {}, .. }} = *inst {{'
            .format(iform.name, fields), '}'):
        if has_type_check:
            # We could implement this if we need to.
            assert has_func, "Recipe predicates can't check type variables."
            fmt.line('let args = inst.arguments(&func.dfg.value_lists);')
        elif has_func:
            # Silence dead argument warning.
            fmt.line('let _ = func;')
        fmt.format('return {};', instp.rust_predicate(0))
    fmt.line('unreachable!();')


def emit_inst_predicates(instps, fmt):
    # type: (OrderedDict[PredNode, int], srcgen.Formatter) -> None
    """
    Emit private functions for matching instruction predicates as well as a
    static `INST_PREDICATES` array indexed by predicate number.
    """
    for instp, number in instps.items():
        name = 'inst_predicate_{}'.format(number)
        with fmt.indented(
                'fn {}(func: &ir::Function, inst: &ir::InstructionData)'
                '-> bool {{'.format(name), '}'):
            emit_instp(instp, fmt, has_func=True)

    # Generate the static table.
    with fmt.indented(
            'pub static INST_PREDICATES: [InstPredicate; {}] = ['
            .format(len(instps)), '];'):
        for instp, number in instps.items():
            fmt.format('inst_predicate_{},', number)


def emit_recipe_predicates(isa, fmt):
    # type: (TargetISA, srcgen.Formatter) -> None
    """
    Emit private functions for checking recipe predicates as well as a static
    `RECIPE_PREDICATES` array indexed by recipe number.

    A recipe predicate is a combination of an ISA predicate and an instruction
    predicates. Many recipes have identical predicates.
    """
    # Table for uniquing recipe predicates. Maps predicate to generated
    # function name.
    pname = dict()  # type: Dict[RecipePred, str]

    # Generate unique recipe predicates.
    for rcp in isa.all_recipes:
        p = rcp.recipe_pred()
        if p is None or p in pname:
            continue
        name = 'recipe_predicate_{}'.format(rcp.name.lower())
        pname[p] = name
        isap, instp = p

        # Generate the predicate function.
        with fmt.indented(
                'fn {}({}: ::settings::PredicateView, '
                '{}: &ir::InstructionData) -> bool {{'
                .format(
                    name,
                    'isap' if isap else '_',
                    'inst' if instp else '_'), '}'):
            if isap:
                n = isa.settings.predicate_number[isap]
                with fmt.indented('if !isap.test({}) {{'.format(n), '}'):
                    fmt.line('return false;')
            if instp:
                emit_instp(instp, fmt)
            else:
                fmt.line('true')

    # Generate the static table.
    with fmt.indented(
            'pub static RECIPE_PREDICATES: [RecipePredicate; {}] = ['
            .format(len(isa.all_recipes)), '];'):
        for rcp in isa.all_recipes:
            p = rcp.recipe_pred()
            if p is None:
                fmt.line('None,')
            else:
                fmt.format('Some({}),', pname[p])


# The u16 values in an encoding list entry are interpreted as follows:
#
# NR = len(all_recipes)
#
# entry < 2*NR
#     Try Encoding(entry/2, next_entry) if the recipe predicate is satisfied.
#     If bit 0 is set, stop with the default legalization code.
#     If bit 0 is clear, keep going down the list.
# entry < PRED_START
#     Stop with legalization code `entry - 2*NR`.
#
# Remaining entries are interpreted as (skip, pred) pairs, where:
#
#     skip = (entry - PRED_START) >> PRED_BITS
#     pred = (entry - PRED_START) & PRED_MASK
#
# If the predicate is satisfied, keep going. Otherwise skip over the next
# `skip` entries. If skip == 0, stop with the default legalization code.
#
# The `pred` predicate number is interpreted as an instruction predicate if it
# is in range, otherwise an ISA predicate.


class Encoder:
    """
    Encoder for the list format above.

    Two parameters are needed:

    :param NR: Number of recipes.
    :param NI: Number of instruction predicates.
    """

    def __init__(self, isa):
        # type: (TargetISA) -> None
        self.isa = isa
        self.NR = len(isa.all_recipes)
        self.NI = len(isa.instp_number)
        # u16 encoding list words.
        self.words = list()  # type: List[int]
        # Documentation comments: Index into `words` + comment.
        self.docs = list()  # type: List[Tuple[int, str]]

    # Encoding lists are represented as u16 arrays.
    CODE_BITS = 16

    # Beginning of the predicate code words.
    PRED_START = 0x1000

    # Number of bits used to hold a predicate number (instruction + ISA
    # predicates.
    PRED_BITS = 12

    # Mask for extracting the predicate number.
    PRED_MASK = (1 << PRED_BITS) - 1

    def max_skip(self):
        # type: () -> int
        """The maximum number of entries that a predicate can skip."""
        return (1 << (self.CODE_BITS - self.PRED_BITS)) - 1

    def recipe(self, enc, final):
        # type: (Encoding, bool) -> None
        """Add a recipe+bits entry to the list."""
        offset = len(self.words)
        code = 2 * enc.recipe.number
        doc = '--> {}'.format(enc)
        if final:
            code += 1
            doc += ' and stop'

        assert(code < self.PRED_START)
        self.words.extend((code, enc.encbits))
        self.docs.append((offset, doc))

    def _pred(self, pred, skip, n):
        # type: (PredNode, int, int) -> None
        """Add a predicate entry."""
        assert n <= self.PRED_MASK
        code = n | (skip << self.PRED_BITS)
        code += self.PRED_START
        assert code < (1 << self.CODE_BITS)

        if skip == 0:
            doc = 'stop'
        else:
            doc = 'skip ' + str(skip)
        doc = '{} unless {}'.format(doc, pred)

        self.docs.append((len(self.words), doc))
        self.words.append(code)

    def instp(self, pred, skip):
        # type: (PredNode, int) -> None
        """Add an instruction predicate entry."""
        number = self.isa.instp_number[pred]
        self._pred(pred, skip, number)

    def isap(self, pred, skip):
        # type: (PredNode, int) -> None
        """Add an ISA predicate entry."""
        n = self.isa.settings.predicate_number[pred]
        # ISA predicates follow the instruction predicates.
        self._pred(pred, skip, self.NI + n)


class EncNode(object):
    """
    An abstract node in the encoder tree for an instruction.

    This tree is used to simplify the predicates guarding recipe+bits entries.
    """

    def size(self):
        # type: () -> int
        """Get the number of list entries needed to encode this tree."""
        raise NotImplementedError('EncNode.size() is abstract')

    def encode(self, encoder, final):
        # type: (Encoder, bool) -> None
        """Encode this tree."""
        raise NotImplementedError('EncNode.encode() is abstract')

    def optimize(self):
        # type: () -> EncNode
        """Transform this encoder tree into something simpler."""
        return self

    def predicate(self):
        # type: () -> PredNode
        """Get the predicate guarding this tree, or `None` for always"""
        return None


class EncPred(EncNode):
    """
    An encoder tree node which asserts a predicate on its child nodes.

    A `None` predicate is always satisfied.
    """

    def __init__(self, pred, children):
        # type: (PredNode, List[EncNode]) -> None
        self.pred = pred
        self.children = children

    def size(self):
        # type: () -> int
        s = 1 if self.pred else 0
        s += sum(c.size() for c in self.children)
        return s

    def encode(self, encoder, final):
        # type: (Encoder, bool) -> None
        if self.pred:
            skip = 0 if final else self.size() - 1
            ctx = self.pred.predicate_context()
            if isinstance(ctx, SettingGroup):
                encoder.isap(self.pred, skip)
            else:
                encoder.instp(self.pred, skip)

        final_idx = len(self.children) - 1 if final else -1
        for idx, node in enumerate(self.children):
            node.encode(encoder, idx == final_idx)

    def predicate(self):
        # type: () -> PredNode
        return self.pred

    def optimize(self):
        # type: () -> EncNode
        """
        Optimize a predicate node in the tree by combining child nodes that
        have identical predicates.
        """
        cnodes = list()  # type: List[EncNode]
        for pred, niter in groupby(
                map(lambda c: c.optimize(), self.children),
                key=lambda c: c.predicate()):
            nodes = list(niter)
            if pred is None or len(nodes) <= 1:
                cnodes.extend(nodes)
                continue

            # We have multiple children with identical predicates.
            # Group them all into `n0`.
            n0 = nodes[0]
            assert isinstance(n0, EncPred)
            for n in nodes[1:]:
                assert isinstance(n, EncPred)
                n0.children.extend(n.children)

            cnodes.append(n0)

        # Finally strip a redundant grouping node.
        if self.pred is None and len(cnodes) == 1:
            return cnodes[0]
        else:
            self.children = cnodes
            return self


class EncLeaf(EncNode):
    """
    A leaf in the encoder tree.

    This represents a single `Encoding`, without its predicates (they are
    represented in the tree by parent nodes.
    """

    def __init__(self, encoding):
        # type: (Encoding) -> None
        self.encoding = encoding

    def size(self):
        # type: () -> int
        # recipe + bits.
        return 2

    def encode(self, encoder, final):
        # type: (Encoder, bool) -> None
        encoder.recipe(self.encoding, final)


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

    def encoder_tree(self):
        # type: () -> EncNode
        """
        Generate an optimized encoder tree for this list. The tree represents
        all of the encodings with parent nodes for the predicates that need
        checking.
        """
        forest = list()  # type: List[EncNode]
        for enc in self.encodings:
            n = EncLeaf(enc)  # type: EncNode
            if enc.instp:
                n = EncPred(enc.instp, [n])
            if enc.isap:
                n = EncPred(enc.isap, [n])
            forest.append(n)

        return EncPred(None, forest).optimize()

    def encode(self, seq_table, doc_table, isa):
        # type: (UniqueSeqTable, DefaultDict[int, List[str]], TargetISA) -> None  # noqa
        """
        Encode this list as a sequence of u16 numbers.

        Adds the sequence to `seq_table` and records the returned offset as
        `self.offset`.

        Adds comment lines to `doc_table` keyed by seq_table offsets.
        """
        # Use an encoder object to hold the parameters.
        encoder = Encoder(isa)
        tree = self.encoder_tree()
        tree.encode(encoder, True)

        self.offset = seq_table.add(encoder.words)

        # Add doc comments.
        doc_table[self.offset].append(
                '{:06x}: {}'.format(self.offset, self.name()))
        for pos, doc in encoder.docs:
            doc_table[self.offset + pos].append(doc)
        doc_table[self.offset + len(encoder.words)].insert(
                0, 'end of: {}'.format(self.name()))


class Level2Table(object):
    """
    Level 2 table mapping instruction opcodes to `EncList` objects.

    A level 2 table can be completely empty if it only holds a custom
    legalization action for `ty`.

    :param ty: Controlling type variable of all entries, or `None`.
    :param legalize: Default legalize action for `ty`.
    """

    def __init__(self, ty, legalize):
        # type: (ValueType, XFormGroup) -> None
        self.ty = ty
        self.legalize = legalize
        # Maps inst -> EncList
        self.lists = OrderedDict()  # type: OrderedDict[Instruction, EncList]

    def __getitem__(self, inst):
        # type: (Instruction) -> EncList
        ls = self.lists.get(inst)
        if not ls:
            ls = EncList(inst, self.ty)
            self.lists[inst] = ls
        return ls

    def is_empty(self):
        # type: () -> bool
        """
        Check if this level 2 table is completely empty.

        This can happen if the associated type simply has an overridden
        legalize action.
        """
        return len(self.lists) == 0

    def enclists(self):
        # type: () -> Iterable[EncList]
        return iter(self.lists.values())

    def layout_hashtable(self, level2_hashtables, level2_doc):
        # type: (List[EncList], DefaultDict[int, List[str]]) -> None
        """
        Compute the hash table mapping opcode -> enclist.

        Append the hash table to `level2_hashtables` and record the offset.
        """
        def hash_func(enclist):
            # type: (EncList) -> int
            return enclist.inst.number
        hash_table = compute_quadratic(self.lists.values(), hash_func)

        self.hash_table_offset = len(level2_hashtables)
        self.hash_table_len = len(hash_table)

        level2_doc[self.hash_table_offset].append(
                '{:06x}: {}, {} entries'.format(
                    self.hash_table_offset,
                    self.ty,
                    self.hash_table_len))
        level2_hashtables.extend(hash_table)


class Level1Table(object):
    """
    Level 1 table mapping types to `Level2` objects.
    """

    def __init__(self, cpumode):
        # type: (CPUMode) -> None
        self.cpumode = cpumode
        self.tables = OrderedDict()  # type: OrderedDict[ValueType, Level2Table]  # noqa

        if cpumode.default_legalize is None:
            raise AssertionError(
                    'CPU mode {}.{} needs a default legalize action'
                    .format(cpumode.isa, cpumode))
        self.legalize_code = cpumode.isa.legalize_code(
                cpumode.default_legalize)

    def __getitem__(self, ty):
        # type: (ValueType) -> Level2Table
        tbl = self.tables.get(ty)
        if not tbl:
            legalize = self.cpumode.get_legalize_action(ty)
            # Allocate a legalization code in a predictable order.
            self.cpumode.isa.legalize_code(legalize)
            tbl = Level2Table(ty, legalize)
            self.tables[ty] = tbl
        return tbl

    def l2tables(self):
        # type: () -> Iterable[Level2Table]
        return (l2 for l2 in self.tables.values() if not l2.is_empty())


def make_tables(cpumode):
    # type: (CPUMode) -> Level1Table
    """
    Generate tables for `cpumode` as described above.
    """
    table = Level1Table(cpumode)
    for enc in cpumode.encodings:
        ty = enc.ctrl_typevar()
        inst = enc.inst
        table[ty][inst].encodings.append(enc)

    # Ensure there are level 1 table entries for all types with a custom
    # legalize action.
    for ty in cpumode.type_legalize.keys():
        table[ty]

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
                        '{{ opcode: Some(ir::Opcode::{}), offset: {:#08x} }},'
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
    def hash_func(level2):
        # type: (Level2Table) -> int
        return level2.ty.number if level2.ty is not None else 0
    hash_table = compute_quadratic(level1.tables.values(), hash_func)

    with fmt.indented(
            'pub static LEVEL1_{}: [Level1Entry<{}>; {}] = ['
            .format(cpumode.name.upper(), offt, len(hash_table)), '];'):
        for level2 in hash_table:
            # Empty hash table entry. Include the default legalization action.
            if not level2:
                fmt.format(
                        'Level1Entry {{ ty: ir::types::INVALID, log2len: !0, '
                        'offset: 0, legalize: {} }},',
                        level1.legalize_code)
                continue

            if level2.ty is not None:
                tyname = level2.ty.rust_name()
            else:
                tyname = 'ir::types::INVALID'

            lcode = cpumode.isa.legalize_code(level2.legalize)

            # Empty level 2 table: Only a specialized legalization action, no
            # actual table.
            # Set an offset that is out of bounds, but make sure it doesn't
            # overflow its type when adding `1<<log2len`.
            if level2.is_empty():
                fmt.format(
                        'Level1Entry {{ '
                        'ty: {}, log2len: 0, offset: !0 - 1, '
                        'legalize: {} }}, // {}',
                        tyname, lcode, level2.legalize)
                continue

            # Proper level 2 hash table.
            l2l = int(math.log(level2.hash_table_len, 2))
            assert l2l > 0, "Level2 hash table too small"
            fmt.format(
                    'Level1Entry {{ '
                    'ty: {}, log2len: {}, offset: {:#08x}, '
                    'legalize: {} }}, // {}',
                    tyname, l2l, level2.hash_table_offset,
                    lcode, level2.legalize)


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
            'static RECIPE_NAMES: [&str; {}] = ['
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
            'static RECIPE_CONSTRAINTS: [RecipeConstraints; {}] = ['
            .format(len(isa.all_recipes)), '];'):
        for r in isa.all_recipes:
            fmt.comment('Constraints for recipe {}:'.format(r.name))
            tied_i2o, tied_o2i = r.ties()
            fixed_ins, fixed_outs = r.fixed_ops()
            with fmt.indented('RecipeConstraints {', '},'):
                emit_operand_constraints(
                    r, r.ins, 'ins', tied_i2o, fixed_outs, fmt)
                emit_operand_constraints(
                    r, r.outs, 'outs', tied_o2i, fixed_ins, fmt)
                fmt.format('fixed_ins: {},', str(bool(fixed_ins)).lower())
                fmt.format('fixed_outs: {},', str(bool(fixed_outs)).lower())
                fmt.format('tied_ops: {},', str(bool(tied_i2o)).lower())
                fmt.format(
                        'clobbers_flags: {},',
                        str(bool(r.clobbers_flags)).lower())


def emit_operand_constraints(
        recipe,  # type: EncRecipe
        seq,     # type: Sequence[OperandConstraint]
        field,   # type: str
        tied,    # type: Dict[int, int]
        fixops,  # type: Set[Register]
        fmt      # type: srcgen.Formatter
        ):
    # type: (...) -> None
    """
    Emit a struct field initializer for an array of operand constraints.

    :param field: The name of the struct field to emit.
    :param tied: Map of tied opnums to counterparts.
    :param fix_ops: Set of fixed operands on the other side of the inst.
    """
    if len(seq) == 0:
        fmt.line('{}: &[],'.format(field))
        return
    with fmt.indented('{}: &['.format(field), '],'):
        for n, cons in enumerate(seq):
            with fmt.indented('OperandConstraint {', '},'):
                if isinstance(cons, RegClass):
                    if n in tied:
                        fmt.format('kind: ConstraintKind::Tied({}),', tied[n])
                    else:
                        fmt.line('kind: ConstraintKind::Reg,')
                    fmt.format('regclass: &{}_DATA,', cons)
                elif isinstance(cons, Register):
                    assert n not in tied, "Can't tie fixed register operand"
                    # See if this fixed register is also on the other side.
                    t = 'FixedTied' if cons in fixops else 'FixedReg'
                    fmt.format('kind: ConstraintKind::{}({}),', t, cons.unit)
                    fmt.format('regclass: &{}_DATA,', cons.regclass)
                elif isinstance(cons, int):
                    # This is a tied output constraint. It should never happen
                    # for input constraints.
                    assert cons == tied[n], "Invalid tied constraint"
                    fmt.format('kind: ConstraintKind::Tied({}),', cons)
                    fmt.format('regclass: &{}_DATA,', recipe.ins[cons])
                elif isinstance(cons, Stack):
                    assert n not in tied, "Can't tie stack operand"
                    fmt.line('kind: ConstraintKind::Stack,')
                    fmt.format('regclass: &{}_DATA,', cons.regclass)
                else:
                    raise AssertionError(
                            'Unsupported constraint {}'.format(cons))


def emit_recipe_sizing(isa, fmt):
    # type: (TargetISA, srcgen.Formatter) -> None
    """
    Emit a table of encoding recipe code size information.
    """
    with fmt.indented(
            'static RECIPE_SIZING: [RecipeSizing; {}] = ['
            .format(len(isa.all_recipes)), '];'):
        for r in isa.all_recipes:
            fmt.comment('Code size information for recipe {}:'.format(r.name))
            with fmt.indented('RecipeSizing {', '},'):
                fmt.format('base_size: {},', r.base_size)
                fmt.format('compute_size: {},', r.compute_size)
                if r.branch_range:
                    fmt.format(
                        'branch_range: '
                        'Some(BranchRange {{ origin: {}, bits: {} }}),',
                        *r.branch_range)
                else:
                    fmt.line('branch_range: None,')


def gen_isa(isa, fmt):
    # type: (TargetISA, srcgen.Formatter) -> None

    # Make the `RECIPE_PREDICATES` table.
    emit_recipe_predicates(isa, fmt)

    # Make the `INST_PREDICATES` table.
    emit_inst_predicates(isa.instp_number, fmt)

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
    emit_recipe_sizing(isa, fmt)

    # Finally, tie it all together in an `EncInfo`.
    with fmt.indented('pub static INFO: isa::EncInfo = isa::EncInfo {', '};'):
        fmt.line('constraints: &RECIPE_CONSTRAINTS,')
        fmt.line('sizing: &RECIPE_SIZING,')
        fmt.line('names: &RECIPE_NAMES,')


def generate(isas, out_dir):
    # type: (Sequence[TargetISA], str) -> None
    for isa in isas:
        fmt = srcgen.Formatter()
        gen_isa(isa, fmt)
        fmt.update_file('encoding-{}.rs'.format(isa.name), out_dir)
