"""
Intel Encoding recipes.
"""
from __future__ import absolute_import
from cdsl.isa import EncRecipe
from cdsl.predicates import IsSignedInt, IsEqual, Or
from cdsl.registers import RegClass
from base.formats import Unary, UnaryImm, Binary, BinaryImm, MultiAry
from base.formats import Trap, Call, IndirectCall, Store, Load
from base.formats import IntCompare, FloatCompare
from base.formats import RegMove, Ternary, Jump, Branch, FuncAddr
from .registers import GPR, ABCD, FPR, GPR8, FPR8, StackGPR32, StackFPR32
from .defs import supported_floatccs

try:
    from typing import Tuple, Dict, Sequence, Any  # noqa
    from cdsl.instructions import InstructionFormat  # noqa
    from cdsl.isa import ConstraintSeq, BranchRange, PredNode, OperandConstraint  # noqa
except ImportError:
    pass


# Opcode representation.
#
# Cretonne requires each recipe to have a single encoding size in bytes, and
# Intel opcodes are variable length, so we use separate recipes for different
# styles of opcodes and prefixes. The opcode format is indicated by the recipe
# name prefix:

OPCODE_PREFIX = {
        # Prefix bytes       Name     mmpp
        ():                 ('Op1', 0b0000),
        (0x66,):            ('Mp1', 0b0001),
        (0xf3,):            ('Mp1', 0b0010),
        (0xf2,):            ('Mp1', 0b0011),
        (0x0f,):            ('Op2', 0b0100),
        (0x66, 0x0f):       ('Mp2', 0b0101),
        (0xf3, 0x0f):       ('Mp2', 0b0110),
        (0xf2, 0x0f):       ('Mp2', 0b0111),
        (0x0f, 0x38):       ('Op3', 0b1000),
        (0x66, 0x0f, 0x38): ('Mp3', 0b1001),
        (0xf3, 0x0f, 0x38): ('Mp3', 0b1010),
        (0xf2, 0x0f, 0x38): ('Mp3', 0b1011),
        (0x0f, 0x3a):       ('Op3', 0b1100),
        (0x66, 0x0f, 0x3a): ('Mp3', 0b1101),
        (0xf3, 0x0f, 0x3a): ('Mp3', 0b1110),
        (0xf2, 0x0f, 0x3a): ('Mp3', 0b1111)
        }

# The table above does not include the REX prefix which goes after the
# mandatory prefix. VEX/XOP and EVEX prefixes are not yet supported. Encodings
# using any of these prefixes are represented by separate recipes.
#
# The encoding bits are:
#
# 0-7:   The opcode byte <op>.
# 8-9:   pp, mandatory prefix:
#        00 none (Op*)
#        01 66   (Mp*)
#        10 F3   (Mp*)
#        11 F2   (Mp*)
# 10-11: mm, opcode map:
#        00 <op>        (Op1/Mp1)
#        01 0F <op>     (Op2/Mp2)
#        10 0F 38 <op>  (Op3/Mp3)
#        11 0F 3A <op>  (Op3/Mp3)
# 12-14  rrr, opcode bits for the ModR/M byte for certain opcodes.
# 15:    REX.W bit (or VEX.W/E)
#
# There is some redundancy between bits 8-11 and the recipe names, but we have
# enough bits, and the pp+mm format is ready for supporting VEX prefixes.


def decode_ops(ops, rrr=0, w=0):
    # type: (Tuple[int, ...], int, int) -> Tuple[str, int]
    """
    Given a sequence of opcode bytes, compute the recipe name prefix and
    encoding bits.
    """
    assert rrr <= 0b111
    assert w <= 1
    name, mmpp = OPCODE_PREFIX[ops[:-1]]
    op = ops[-1]
    assert op <= 256
    return (name, op | (mmpp << 8) | (rrr << 12) | (w << 15))


def replace_put_op(emit, prefix):
    # type: (str, str) -> str
    """
    Given a snippet of Rust code (or None), replace the `PUT_OP` macro with the
    corresponding `put_*` function from the `binemit.rs` module.
    """
    if emit is None:
        return None
    else:
        return emit.replace('PUT_OP', 'put_' + prefix.lower())


# Register class mapping for no-REX instructions.
NOREX_MAP = {
        GPR: GPR8,
        FPR: FPR8
    }

# Register class mapping for REX instructions. The ABCD constraint no longer
# applies.
REX_MAP = {
        ABCD: GPR
    }


def map_regs(
        regs,  # type: Sequence[OperandConstraint]
        mapping  # type: Dict[RegClass, RegClass]
):
    # type: (...) -> Sequence[OperandConstraint]
    return tuple(mapping.get(rc, rc) if isinstance(rc, RegClass) else rc
                 for rc in regs)


class TailRecipe:
    """
    Generate encoding recipes on demand.

    Intel encodings are somewhat orthogonal with the opcode representation on
    one side and the ModR/M, SIB and immediate fields on the other side.

    A `TailRecipe` represents the part of an encoding that follow the opcode.
    It is used to generate full encoding recipes on demand when combined with
    an opcode.

    The arguments are the same as for an `EncRecipe`, except for `size` which
    does not include the size of the opcode.

    The `emit` parameter contains Rust code to actually emit an encoding, like
    `EncRecipe` does it. Additionally, the text `PUT_OP` is substituted with
    the proper `put_*` function from the `intel/binemit.rs` module.
    """

    def __init__(
            self,
            name,               # type: str
            format,             # type: InstructionFormat
            size,               # type: int
            ins,                # type: ConstraintSeq
            outs,               # type: ConstraintSeq
            branch_range=None,  # type: int
            instp=None,         # type: PredNode
            isap=None,          # type: PredNode
            emit=None           # type: str
            ):
        # type: (...) -> None
        self.name = name
        self.format = format
        self.size = size
        self.ins = ins
        self.outs = outs
        self.branch_range = branch_range
        self.instp = instp
        self.isap = isap
        self.emit = emit

        # Cached recipes, keyed by name prefix.
        self.recipes = dict()  # type: Dict[str, EncRecipe]

    def __call__(self, *ops, **kwargs):
        # type: (*int, **int) -> Tuple[EncRecipe, int]
        """
        Create an encoding recipe and encoding bits for the opcode bytes in
        `ops`.
        """
        rrr = kwargs.get('rrr', 0)
        w = kwargs.get('w', 0)
        name, bits = decode_ops(ops, rrr, w)
        size = len(ops) + self.size

        # All branch ranges are relative to the end of the instruction.
        branch_range = None  # type BranchRange
        if self.branch_range is not None:
            branch_range = (size, self.branch_range)

        if name not in self.recipes:
            recipe = EncRecipe(
                name + self.name,
                self.format,
                size,
                ins=self.ins,
                outs=self.outs,
                branch_range=branch_range,
                instp=self.instp,
                isap=self.isap,
                emit=replace_put_op(self.emit, name))

            recipe.ins = map_regs(recipe.ins, NOREX_MAP)
            recipe.outs = map_regs(recipe.outs, NOREX_MAP)
            self.recipes[name] = recipe
        return (self.recipes[name], bits)

    def rex(self, *ops, **kwargs):
        # type: (*int, **int) -> Tuple[EncRecipe, int]
        """
        Create a REX encoding recipe and encoding bits for the opcode bytes in
        `ops`.

        The recipe will always generate a REX prefix, whether it is required or
        not. For instructions that don't require a REX prefix, two encodings
        should be added: One with REX and one without.
        """
        rrr = kwargs.get('rrr', 0)
        w = kwargs.get('w', 0)
        name, bits = decode_ops(ops, rrr, w)
        name = 'Rex' + name
        size = 1 + len(ops) + self.size

        # All branch ranges are relative to the end of the instruction.
        branch_range = None  # type BranchRange
        if self.branch_range is not None:
            branch_range = (size, self.branch_range)

        if name not in self.recipes:
            recipe = EncRecipe(
                name + self.name,
                self.format,
                size,
                ins=self.ins,
                outs=self.outs,
                branch_range=branch_range,
                instp=self.instp,
                isap=self.isap,
                emit=replace_put_op(self.emit, name))
            recipe.ins = map_regs(recipe.ins, REX_MAP)
            recipe.outs = map_regs(recipe.outs, REX_MAP)
            self.recipes[name] = recipe

        return (self.recipes[name], bits)

    @staticmethod
    def check_names(globs):
        # type: (Dict[str, Any]) -> None
        for name, obj in globs.items():
            if isinstance(obj, TailRecipe):
                assert name == obj.name, "Mismatched TailRecipe name: " + name


# A null unary instruction that takes a GPR register. Can be used for identity
# copies and no-op conversions.
null = EncRecipe('null', Unary, size=0, ins=GPR, outs=0, emit='')

# XX opcode, no ModR/M.
trap = TailRecipe(
        'trap', Trap, size=0, ins=(), outs=(),
        emit='PUT_OP(bits, BASE_REX, sink);')

# XX /r
rr = TailRecipe(
        'rr', Binary, size=1, ins=(GPR, GPR), outs=0,
        emit='''
        PUT_OP(bits, rex2(in_reg0, in_reg1), sink);
        modrm_rr(in_reg0, in_reg1, sink);
        ''')

# XX /r with operands swapped. (RM form).
rrx = TailRecipe(
        'rrx', Binary, size=1, ins=(GPR, GPR), outs=0,
        emit='''
        PUT_OP(bits, rex2(in_reg1, in_reg0), sink);
        modrm_rr(in_reg1, in_reg0, sink);
        ''')

# XX /r with FPR ins and outs. A form.
fa = TailRecipe(
        'fa', Binary, size=1, ins=(FPR, FPR), outs=0,
        emit='''
        PUT_OP(bits, rex2(in_reg1, in_reg0), sink);
        modrm_rr(in_reg1, in_reg0, sink);
        ''')

# XX /r with FPR ins and outs. A form with input operands swapped.
fax = TailRecipe(
        'fax', Binary, size=1, ins=(FPR, FPR), outs=1,
        emit='''
        PUT_OP(bits, rex2(in_reg0, in_reg1), sink);
        modrm_rr(in_reg0, in_reg1, sink);
        ''')

# XX /r, but for a unary operator with separate input/output register, like
# copies. MR form.
umr = TailRecipe(
        'umr', Unary, size=1, ins=GPR, outs=GPR,
        emit='''
        PUT_OP(bits, rex2(out_reg0, in_reg0), sink);
        modrm_rr(out_reg0, in_reg0, sink);
        ''')

# Same as umr, but with FPR -> GPR registers.
rfumr = TailRecipe(
        'rfumr', Unary, size=1, ins=FPR, outs=GPR,
        emit='''
        PUT_OP(bits, rex2(out_reg0, in_reg0), sink);
        modrm_rr(out_reg0, in_reg0, sink);
        ''')

# XX /r, but for a unary operator with separate input/output register.
# RM form.
urm = TailRecipe(
        'urm', Unary, size=1, ins=GPR, outs=GPR,
        emit='''
        PUT_OP(bits, rex2(in_reg0, out_reg0), sink);
        modrm_rr(in_reg0, out_reg0, sink);
        ''')

# XX /r. Same as urm, but input limited to ABCD.
urm_abcd = TailRecipe(
        'urm_abcd', Unary, size=1, ins=ABCD, outs=GPR,
        emit='''
        PUT_OP(bits, rex2(in_reg0, out_reg0), sink);
        modrm_rr(in_reg0, out_reg0, sink);
        ''')

# XX /r, RM form, FPR -> FPR.
furm = TailRecipe(
        'furm', Unary, size=1, ins=FPR, outs=FPR,
        emit='''
        PUT_OP(bits, rex2(in_reg0, out_reg0), sink);
        modrm_rr(in_reg0, out_reg0, sink);
        ''')

# XX /r, RM form, GPR -> FPR.
frurm = TailRecipe(
        'frurm', Unary, size=1, ins=GPR, outs=FPR,
        emit='''
        PUT_OP(bits, rex2(in_reg0, out_reg0), sink);
        modrm_rr(in_reg0, out_reg0, sink);
        ''')

# XX /r, RM form, FPR -> GPR.
rfurm = TailRecipe(
        'rfurm', Unary, size=1, ins=FPR, outs=GPR,
        emit='''
        PUT_OP(bits, rex2(in_reg0, out_reg0), sink);
        modrm_rr(in_reg0, out_reg0, sink);
        ''')

# XX /r, RMI form for one of the roundXX SSE 4.1 instructions.
furmi_rnd = TailRecipe(
        'furmi_rnd', Unary, size=2, ins=FPR, outs=FPR,
        emit='''
        PUT_OP(bits, rex2(in_reg0, out_reg0), sink);
        modrm_rr(in_reg0, out_reg0, sink);
        sink.put1(match opcode {
            Opcode::Nearest => 0b00,
            Opcode::Floor => 0b01,
            Opcode::Ceil => 0b10,
            Opcode::Trunc => 0b11,
            x => panic!("{} unexpected for furmi_rnd", opcode),
        });
        ''')

# XX /r, for regmove instructions.
rmov = TailRecipe(
        'rmov', RegMove, size=1, ins=GPR, outs=(),
        emit='''
        PUT_OP(bits, rex2(dst, src), sink);
        modrm_rr(dst, src, sink);
        ''')

# XX /r, for regmove instructions (FPR version, RM encoded).
frmov = TailRecipe(
        'frmov', RegMove, size=1, ins=FPR, outs=(),
        emit='''
        PUT_OP(bits, rex2(src, dst), sink);
        modrm_rr(src, dst, sink);
        ''')

# XX /n with one arg in %rcx, for shifts.
rc = TailRecipe(
        'rc', Binary, size=1, ins=(GPR, GPR.rcx), outs=0,
        emit='''
        PUT_OP(bits, rex1(in_reg0), sink);
        modrm_r_bits(in_reg0, bits, sink);
        ''')

# XX /n for division: inputs in %rax, %rdx, r. Outputs in %rax, %rdx.
div = TailRecipe(
        'div', Ternary, size=1,
        ins=(GPR.rax, GPR.rdx, GPR), outs=(GPR.rax, GPR.rdx),
        emit='''
        PUT_OP(bits, rex1(in_reg2), sink);
        modrm_r_bits(in_reg2, bits, sink);
        ''')

# XX /n ib with 8-bit immediate sign-extended.
rib = TailRecipe(
        'rib', BinaryImm, size=2, ins=GPR, outs=0,
        instp=IsSignedInt(BinaryImm.imm, 8),
        emit='''
        PUT_OP(bits, rex1(in_reg0), sink);
        modrm_r_bits(in_reg0, bits, sink);
        let imm: i64 = imm.into();
        sink.put1(imm as u8);
        ''')

# XX /n id with 32-bit immediate sign-extended.
rid = TailRecipe(
        'rid', BinaryImm, size=5, ins=GPR, outs=0,
        instp=IsSignedInt(BinaryImm.imm, 32),
        emit='''
        PUT_OP(bits, rex1(in_reg0), sink);
        modrm_r_bits(in_reg0, bits, sink);
        let imm: i64 = imm.into();
        sink.put4(imm as u32);
        ''')

# XX /n id with 32-bit immediate sign-extended. UnaryImm version.
uid = TailRecipe(
        'uid', UnaryImm, size=5, ins=(), outs=GPR,
        instp=IsSignedInt(UnaryImm.imm, 32),
        emit='''
        PUT_OP(bits, rex1(out_reg0), sink);
        modrm_r_bits(out_reg0, bits, sink);
        let imm: i64 = imm.into();
        sink.put4(imm as u32);
        ''')

# XX+rd id unary with 32-bit immediate. Note no recipe predicate.
puid = TailRecipe(
        'puid', UnaryImm, size=4, ins=(), outs=GPR,
        emit='''
        // The destination register is encoded in the low bits of the opcode.
        // No ModR/M.
        PUT_OP(bits | (out_reg0 & 7), rex1(out_reg0), sink);
        let imm: i64 = imm.into();
        sink.put4(imm as u32);
        ''')

# XX+rd iq unary with 64-bit immediate.
puiq = TailRecipe(
        'puiq', UnaryImm, size=8, ins=(), outs=GPR,
        emit='''
        PUT_OP(bits | (out_reg0 & 7), rex1(out_reg0), sink);
        let imm: i64 = imm.into();
        sink.put8(imm as u64);
        ''')

# XX+rd id with Abs4 function relocation.
fnaddr4 = TailRecipe(
        'fnaddr4', FuncAddr, size=4, ins=(), outs=GPR,
        emit='''
        PUT_OP(bits | (out_reg0 & 7), rex1(out_reg0), sink);
        sink.reloc_func(RelocKind::Abs4.into(), func_ref);
        // Write the immediate as `!0` for the benefit of BaldrMonkey.
        sink.put4(!0);
        ''')

# XX+rd iq with Abs8 function relocation.
fnaddr8 = TailRecipe(
        'fnaddr8', FuncAddr, size=8, ins=(), outs=GPR,
        emit='''
        PUT_OP(bits | (out_reg0 & 7), rex1(out_reg0), sink);
        sink.reloc_func(RelocKind::Abs8.into(), func_ref);
        // Write the immediate as `!0` for the benefit of BaldrMonkey.
        sink.put8(!0);
        ''')

#
# Store recipes.
#

# XX /r register-indirect store with no offset.
st = TailRecipe(
        'st', Store, size=1, ins=(GPR, GPR), outs=(),
        instp=IsEqual(Store.offset, 0),
        emit='''
        PUT_OP(bits, rex2(in_reg1, in_reg0), sink);
        modrm_rm(in_reg1, in_reg0, sink);
        ''')

# XX /r register-indirect store with no offset.
# Only ABCD allowed for stored value. This is for byte stores.
st_abcd = TailRecipe(
        'st_abcd', Store, size=1, ins=(ABCD, GPR), outs=(),
        instp=IsEqual(Store.offset, 0),
        emit='''
        PUT_OP(bits, rex2(in_reg1, in_reg0), sink);
        modrm_rm(in_reg1, in_reg0, sink);
        ''')

# XX /r register-indirect store of FPR with no offset.
fst = TailRecipe(
        'fst', Store, size=1, ins=(FPR, GPR), outs=(),
        instp=IsEqual(Store.offset, 0),
        emit='''
        PUT_OP(bits, rex2(in_reg1, in_reg0), sink);
        modrm_rm(in_reg1, in_reg0, sink);
        ''')

# XX /r register-indirect store with 8-bit offset.
stDisp8 = TailRecipe(
        'stDisp8', Store, size=2, ins=(GPR, GPR), outs=(),
        instp=IsSignedInt(Store.offset, 8),
        emit='''
        PUT_OP(bits, rex2(in_reg1, in_reg0), sink);
        modrm_disp8(in_reg1, in_reg0, sink);
        let offset: i32 = offset.into();
        sink.put1(offset as u8);
        ''')
stDisp8_abcd = TailRecipe(
        'stDisp8_abcd', Store, size=2, ins=(ABCD, GPR), outs=(),
        instp=IsSignedInt(Store.offset, 8),
        emit='''
        PUT_OP(bits, rex2(in_reg1, in_reg0), sink);
        modrm_disp8(in_reg1, in_reg0, sink);
        let offset: i32 = offset.into();
        sink.put1(offset as u8);
        ''')
fstDisp8 = TailRecipe(
        'fstDisp8', Store, size=2, ins=(FPR, GPR), outs=(),
        instp=IsSignedInt(Store.offset, 8),
        emit='''
        PUT_OP(bits, rex2(in_reg1, in_reg0), sink);
        modrm_disp8(in_reg1, in_reg0, sink);
        let offset: i32 = offset.into();
        sink.put1(offset as u8);
        ''')

# XX /r register-indirect store with 32-bit offset.
stDisp32 = TailRecipe(
        'stDisp32', Store, size=5, ins=(GPR, GPR), outs=(),
        emit='''
        PUT_OP(bits, rex2(in_reg1, in_reg0), sink);
        modrm_disp32(in_reg1, in_reg0, sink);
        let offset: i32 = offset.into();
        sink.put4(offset as u32);
        ''')
stDisp32_abcd = TailRecipe(
        'stDisp32_abcd', Store, size=5, ins=(ABCD, GPR), outs=(),
        emit='''
        PUT_OP(bits, rex2(in_reg1, in_reg0), sink);
        modrm_disp32(in_reg1, in_reg0, sink);
        let offset: i32 = offset.into();
        sink.put4(offset as u32);
        ''')
fstDisp32 = TailRecipe(
        'fstDisp32', Store, size=5, ins=(FPR, GPR), outs=(),
        emit='''
        PUT_OP(bits, rex2(in_reg1, in_reg0), sink);
        modrm_disp32(in_reg1, in_reg0, sink);
        let offset: i32 = offset.into();
        sink.put4(offset as u32);
        ''')

# Unary spill with SIB and 32-bit displacement.
spSib32 = TailRecipe(
        'spSib32', Unary, size=6, ins=GPR, outs=StackGPR32,
        emit='''
        let base = stk_base(out_stk0.base);
        PUT_OP(bits, rex2(base, in_reg0), sink);
        modrm_sib_disp32(in_reg0, sink);
        sib_noindex(base, sink);
        sink.put4(out_stk0.offset as u32);
        ''')
fspSib32 = TailRecipe(
        'fspSib32', Unary, size=6, ins=FPR, outs=StackFPR32,
        emit='''
        let base = stk_base(out_stk0.base);
        PUT_OP(bits, rex2(base, in_reg0), sink);
        modrm_sib_disp32(in_reg0, sink);
        sib_noindex(base, sink);
        sink.put4(out_stk0.offset as u32);
        ''')

#
# Load recipes
#

# XX /r load with no offset.
ld = TailRecipe(
        'ld', Load, size=1, ins=(GPR), outs=(GPR),
        instp=IsEqual(Load.offset, 0),
        emit='''
        PUT_OP(bits, rex2(in_reg0, out_reg0), sink);
        modrm_rm(in_reg0, out_reg0, sink);
        ''')

# XX /r float load with no offset.
fld = TailRecipe(
        'fld', Load, size=1, ins=(GPR), outs=(FPR),
        instp=IsEqual(Load.offset, 0),
        emit='''
        PUT_OP(bits, rex2(in_reg0, out_reg0), sink);
        modrm_rm(in_reg0, out_reg0, sink);
        ''')

# XX /r load with 8-bit offset.
ldDisp8 = TailRecipe(
        'ldDisp8', Load, size=2, ins=(GPR), outs=(GPR),
        instp=IsSignedInt(Load.offset, 8),
        emit='''
        PUT_OP(bits, rex2(in_reg0, out_reg0), sink);
        modrm_disp8(in_reg0, out_reg0, sink);
        let offset: i32 = offset.into();
        sink.put1(offset as u8);
        ''')

# XX /r float load with 8-bit offset.
fldDisp8 = TailRecipe(
        'fldDisp8', Load, size=2, ins=(GPR), outs=(FPR),
        instp=IsSignedInt(Load.offset, 8),
        emit='''
        PUT_OP(bits, rex2(in_reg0, out_reg0), sink);
        modrm_disp8(in_reg0, out_reg0, sink);
        let offset: i32 = offset.into();
        sink.put1(offset as u8);
        ''')

# XX /r load with 32-bit offset.
ldDisp32 = TailRecipe(
        'ldDisp32', Load, size=5, ins=(GPR), outs=(GPR),
        instp=IsSignedInt(Load.offset, 32),
        emit='''
        PUT_OP(bits, rex2(in_reg0, out_reg0), sink);
        modrm_disp32(in_reg0, out_reg0, sink);
        let offset: i32 = offset.into();
        sink.put4(offset as u32);
        ''')

# XX /r float load with 32-bit offset.
fldDisp32 = TailRecipe(
        'fldDisp32', Load, size=5, ins=(GPR), outs=(FPR),
        instp=IsSignedInt(Load.offset, 32),
        emit='''
        PUT_OP(bits, rex2(in_reg0, out_reg0), sink);
        modrm_disp32(in_reg0, out_reg0, sink);
        let offset: i32 = offset.into();
        sink.put4(offset as u32);
        ''')

# Unary fill with SIB and 32-bit displacement.
fiSib32 = TailRecipe(
        'fiSib32', Unary, size=6, ins=StackGPR32, outs=GPR,
        emit='''
        let base = stk_base(in_stk0.base);
        PUT_OP(bits, rex2(base, out_reg0), sink);
        modrm_sib_disp32(out_reg0, sink);
        sib_noindex(base, sink);
        sink.put4(in_stk0.offset as u32);
        ''')
ffiSib32 = TailRecipe(
        'ffiSib32', Unary, size=6, ins=StackFPR32, outs=FPR,
        emit='''
        let base = stk_base(in_stk0.base);
        PUT_OP(bits, rex2(base, out_reg0), sink);
        modrm_sib_disp32(out_reg0, sink);
        sib_noindex(base, sink);
        sink.put4(in_stk0.offset as u32);
        ''')

#
# Call/return
#
call_id = TailRecipe(
        'call_id', Call, size=4, ins=(), outs=(),
        emit='''
        PUT_OP(bits, BASE_REX, sink);
        sink.reloc_func(RelocKind::PCRel4.into(), func_ref);
        sink.put4(0);
        ''')

call_r = TailRecipe(
        'call_r', IndirectCall, size=1, ins=GPR, outs=(),
        emit='''
        PUT_OP(bits, rex1(in_reg0), sink);
        modrm_r_bits(in_reg0, bits, sink);
        ''')

ret = TailRecipe(
        'ret', MultiAry, size=0, ins=(), outs=(),
        emit='''
        PUT_OP(bits, BASE_REX, sink);
        ''')

#
# Branches
#
jmpb = TailRecipe(
        'jmpb', Jump, size=1, ins=(), outs=(),
        branch_range=8,
        emit='''
        PUT_OP(bits, BASE_REX, sink);
        disp1(destination, func, sink);
        ''')

jmpd = TailRecipe(
        'jmpd', Jump, size=4, ins=(), outs=(),
        branch_range=32,
        emit='''
        PUT_OP(bits, BASE_REX, sink);
        disp4(destination, func, sink);
        ''')

# Test-and-branch.
#
# This recipe represents the macro fusion of a test and a conditional branch.
# This serves two purposes:
#
# 1. Guarantee that the test and branch get scheduled next to each other so
#    macro fusion is guaranteed to be possible.
# 2. Hide the status flags from Cretonne which doesn't currently model flags.
#
# The encoding bits affect both the test and the branch instruction:
#
# Bits 0-7 are the Jcc opcode.
# Bits 8-15 control the test instruction which always has opcode byte 0x85.
tjccb = TailRecipe(
        'tjccb', Branch, size=1 + 2, ins=GPR, outs=(),
        branch_range=8,
        emit='''
        // test r, r.
        PUT_OP((bits & 0xff00) | 0x85, rex2(in_reg0, in_reg0), sink);
        modrm_rr(in_reg0, in_reg0, sink);
        // Jcc instruction.
        sink.put1(bits as u8);
        disp1(destination, func, sink);
        ''')

tjccd = TailRecipe(
        'tjccd', Branch, size=1 + 6, ins=GPR, outs=(),
        branch_range=32,
        emit='''
        // test r, r.
        PUT_OP((bits & 0xff00) | 0x85, rex2(in_reg0, in_reg0), sink);
        modrm_rr(in_reg0, in_reg0, sink);
        // Jcc instruction.
        sink.put1(0x0f);
        sink.put1(bits as u8);
        disp4(destination, func, sink);
        ''')

# 8-bit test-and-branch.
#
# Same as tjccb, but only looks at the low 8 bits of the register, for b1
# types.
t8jccb_abcd = TailRecipe(
        't8jccb_abcd', Branch, size=1 + 2, ins=ABCD, outs=(),
        branch_range=8,
        emit='''
        // test8 r, r.
        PUT_OP((bits & 0xff00) | 0x84, rex2(in_reg0, in_reg0), sink);
        modrm_rr(in_reg0, in_reg0, sink);
        // Jcc instruction.
        sink.put1(bits as u8);
        disp1(destination, func, sink);
        ''')

t8jccd_abcd = TailRecipe(
        't8jccd_abcd', Branch, size=1 + 6, ins=ABCD, outs=(),
        branch_range=32,
        emit='''
        // test8 r, r.
        PUT_OP((bits & 0xff00) | 0x84, rex2(in_reg0, in_reg0), sink);
        modrm_rr(in_reg0, in_reg0, sink);
        // Jcc instruction.
        sink.put1(0x0f);
        sink.put1(bits as u8);
        disp4(destination, func, sink);
        ''')

# Comparison that produces a `b1` result in a GPR.
#
# This is a macro of a `cmp` instruction followed by a `setCC` instruction.
# This is not a great solution because:
#
# - The cmp+setcc combination is not recognized by CPU's macro fusion.
# - The 64-bit encoding has issues with REX prefixes. The `cmp` and `setCC`
#   instructions may need a REX independently.
# - Modeling CPU flags in the type system would be better.
#
# Since the `setCC` instructions only write an 8-bit register, we use that as
# our `b1` representation: A `b1` value is represented as a GPR where the low 8
# bits are known to be 0 or 1. The high bits are undefined.
#
# This bandaid macro doesn't support a REX prefix for the final `setCC`
# instruction, so it is limited to the `ABCD` register class for booleans.
icscc = TailRecipe(
        'icscc', IntCompare, size=1 + 3, ins=(GPR, GPR), outs=ABCD,
        emit='''
        // Comparison instruction.
        PUT_OP(bits, rex2(in_reg0, in_reg1), sink);
        modrm_rr(in_reg0, in_reg1, sink);
        // `setCC` instruction, no REX.
        use ir::condcodes::IntCC::*;
        let setcc = match cond {
            Equal => 0x94,
            NotEqual => 0x95,
            SignedLessThan => 0x9c,
            SignedGreaterThanOrEqual => 0x9d,
            SignedGreaterThan => 0x9f,
            SignedLessThanOrEqual => 0x9e,
            UnsignedLessThan => 0x92,
            UnsignedGreaterThanOrEqual => 0x93,
            UnsignedGreaterThan => 0x97,
            UnsignedLessThanOrEqual => 0x96,
        };
        sink.put1(0x0f);
        sink.put1(setcc);
        modrm_rr(out_reg0, 0, sink);
        ''')


# Make a FloatCompare instruction predicate with the supported condition codes.

# Same thing for floating point.
#
# The ucomiss/ucomisd instructions set the EFLAGS bits CF/PF/CF like this:
#
#    ZPC OSA
# UN 111 000
# GT 000 000
# LT 001 000
# EQ 100 000
#
# Not all floating point condition codes are supported.
fcscc = TailRecipe(
        'fcscc', FloatCompare, size=1 + 3, ins=(FPR, FPR), outs=ABCD,
        instp=Or(*(IsEqual(FloatCompare.cond, cc)
                   for cc in supported_floatccs)),
        emit='''
        // Comparison instruction.
        PUT_OP(bits, rex2(in_reg1, in_reg0), sink);
        modrm_rr(in_reg1, in_reg0, sink);
        // `setCC` instruction, no REX.
        use ir::condcodes::FloatCC::*;
        let setcc = match cond {
            Ordered                    => 0x9b, // EQ|LT|GT => setnp (P=0)
            Unordered                  => 0x9a, // UN       => setp  (P=1)
            OrderedNotEqual            => 0x95, // LT|GT    => setne (Z=0),
            UnorderedOrEqual           => 0x94, // UN|EQ    => sete  (Z=1)
            GreaterThan                => 0x97, // GT       => seta  (C=0&Z=0)
            GreaterThanOrEqual         => 0x93, // GT|EQ    => setae (C=0)
            UnorderedOrLessThan        => 0x92, // UN|LT    => setb  (C=1)
            UnorderedOrLessThanOrEqual => 0x96, // UN|LT|EQ => setbe (Z=1|C=1)
            Equal |                       // EQ
            NotEqual |                    // UN|LT|GT
            LessThan |                    // LT
            LessThanOrEqual |             // LT|EQ
            UnorderedOrGreaterThan |      // UN|GT
            UnorderedOrGreaterThanOrEqual // UN|GT|EQ
            => panic!("{} not supported by fcscc", cond),
        };
        sink.put1(0x0f);
        sink.put1(setcc);
        modrm_rr(out_reg0, 0, sink);
        ''')

TailRecipe.check_names(globals())
