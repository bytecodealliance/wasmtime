//! Generate format-related Rust code; this also includes generation of encoding
//! Rust code.
use super::{Formatter, fmtln};
use crate::dsl;

/// Different methods of emitting a ModR/M operand and encoding various bits and
/// pieces of information into it. The REX/VEX formats plus the operand kinds
/// dictate how exactly each instruction uses this, if at all.
#[derive(Copy, Clone)]
enum ModRmStyle {
    /// This instruction does not use a ModR/M byte.
    None,

    /// The R/M bits are encoded with `rm` which is a `Gpr` or `Xmm` (it does
    /// not have a "mem" possibility), and the Reg/Opcode bits are encoded
    /// with `reg`.
    Reg { reg: ModRmReg, rm: dsl::Location },

    /// The R/M bits are encoded with `rm` which is a `GprMem` or `XmmMem`, and
    /// the Reg/Opcode bits are encoded with `reg`.
    RegMem { reg: ModRmReg, rm: dsl::Location },

    /// Same as `RegMem` above except that this is also used for VEX-encoded
    /// instructios with "/is4" which indicates that the 4th register operand
    /// is encoded in a byte after the ModR/M byte.
    RegMemIs4 {
        reg: ModRmReg,
        rm: dsl::Location,
        is4: dsl::Location,
    },
}

/// Different methods of encoding the Reg/Opcode bits in a ModR/M byte.
#[derive(Copy, Clone)]
enum ModRmReg {
    /// A static set of bits is used.
    Digit(u8),
    /// A runtime-defined register is used with this field name.
    Reg(dsl::Location),
}

impl dsl::Format {
    /// Re-order the Intel-style operand order to accommodate ATT-style
    /// printing.
    ///
    /// This is an unfortunate necessity to match Cranelift's current
    /// disassembly, which uses AT&T-style printing. The plan is to eventually
    /// transition to Intel-style printing (and avoid this awkward reordering)
    /// once Cranelift has switched to using this assembler predominantly
    /// (TODO).
    #[must_use]
    pub(crate) fn generate_att_style_operands(&self) -> String {
        let ordered_ops: Vec<_> = self
            .operands
            .iter()
            .filter(|o| !o.implicit)
            .rev()
            .map(|o| format!("{{{}}}", o.location))
            .collect();
        ordered_ops.join(", ")
    }

    #[must_use]
    pub(crate) fn generate_implicit_operands(&self) -> String {
        let ops: Vec<_> = self
            .operands
            .iter()
            .filter(|o| o.implicit)
            .map(|o| format!("{{{}}}", o.location))
            .collect();
        if ops.is_empty() {
            String::new()
        } else {
            format!(" ;; implicit: {}", ops.join(", "))
        }
    }

    pub(crate) fn generate_rex_encoding(&self, f: &mut Formatter, rex: &dsl::Rex) {
        self.generate_prefixes(f, rex);
        let style = self.generate_rex_prefix(f, rex);
        rex.generate_opcodes(f, self.locations().next());
        self.generate_modrm_byte(f, style);
        self.generate_immediate(f, style);
    }

    pub fn generate_vex_encoding(&self, f: &mut Formatter, vex: &dsl::Vex) {
        let style = self.generate_vex_prefix(f, vex);
        vex.generate_opcode(f);
        self.generate_modrm_byte(f, style);
        self.generate_immediate(f, style);
    }

    /// `buf.put1(...);`
    fn generate_prefixes(&self, f: &mut Formatter, rex: &dsl::Rex) {
        if !rex.opcodes.prefixes.is_empty() {
            f.empty_line();
            f.comment("Emit prefixes.");
        }
        if let Some(group1) = &rex.opcodes.prefixes.group1 {
            fmtln!(f, "buf.put1({group1});");
        }
        if let Some(group2) = &rex.opcodes.prefixes.group2 {
            fmtln!(f, "buf.put1({group2});");
        }
        if let Some(group3) = &rex.opcodes.prefixes.group3 {
            fmtln!(f, "buf.put1({group3});");
        }
        if let Some(group4) = &rex.opcodes.prefixes.group4 {
            fmtln!(f, "buf.put1({group4});");
        }
    }

    fn generate_rex_prefix(&self, f: &mut Formatter, rex: &dsl::Rex) -> ModRmStyle {
        use dsl::OperandKind::{FixedReg, Imm, Mem, Reg, RegMem};

        // If this instruction has only immediates there's no rex/modrm/etc, so
        // skip everything below.
        match self.operands_by_kind().as_slice() {
            [Imm(_)] => return ModRmStyle::None,
            _ => {}
        }

        f.empty_line();
        f.comment("Possibly emit REX prefix.");

        let find_8bit_registers =
            |l: &dsl::Location| l.bits() == 8 && matches!(l.kind(), Reg(_) | RegMem(_));
        let uses_8bit = self.locations().any(find_8bit_registers);
        fmtln!(f, "let uses_8bit = {uses_8bit};");
        fmtln!(f, "let w_bit = {};", rex.w);
        let bits = "w_bit, uses_8bit";

        let style = match self.operands_by_kind().as_slice() {
            [FixedReg(dst), FixedReg(_)] | [FixedReg(dst)] | [FixedReg(dst), Imm(_)] => {
                // TODO: don't emit REX byte here.
                assert_eq!(rex.unwrap_digit(), None);
                fmtln!(f, "let digit = 0;");
                fmtln!(f, "let dst = self.{dst}.enc();");
                fmtln!(f, "let rex = RexPrefix::with_digit(digit, dst, {bits});");
                ModRmStyle::None
            }
            [Reg(dst)] => {
                assert_eq!(rex.unwrap_digit(), None);
                assert!(rex.opcode_mod.is_some());
                fmtln!(f, "let dst = self.{dst}.enc();");
                fmtln!(f, "let rex = RexPrefix::one_op(dst, {bits});");
                ModRmStyle::None
            }
            [Reg(dst), Imm(_)] => match rex.unwrap_digit() {
                Some(digit) => {
                    fmtln!(f, "let digit = 0x{digit:x};");
                    fmtln!(f, "let dst = self.{dst}.enc();");
                    fmtln!(f, "let rex = RexPrefix::two_op(digit, dst, {bits});");
                    ModRmStyle::Reg {
                        reg: ModRmReg::Digit(digit),
                        rm: *dst,
                    }
                }
                None => {
                    assert!(rex.opcode_mod.is_some());
                    fmtln!(f, "let dst = self.{dst}.enc();");
                    fmtln!(f, "let rex = RexPrefix::one_op(dst, {bits});");
                    ModRmStyle::None
                }
            },
            [FixedReg(_), RegMem(mem)]
            | [FixedReg(_), FixedReg(_), RegMem(mem)]
            | [RegMem(mem), FixedReg(_)]
            | [Mem(mem), Imm(_)]
            | [RegMem(mem), Imm(_)]
            | [RegMem(mem)] => {
                let digit = rex.unwrap_digit().unwrap();
                fmtln!(f, "let digit = 0x{digit:x};");
                fmtln!(f, "let rex = self.{mem}.as_rex_prefix(digit, {bits});");
                ModRmStyle::RegMem {
                    reg: ModRmReg::Digit(digit),
                    rm: *mem,
                }
            }
            [Reg(reg), RegMem(mem) | Mem(mem)]
            | [Reg(reg), RegMem(mem), Imm(_) | FixedReg(_)]
            | [RegMem(mem) | Mem(mem), Reg(reg)]
            | [RegMem(mem), Reg(reg), Imm(_) | FixedReg(_)] => {
                fmtln!(f, "let reg = self.{reg}.enc();");
                fmtln!(f, "let rex = self.{mem}.as_rex_prefix(reg, {bits});");
                ModRmStyle::RegMem {
                    reg: ModRmReg::Reg(*reg),
                    rm: *mem,
                }
            }
            [Reg(dst), Reg(src), Imm(_)] | [Reg(dst), Reg(src)] => {
                fmtln!(f, "let reg = self.{dst}.enc();");
                fmtln!(f, "let rm = self.{src}.enc();");
                fmtln!(f, "let rex = RexPrefix::two_op(reg, rm, {bits});");
                ModRmStyle::Reg {
                    reg: ModRmReg::Reg(*dst),
                    rm: *src,
                }
            }
            unknown => unimplemented!("unknown pattern: {unknown:?}"),
        };

        fmtln!(f, "rex.encode(buf);");
        style
    }

    fn generate_vex_prefix(&self, f: &mut Formatter, vex: &dsl::Vex) -> ModRmStyle {
        use dsl::OperandKind::{FixedReg, Imm, Mem, Reg, RegMem};

        f.empty_line();
        f.comment("Emit VEX prefix.");
        fmtln!(f, "let len = {:#03b};", vex.length.bits());
        fmtln!(f, "let pp = {:#04b};", vex.pp.map_or(0b00, |pp| pp.bits()));
        fmtln!(f, "let mmmmm = {:#07b};", vex.mmmmm.unwrap().bits());
        fmtln!(f, "let w = {};", vex.w.as_bool());
        let bits = "len, pp, mmmmm, w";

        let style = match self.operands_by_kind().as_slice() {
            [Reg(reg), Reg(vvvv), Reg(rm)] => {
                assert!(!vex.is4);
                fmtln!(f, "let reg = self.{reg}.enc();");
                fmtln!(f, "let vvvv = self.{vvvv}.enc();");
                fmtln!(f, "let rm = self.{rm}.encode_bx_regs();");
                fmtln!(f, "let vex = VexPrefix::three_op(reg, vvvv, rm, {bits});");
                ModRmStyle::Reg {
                    reg: ModRmReg::Reg(*reg),
                    rm: *rm,
                }
            }
            [Reg(reg), Reg(vvvv), RegMem(rm)]
            | [Reg(reg), Reg(vvvv), Mem(rm)]
            | [Reg(reg), Reg(vvvv), RegMem(rm), Imm(_) | FixedReg(_)]
            | [Reg(reg), RegMem(rm), Reg(vvvv)] => {
                assert!(!vex.is4);
                fmtln!(f, "let reg = self.{reg}.enc();");
                fmtln!(f, "let vvvv = self.{vvvv}.enc();");
                fmtln!(f, "let rm = self.{rm}.encode_bx_regs();");
                fmtln!(f, "let vex = VexPrefix::three_op(reg, vvvv, rm, {bits});");
                ModRmStyle::RegMem {
                    reg: ModRmReg::Reg(*reg),
                    rm: *rm,
                }
            }
            [Reg(reg), Reg(vvvv), RegMem(rm), Reg(is4)] => {
                assert!(vex.is4);
                fmtln!(f, "let reg = self.{reg}.enc();");
                fmtln!(f, "let vvvv = self.{vvvv}.enc();");
                fmtln!(f, "let rm = self.{rm}.encode_bx_regs();");
                fmtln!(f, "let vex = VexPrefix::three_op(reg, vvvv, rm, {bits});");
                ModRmStyle::RegMemIs4 {
                    reg: ModRmReg::Reg(*reg),
                    rm: *rm,
                    is4: *is4,
                }
            }
            [Reg(reg_or_vvvv), RegMem(rm)]
            | [RegMem(rm), Reg(reg_or_vvvv)]
            | [Reg(reg_or_vvvv), RegMem(rm), Imm(_)] => match vex.unwrap_digit() {
                Some(digit) => {
                    assert!(!vex.is4);
                    let vvvv = reg_or_vvvv;
                    fmtln!(f, "let reg = {digit:#x};");
                    fmtln!(f, "let vvvv = self.{vvvv}.enc();");
                    fmtln!(f, "let rm = self.{rm}.encode_bx_regs();");
                    fmtln!(f, "let vex = VexPrefix::three_op(reg, vvvv, rm, {bits});");
                    ModRmStyle::RegMem {
                        reg: ModRmReg::Digit(digit),
                        rm: *rm,
                    }
                }
                None => {
                    assert!(!vex.is4);
                    let reg = reg_or_vvvv;
                    fmtln!(f, "let reg = self.{reg}.enc();");
                    fmtln!(f, "let vvvv = {};", "0b0");
                    fmtln!(f, "let rm = self.{rm}.encode_bx_regs();");
                    fmtln!(f, "let vex = VexPrefix::three_op(reg, vvvv, rm, {bits});");
                    ModRmStyle::RegMem {
                        reg: ModRmReg::Reg(*reg),
                        rm: *rm,
                    }
                }
            },
            [Reg(reg), Reg(rm)] => {
                assert!(!vex.is4);
                fmtln!(f, "let reg = self.{reg}.enc();");
                fmtln!(f, "let vvvv = 0;");
                fmtln!(f, "let rm = (Some(self.{rm}.enc()), None);");
                fmtln!(f, "let vex = VexPrefix::three_op(reg, vvvv, rm, {bits});");
                ModRmStyle::Reg {
                    reg: ModRmReg::Reg(*reg),
                    rm: *rm,
                }
            }
            unknown => unimplemented!("unknown pattern: {unknown:?}"),
        };

        fmtln!(f, "vex.encode(buf);");
        style
    }

    fn generate_modrm_byte(&self, f: &mut Formatter, modrm_style: ModRmStyle) {
        let operands = self.operands_by_kind();
        let bytes_at_end = match operands.as_slice() {
            [.., dsl::OperandKind::Imm(imm)] => imm.bytes(),
            _ => match modrm_style {
                ModRmStyle::RegMemIs4 { .. } => 1,
                _ => 0,
            },
        };

        f.empty_line();

        match modrm_style {
            ModRmStyle::None => f.comment("No need to emit a ModRM byte."),
            _ => f.comment("Emit ModR/M byte."),
        }

        match modrm_style {
            ModRmStyle::None => {}
            ModRmStyle::RegMem { reg, rm } | ModRmStyle::RegMemIs4 { reg, rm, is4: _ } => {
                match reg {
                    ModRmReg::Reg(reg) => fmtln!(f, "let reg = self.{reg}.enc();"),
                    ModRmReg::Digit(digit) => fmtln!(f, "let reg = {digit:#x};"),
                }
                fmtln!(
                    f,
                    "self.{rm}.encode_rex_suffixes(buf, off, reg, {bytes_at_end});"
                );
            }
            ModRmStyle::Reg { reg, rm } => {
                match reg {
                    ModRmReg::Reg(reg) => fmtln!(f, "let reg = self.{reg}.enc();"),
                    ModRmReg::Digit(digit) => fmtln!(f, "let reg = {digit:#x};"),
                }
                fmtln!(f, "self.{rm}.encode_modrm(buf, reg);");
            }
        }
    }

    fn generate_immediate(&self, f: &mut Formatter, modrm_style: ModRmStyle) {
        use dsl::OperandKind::Imm;
        match self.operands_by_kind().as_slice() {
            [prefix @ .., Imm(imm)] => {
                assert!(!prefix.iter().any(|o| matches!(o, Imm(_))));
                f.empty_line();
                f.comment("Emit immediate.");
                fmtln!(f, "self.{imm}.encode(buf);");
            }
            unknown => {
                if let ModRmStyle::RegMemIs4 { is4, .. } = modrm_style {
                    fmtln!(f, "buf.put1(self.{is4}.enc() << 4);");
                }

                // Do nothing: no immediates expected.
                assert!(!unknown.iter().any(|o| matches!(o, Imm(_))));
            }
        }
    }
}

impl dsl::Rex {
    // `buf.put1(...);`
    fn generate_opcodes(&self, f: &mut Formatter, first_op: Option<&dsl::Location>) {
        f.empty_line();
        f.comment("Emit opcode(s).");
        if self.opcodes.escape {
            fmtln!(f, "buf.put1(0x0f);");
        }
        if self.opcode_mod.is_some() {
            let first_op = first_op.expect("Expected first operand for opcode_mod");
            assert!(matches!(first_op.kind(), dsl::OperandKind::Reg(_)));
            fmtln!(f, "let low_bits = self.{first_op}.enc() & 0b111;");
            fmtln!(f, "buf.put1(0x{:x} | low_bits);", self.opcodes.primary);
        } else {
            fmtln!(f, "buf.put1(0x{:x});", self.opcodes.primary);
        }
        if let Some(secondary) = self.opcodes.secondary {
            fmtln!(f, "buf.put1(0x{:x});", secondary);
        }
    }
}

impl dsl::Vex {
    // `buf.put1(...);`
    fn generate_opcode(&self, f: &mut Formatter) {
        f.empty_line();
        f.comment("Emit opcode.");
        fmtln!(f, "buf.put1(0x{:x});", self.opcode);
    }
}
