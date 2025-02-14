//! Generate format-related Rust code; this also includes generation of encoding
//! Rust code.

use super::{fmtln, Formatter};
use crate::dsl;

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
    pub fn generate_att_style_operands(&self) -> String {
        let mut ordered_ops: Vec<_> = self
            .operands
            .iter()
            .map(|o| format!("{{{}}}", o.location))
            .collect();
        if ordered_ops.len() > 1 {
            let first = ordered_ops.remove(0);
            ordered_ops.push(first);
        }
        ordered_ops.join(", ")
    }

    pub fn generate_rex_encoding(&self, f: &mut Formatter, rex: &dsl::Rex) {
        self.generate_legacy_prefix(f, rex);
        self.generate_rex_prefix(f, rex);
        self.generate_opcode(f, rex);
        self.generate_modrm_byte(f, rex);
        self.generate_immediate(f);
    }

    /// `buf.put1(...);`
    fn generate_legacy_prefix(&self, f: &mut Formatter, rex: &dsl::Rex) {
        use dsl::LegacyPrefix::*;
        if rex.prefix != NoPrefix {
            f.empty_line();
            f.comment("Emit legacy prefixes.");
            match rex.prefix {
                NoPrefix => unreachable!(),
                _66 => fmtln!(f, "buf.put1(0x66);"),
                _F0 => fmtln!(f, "buf.put1(0xf0);"),
                _66F0 => {
                    fmtln!(f, "buf.put1(0x66);");
                    fmtln!(f, "buf.put1(0xf0);");
                }
                _F2 => fmtln!(f, "buf.put1(0xf2);"),
                _F3 => fmtln!(f, "buf.put1(0xf3);"),
                _66F3 => {
                    fmtln!(f, "buf.put1(0x66);");
                    fmtln!(f, "buf.put1(0xf3);");
                }
            }
        }
    }

    // `buf.put1(...);`
    fn generate_opcode(&self, f: &mut Formatter, rex: &dsl::Rex) {
        f.empty_line();
        f.comment("Emit opcode.");
        fmtln!(f, "buf.put1(0x{:x});", rex.opcode);
    }

    fn generate_rex_prefix(&self, f: &mut Formatter, rex: &dsl::Rex) {
        use dsl::OperandKind::{FixedReg, Imm, Reg, RegMem};
        f.empty_line();
        f.comment("Emit REX prefix.");

        let find_8bit_registers = |l: &dsl::Location| l.bits() == 8 && matches!(l.kind(), Reg(_) | RegMem(_));
        if self.locations().any(find_8bit_registers) {
            fmtln!(f, "let mut rex = {};", rex.generate_flags());
            for op in self.locations().copied().filter(find_8bit_registers) {
                fmtln!(f, "self.{op}.always_emit_if_8bit_needed(&mut rex);");
            }
        } else {
            fmtln!(f, "let rex = {};", rex.generate_flags());
        }

        match self.operands_by_kind().as_slice() {
            [FixedReg(dst), Imm(_)] => {
                // TODO: don't emit REX byte here.
                fmtln!(f, "let {dst} = {};", dst.generate_fixed_reg().unwrap());
                fmtln!(f, "let digit = 0x{:x};", rex.digit);
                fmtln!(f, "rex.emit_two_op(buf, digit, {dst}.enc());");
            }
            [RegMem(dst), Imm(_)] => {
                if rex.digit > 0 {
                    fmtln!(f, "let digit = 0x{:x};", rex.digit);
                    fmtln!(f, "match &self.{dst} {{");
                    f.indent(|f| {
                        fmtln!(f, "GprMem::Gpr({dst}) => rex.emit_two_op(buf, digit, {dst}.enc()),");
                        fmtln!(f, "GprMem::Mem({dst}) => {dst}.emit_rex_prefix(rex, digit, buf),");
                    });
                    fmtln!(f, "}}");
                } else {
                    unimplemented!();
                }
            }
            [Reg(dst), RegMem(src)] => {
                fmtln!(f, "let {dst} = self.{dst}.enc();");
                fmtln!(f, "match &self.{src} {{");
                f.indent(|f| {
                    fmtln!(f, "GprMem::Gpr({src}) => rex.emit_two_op(buf, {dst}, {src}.enc()),");
                    fmtln!(f, "GprMem::Mem({src}) => {src}.emit_rex_prefix(rex, {dst}, buf),");
                });
                fmtln!(f, "}}");
            }
            [RegMem(dst), Reg(src)] => {
                fmtln!(f, "let {src} = self.{src}.enc();");
                fmtln!(f, "match &self.{dst} {{");
                f.indent(|f| {
                    fmtln!(f, "GprMem::Gpr({dst}) => rex.emit_two_op(buf, {src}, {dst}.enc()),");
                    fmtln!(f, "GprMem::Mem({dst}) => {dst}.emit_rex_prefix(rex, {src}, buf),");
                });
                fmtln!(f, "}}");
            }

            unknown => unimplemented!("unknown pattern: {unknown:?}"),
        }
    }

    fn generate_modrm_byte(&self, f: &mut Formatter, rex: &dsl::Rex) {
        use dsl::OperandKind::{FixedReg, Imm, Reg, RegMem};

        if let [FixedReg(_), Imm(_)] = self.operands_by_kind().as_slice() {
            // No need to emit a comment.
        } else {
            f.empty_line();
            f.comment("Emit ModR/M byte.");
        }

        match self.operands_by_kind().as_slice() {
            [FixedReg(_), Imm(_)] => {
                // No need to emit a ModRM byte: we know the register used.
            }
            [RegMem(dst), Imm(_)] => {
                debug_assert!(rex.digit > 0);
                fmtln!(f, "let digit = 0x{:x};", rex.digit);
                fmtln!(f, "match &self.{dst} {{");
                f.indent(|f| {
                    fmtln!(f, "GprMem::Gpr({dst}) => emit_modrm(buf, digit, {dst}.enc()),");
                    fmtln!(f, "GprMem::Mem({dst}) => emit_modrm_sib_disp(buf, off, digit, {dst}, 0, None),");
                });
                fmtln!(f, "}}");
            }
            [Reg(dst), RegMem(src)] => {
                fmtln!(f, "let {dst} = self.{dst}.enc();");
                fmtln!(f, "match &self.{src} {{");
                f.indent(|f| {
                    fmtln!(f, "GprMem::Gpr({src}) => emit_modrm(buf, {dst}, {src}.enc()),");
                    fmtln!(f, "GprMem::Mem({src}) => emit_modrm_sib_disp(buf, off, {dst}, {src}, 0, None),");
                });
                fmtln!(f, "}}");
            }
            [RegMem(dst), Reg(src)] => {
                fmtln!(f, "let {src} = self.{src}.enc();");
                fmtln!(f, "match &self.{dst} {{");
                f.indent(|f| {
                    fmtln!(f, "GprMem::Gpr({dst}) => emit_modrm(buf, {src}, {dst}.enc()),");
                    fmtln!(f, "GprMem::Mem({dst}) => emit_modrm_sib_disp(buf, off, {src}, {dst}, 0, None),");
                });
                fmtln!(f, "}}");
            }

            unknown => unimplemented!("unknown pattern: {unknown:?}"),
        }
    }

    fn generate_immediate(&self, f: &mut Formatter) {
        use dsl::OperandKind::Imm;
        match self.operands_by_kind().as_slice() {
            [_, Imm(imm)] => {
                f.empty_line();
                f.comment("Emit immediate.");
                fmtln!(f, "self.{imm}.encode(buf);");
            }
            unknown => {
                // Do nothing: no immediates expected.
                debug_assert!(!unknown.iter().any(|o| matches!(o, Imm(_))));
            }
        }
    }
}

impl dsl::Rex {
    fn generate_flags(&self) -> &str {
        if self.w {
            "RexFlags::set_w()"
        } else {
            "RexFlags::clear_w()"
        }
    }
}
