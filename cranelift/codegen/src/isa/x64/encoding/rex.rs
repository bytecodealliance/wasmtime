//! Encodes instructions in the standard x86 encoding mode. This is called
//! IA-32E mode in the Intel manuals but corresponds to the addition of the
//! REX-prefix format (hence the name of this module) that allowed encoding
//! instructions in both compatibility mode (32-bit instructions running on a
//! 64-bit OS) and in 64-bit mode (using the full 64-bit address space).
//!
//! For all of the routines that take both a memory-or-reg operand (sometimes
//! called "E" in the Intel documentation, see the Intel Developer's manual,
//! vol. 2, section A.2) and a reg-only operand ("G" in Intel-ese), the order is
//! always G first, then E. The term "enc" in the following means "hardware
//! register encoding number".

use super::ByteSink;
use crate::isa::x64::inst::args::{Amode, OperandSize};
use crate::isa::x64::inst::{regs, Inst, LabelUse};
use crate::machinst::{MachBuffer, Reg, RegClass};

pub(crate) fn low8_will_sign_extend_to_64(x: u32) -> bool {
    let xs = (x as i32) as i64;
    xs == ((xs << 56) >> 56)
}

pub(crate) fn low8_will_sign_extend_to_32(x: u32) -> bool {
    let xs = x as i32;
    xs == ((xs << 24) >> 24)
}

/// Encode the ModR/M byte.
#[inline(always)]
pub fn encode_modrm(m0d: u8, enc_reg_g: u8, rm_e: u8) -> u8 {
    debug_assert!(m0d < 4);
    debug_assert!(enc_reg_g < 8);
    debug_assert!(rm_e < 8);
    ((m0d & 3) << 6) | ((enc_reg_g & 7) << 3) | (rm_e & 7)
}

#[inline(always)]
pub(crate) fn encode_sib(shift: u8, enc_index: u8, enc_base: u8) -> u8 {
    debug_assert!(shift < 4);
    debug_assert!(enc_index < 8);
    debug_assert!(enc_base < 8);
    ((shift & 3) << 6) | ((enc_index & 7) << 3) | (enc_base & 7)
}

/// Get the encoding number of a GPR.
#[inline(always)]
pub(crate) fn int_reg_enc(reg: impl Into<Reg>) -> u8 {
    let reg = reg.into();
    debug_assert!(reg.is_real(), "reg = {reg:?}");
    debug_assert_eq!(reg.class(), RegClass::Int);
    reg.to_real_reg().unwrap().hw_enc()
}

/// Get the encoding number of any register.
#[inline(always)]
pub(crate) fn reg_enc(reg: impl Into<Reg>) -> u8 {
    let reg = reg.into();
    debug_assert!(reg.is_real());
    reg.to_real_reg().unwrap().hw_enc()
}

/// A small bit field to record a REX prefix specification:
/// - bit 0 set to 1 indicates REX.W must be 0 (cleared).
/// - bit 1 set to 1 indicates the REX prefix must always be emitted.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct RexFlags(u8);

impl RexFlags {
    /// By default, set the W field, and don't always emit.
    #[inline(always)]
    pub fn set_w() -> Self {
        Self(0)
    }

    /// Creates a new RexPrefix for which the REX.W bit will be cleared.
    #[inline(always)]
    pub fn clear_w() -> Self {
        Self(1)
    }

    /// True if 64-bit operands are used.
    #[inline(always)]
    pub fn must_clear_w(&self) -> bool {
        (self.0 & 1) != 0
    }

    /// Require that the REX prefix is emitted.
    #[inline(always)]
    pub fn always_emit(&mut self) -> &mut Self {
        self.0 = self.0 | 2;
        self
    }

    /// True if the REX prefix must always be emitted.
    #[inline(always)]
    pub fn must_always_emit(&self) -> bool {
        (self.0 & 2) != 0
    }

    /// Emit the rex prefix if the referenced register would require it for 8-bit operations.
    #[inline(always)]
    pub fn always_emit_if_8bit_needed(&mut self, reg: Reg) -> &mut Self {
        let enc_reg = int_reg_enc(reg);
        if enc_reg >= 4 && enc_reg <= 7 {
            self.always_emit();
        }
        self
    }

    /// Emit a unary instruction.
    #[inline(always)]
    pub fn emit_one_op<BS: ByteSink + ?Sized>(&self, sink: &mut BS, enc_e: u8) {
        // Register Operand coded in Opcode Byte
        // REX.R and REX.X unused
        // REX.B == 1 accesses r8-r15
        let w = if self.must_clear_w() { 0 } else { 1 };
        let r = 0;
        let x = 0;
        let b = (enc_e >> 3) & 1;
        let rex = 0x40 | (w << 3) | (r << 2) | (x << 1) | b;
        if rex != 0x40 || self.must_always_emit() {
            sink.put1(rex);
        }
    }

    /// Emit a binary instruction.
    #[inline(always)]
    pub fn emit_two_op<BS: ByteSink + ?Sized>(&self, sink: &mut BS, enc_g: u8, enc_e: u8) {
        let w = if self.must_clear_w() { 0 } else { 1 };
        let r = (enc_g >> 3) & 1;
        let x = 0;
        let b = (enc_e >> 3) & 1;
        let rex = 0x40 | (w << 3) | (r << 2) | (x << 1) | b;
        if rex != 0x40 || self.must_always_emit() {
            sink.put1(rex);
        }
    }

    /// Emit a ternary instruction.
    #[inline(always)]
    pub fn emit_three_op<BS: ByteSink + ?Sized>(
        &self,
        sink: &mut BS,
        enc_g: u8,
        enc_index: u8,
        enc_base: u8,
    ) {
        let w = if self.must_clear_w() { 0 } else { 1 };
        let r = (enc_g >> 3) & 1;
        let x = (enc_index >> 3) & 1;
        let b = (enc_base >> 3) & 1;
        let rex = 0x40 | (w << 3) | (r << 2) | (x << 1) | b;
        if rex != 0x40 || self.must_always_emit() {
            sink.put1(rex);
        }
    }
}

/// Generate the proper Rex flags for the given operand size.
impl From<OperandSize> for RexFlags {
    fn from(size: OperandSize) -> Self {
        match size {
            OperandSize::Size64 => RexFlags::set_w(),
            _ => RexFlags::clear_w(),
        }
    }
}
/// Generate Rex flags for an OperandSize/register tuple.
impl From<(OperandSize, Reg)> for RexFlags {
    fn from((size, reg): (OperandSize, Reg)) -> Self {
        let mut rex = RexFlags::from(size);
        if size == OperandSize::Size8 {
            rex.always_emit_if_8bit_needed(reg);
        }
        rex
    }
}

/// Allows using the same opcode byte in different "opcode maps" to allow for more instruction
/// encodings. See appendix A in the Intel Software Developer's Manual, volume 2A, for more details.
#[allow(missing_docs)]
#[derive(PartialEq)]
pub enum OpcodeMap {
    None,
    _0F,
    _0F38,
    _0F3A,
}

impl OpcodeMap {
    /// Normally the opcode map is specified as bytes in the instruction, but some x64 encoding
    /// formats pack this information as bits in a prefix (e.g. VEX / EVEX).
    pub(crate) fn bits(&self) -> u8 {
        match self {
            OpcodeMap::None => 0b00,
            OpcodeMap::_0F => 0b01,
            OpcodeMap::_0F38 => 0b10,
            OpcodeMap::_0F3A => 0b11,
        }
    }
}

impl Default for OpcodeMap {
    fn default() -> Self {
        Self::None
    }
}

/// We may need to include one or more legacy prefix bytes before the REX prefix.  This enum
/// covers only the small set of possibilities that we actually need.
#[derive(PartialEq)]
pub enum LegacyPrefixes {
    /// No prefix bytes.
    None,
    /// Operand Size Override -- here, denoting "16-bit operation".
    _66,
    /// The Lock prefix.
    _F0,
    /// Operand size override and Lock.
    _66F0,
    /// REPNE, but no specific meaning here -- is just an opcode extension.
    _F2,
    /// REP/REPE, but no specific meaning here -- is just an opcode extension.
    _F3,
    /// Operand size override and same effect as F3.
    _66F3,
}

impl LegacyPrefixes {
    /// Emit the legacy prefix as bytes (e.g. in REX instructions).
    #[inline(always)]
    pub(crate) fn emit<BS: ByteSink + ?Sized>(&self, sink: &mut BS) {
        match self {
            Self::_66 => sink.put1(0x66),
            Self::_F0 => sink.put1(0xF0),
            Self::_66F0 => {
                // I don't think the order matters, but in any case, this is the same order that
                // the GNU assembler uses.
                sink.put1(0x66);
                sink.put1(0xF0);
            }
            Self::_F2 => sink.put1(0xF2),
            Self::_F3 => sink.put1(0xF3),
            Self::_66F3 => {
                sink.put1(0x66);
                sink.put1(0xF3);
            }
            Self::None => (),
        }
    }

    /// Emit the legacy prefix as bits (e.g. for EVEX instructions).
    #[inline(always)]
    pub(crate) fn bits(&self) -> u8 {
        match self {
            Self::None => 0b00,
            Self::_66 => 0b01,
            Self::_F3 => 0b10,
            Self::_F2 => 0b11,
            _ => panic!(
                "VEX and EVEX bits can only be extracted from single prefixes: None, 66, F3, F2"
            ),
        }
    }
}

impl Default for LegacyPrefixes {
    fn default() -> Self {
        Self::None
    }
}

/// This is the core 'emit' function for instructions that reference memory.
///
/// For an instruction that has as operands a reg encoding `enc_g` and a memory address `mem_e`,
/// create and emit:
/// - first the legacy prefixes, if any
/// - then the REX prefix, if needed
/// - then caller-supplied opcode byte(s) (`opcodes` and `num_opcodes`),
/// - then the MOD/RM byte,
/// - then optionally, a SIB byte,
/// - and finally optionally an immediate that will be derived from the `mem_e` operand.
///
/// For most instructions up to and including SSE4.2, that will be the whole instruction: this is
/// what we call "standard" instructions, and abbreviate "std" in the name here. VEX-prefixed
/// instructions will require their own emitter functions.
///
/// This will also work for 32-bits x86 instructions, assuming no REX prefix is provided.
///
/// The opcodes are written bigendianly for the convenience of callers.  For example, if the opcode
/// bytes to be emitted are, in this order, F3 0F 27, then the caller should pass `opcodes` ==
/// 0xF3_0F_27 and `num_opcodes` == 3.
///
/// The register operand is represented here not as a `Reg` but as its hardware encoding, `enc_g`.
/// `rex` can specify special handling for the REX prefix.  By default, the REX prefix will
/// indicate a 64-bit operation and will be deleted if it is redundant (0x40).  Note that for a
/// 64-bit operation, the REX prefix will normally never be redundant, since REX.W must be 1 to
/// indicate a 64-bit operation.
pub(crate) fn emit_std_enc_mem(
    sink: &mut MachBuffer<Inst>,
    prefixes: LegacyPrefixes,
    opcodes: u32,
    mut num_opcodes: usize,
    enc_g: u8,
    mem_e: &Amode,
    rex: RexFlags,
    bytes_at_end: u8,
) {
    // General comment for this function: the registers in `mem_e` must be
    // 64-bit integer registers, because they are part of an address
    // expression.  But `enc_g` can be derived from a register of any class.

    if let Some(trap_code) = mem_e.get_flags().trap_code() {
        sink.add_trap(trap_code);
    }

    prefixes.emit(sink);

    // After prefixes, first emit the REX byte depending on the kind of
    // addressing mode that's being used.
    match *mem_e {
        Amode::ImmReg { base, .. } => {
            let enc_e = int_reg_enc(base);
            rex.emit_two_op(sink, enc_g, enc_e);
        }

        Amode::ImmRegRegShift {
            base: reg_base,
            index: reg_index,
            ..
        } => {
            let enc_base = int_reg_enc(*reg_base);
            let enc_index = int_reg_enc(*reg_index);
            rex.emit_three_op(sink, enc_g, enc_index, enc_base);
        }

        Amode::RipRelative { .. } => {
            // note REX.B = 0.
            rex.emit_two_op(sink, enc_g, 0);
        }
    }

    // Now the opcode(s).  These include any other prefixes the caller
    // hands to us.
    while num_opcodes > 0 {
        num_opcodes -= 1;
        sink.put1(((opcodes >> (num_opcodes << 3)) & 0xFF) as u8);
    }

    // And finally encode the mod/rm bytes and all further information.
    emit_modrm_sib_disp(sink, enc_g, mem_e, bytes_at_end, None)
}

pub(crate) fn emit_modrm_sib_disp(
    sink: &mut MachBuffer<Inst>,
    enc_g: u8,
    mem_e: &Amode,
    bytes_at_end: u8,
    evex_scaling: Option<i8>,
) {
    match *mem_e {
        Amode::ImmReg { simm32, base, .. } => {
            let enc_e = int_reg_enc(base);
            let mut imm = Imm::new(simm32, evex_scaling);

            // Most base registers allow for a single ModRM byte plus an
            // optional immediate. If rsp is the base register, however, then a
            // SIB byte must be used.
            let enc_e_low3 = enc_e & 7;
            if enc_e_low3 != regs::ENC_RSP {
                // If the base register is rbp and there's no offset then force
                // a 1-byte zero offset since otherwise the encoding would be
                // invalid.
                if enc_e_low3 == regs::ENC_RBP {
                    imm.force_immediate();
                }
                sink.put1(encode_modrm(imm.m0d(), enc_g & 7, enc_e & 7));
                imm.emit(sink);
            } else {
                // Displacement from RSP is encoded with a SIB byte where
                // the index and base are both encoded as RSP's encoding of
                // 0b100. This special encoding means that the index register
                // isn't used and the base is 0b100 with or without a
                // REX-encoded 4th bit (e.g. rsp or r12)
                sink.put1(encode_modrm(imm.m0d(), enc_g & 7, 0b100));
                sink.put1(0b00_100_100);
                imm.emit(sink);
            }
        }

        Amode::ImmRegRegShift {
            simm32,
            base: reg_base,
            index: reg_index,
            shift,
            ..
        } => {
            let enc_base = int_reg_enc(*reg_base);
            let enc_index = int_reg_enc(*reg_index);

            // Encoding of ModRM/SIB bytes don't allow the index register to
            // ever be rsp. Note, though, that the encoding of r12, whose three
            // lower bits match the encoding of rsp, is explicitly allowed with
            // REX bytes so only rsp is disallowed.
            assert!(enc_index != regs::ENC_RSP);

            // If the offset is zero then there is no immediate. Note, though,
            // that if the base register's lower three bits are `101` then an
            // offset must be present. This is a special case in the encoding of
            // the SIB byte and requires an explicit displacement with rbp/r13.
            let mut imm = Imm::new(simm32, evex_scaling);
            if enc_base & 7 == regs::ENC_RBP {
                imm.force_immediate();
            }

            // With the above determined encode the ModRM byte, then the SIB
            // byte, then any immediate as necessary.
            sink.put1(encode_modrm(imm.m0d(), enc_g & 7, 0b100));
            sink.put1(encode_sib(shift, enc_index & 7, enc_base & 7));
            imm.emit(sink);
        }

        Amode::RipRelative { ref target } => {
            // RIP-relative is mod=00, rm=101.
            sink.put1(encode_modrm(0b00, enc_g & 7, 0b101));

            let offset = sink.cur_offset();
            sink.use_label_at_offset(offset, *target, LabelUse::JmpRel32);
            // N.B.: some instructions (XmmRmRImm format for example)
            // have bytes *after* the RIP-relative offset. The
            // addressed location is relative to the end of the
            // instruction, but the relocation is nominally relative
            // to the end of the u32 field. So, to compensate for
            // this, we emit a negative extra offset in the u32 field
            // initially, and the relocation will add to it.
            sink.put4(-(i32::from(bytes_at_end)) as u32);
        }
    }
}

#[derive(Copy, Clone)]
enum Imm {
    None,
    Imm8(i8),
    Imm32(i32),
}

impl Imm {
    /// Classifies the 32-bit immediate `val` as how this can be encoded
    /// with ModRM/SIB bytes.
    ///
    /// For `evex_scaling` according to Section 2.7.5 of Intel's manual:
    ///
    /// > EVEX-encoded instructions always use a compressed displacement scheme
    /// > by multiplying disp8 in conjunction with a scaling factor N that is
    /// > determined based on the vector length, the value of EVEX.b bit
    /// > (embedded broadcast) and the input element size of the instruction
    ///
    /// The `evex_scaling` factor provided here is `Some(N)` for EVEX
    /// instructions.  This is taken into account where the `Imm` value
    /// contained is the raw byte offset.
    fn new(val: i32, evex_scaling: Option<i8>) -> Imm {
        if val == 0 {
            return Imm::None;
        }
        match evex_scaling {
            Some(scaling) => {
                if val % i32::from(scaling) == 0 {
                    let scaled = val / i32::from(scaling);
                    if low8_will_sign_extend_to_32(scaled as u32) {
                        return Imm::Imm8(scaled as i8);
                    }
                }
                Imm::Imm32(val)
            }
            None => match i8::try_from(val) {
                Ok(val) => Imm::Imm8(val),
                Err(_) => Imm::Imm32(val),
            },
        }
    }

    /// Forces `Imm::None` to become `Imm::Imm8(0)`, used for special cases
    /// where some base registers require an immediate.
    fn force_immediate(&mut self) {
        if let Imm::None = self {
            *self = Imm::Imm8(0);
        }
    }

    /// Returns the two "mod" bits present at the upper bits of the mod/rm
    /// byte.
    fn m0d(&self) -> u8 {
        match self {
            Imm::None => 0b00,
            Imm::Imm8(_) => 0b01,
            Imm::Imm32(_) => 0b10,
        }
    }

    fn emit<BS: ByteSink + ?Sized>(&self, sink: &mut BS) {
        match self {
            Imm::None => {}
            Imm::Imm8(n) => sink.put1(*n as u8),
            Imm::Imm32(n) => sink.put4(*n as u32),
        }
    }
}

/// This is the core 'emit' function for instructions that do not reference memory.
///
/// This is conceptually the same as emit_modrm_sib_enc_ge, except it is for the case where the E
/// operand is a register rather than memory.  Hence it is much simpler.
pub(crate) fn emit_std_enc_enc<BS: ByteSink + ?Sized>(
    sink: &mut BS,
    prefixes: LegacyPrefixes,
    opcodes: u32,
    mut num_opcodes: usize,
    enc_g: u8,
    enc_e: u8,
    rex: RexFlags,
) {
    // EncG and EncE can be derived from registers of any class, and they
    // don't even have to be from the same class.  For example, for an
    // integer-to-FP conversion insn, one might be RegClass::I64 and the other
    // RegClass::V128.

    // The legacy prefixes.
    prefixes.emit(sink);

    // The rex byte.
    rex.emit_two_op(sink, enc_g, enc_e);

    // All other prefixes and opcodes.
    while num_opcodes > 0 {
        num_opcodes -= 1;
        sink.put1(((opcodes >> (num_opcodes << 3)) & 0xFF) as u8);
    }

    // Now the mod/rm byte.  The instruction we're generating doesn't access
    // memory, so there is no SIB byte or immediate -- we're done.
    sink.put1(encode_modrm(0b11, enc_g & 7, enc_e & 7));
}

// These are merely wrappers for the above two functions that facilitate passing
// actual `Reg`s rather than their encodings.

pub(crate) fn emit_std_reg_mem(
    sink: &mut MachBuffer<Inst>,
    prefixes: LegacyPrefixes,
    opcodes: u32,
    num_opcodes: usize,
    reg_g: Reg,
    mem_e: &Amode,
    rex: RexFlags,
    bytes_at_end: u8,
) {
    let enc_g = reg_enc(reg_g);
    emit_std_enc_mem(
        sink,
        prefixes,
        opcodes,
        num_opcodes,
        enc_g,
        mem_e,
        rex,
        bytes_at_end,
    );
}

pub(crate) fn emit_std_reg_reg<BS: ByteSink + ?Sized>(
    sink: &mut BS,
    prefixes: LegacyPrefixes,
    opcodes: u32,
    num_opcodes: usize,
    reg_g: Reg,
    reg_e: Reg,
    rex: RexFlags,
) {
    let enc_g = reg_enc(reg_g);
    let enc_e = reg_enc(reg_e);
    emit_std_enc_enc(sink, prefixes, opcodes, num_opcodes, enc_g, enc_e, rex);
}

/// Write a suitable number of bits from an imm64 to the sink.
pub(crate) fn emit_simm<BS: ByteSink + ?Sized>(sink: &mut BS, size: u8, simm32: u32) {
    match size {
        8 | 4 => sink.put4(simm32),
        2 => sink.put2(simm32 as u16),
        1 => sink.put1(simm32 as u8),
        _ => unreachable!(),
    }
}
