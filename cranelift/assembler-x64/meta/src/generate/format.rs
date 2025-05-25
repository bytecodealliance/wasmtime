//! Generate format-related Rust code; this also includes generation of encoding
//! Rust code.
use super::{Formatter, fmtln};
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
        self.generate_rex_prefix(f, rex);
        self.generate_opcodes(f, rex);
        self.generate_modrm_byte(f, rex);
        self.generate_immediate(f);
    }

    pub fn generate_vex_encoding(&self, f: &mut Formatter, vex: &dsl::Vex) {
        use dsl::OperandKind::{Reg, RegMem};
        f.empty_line();
        f.comment("Emit New VEX prefix.");

        match self.operands_by_kind().as_slice() {
            [Reg(xmm1), Reg(xmm2), RegMem(xmm_m128)] => {
                fmtln!(
                    f,
                    "vex_instruction::<R>(
                    0x{:0x},
                    VexVectorLength::{},
                    VexPP::{},
                    OpcodeMap::{},
                    self.{}.enc(),
                    Some(self.{}.enc()),
                    Some(self.{}),
                    {}).encode(buf, off);",
                    vex.opcodes.primary,
                    vex.length.to_string(),
                    vex.pp.to_string(),
                    vex.mmmmm.to_string(),
                    xmm1,
                    xmm2,
                    xmm_m128,
                    "None"
                );
            }
            _ => unimplemented!(),
        }
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

    // `buf.put1(...);`
    fn generate_opcodes(&self, f: &mut Formatter, rex: &dsl::Rex) {
        f.empty_line();
        f.comment("Emit opcode(s).");
        if rex.opcodes.escape {
            fmtln!(f, "buf.put1(0x0f);");
        }
        if rex.opcode_mod.is_some() {
            let loc = self.locations().next().unwrap();
            assert!(matches!(loc.kind(), dsl::OperandKind::Reg(_)));
            fmtln!(f, "let low_bits = self.{loc}.enc() & 0b111;");
            fmtln!(f, "buf.put1(0x{:x} | low_bits);", rex.opcodes.primary);
        } else {
            fmtln!(f, "buf.put1(0x{:x});", rex.opcodes.primary);
        }
        if let Some(secondary) = rex.opcodes.secondary {
            fmtln!(f, "buf.put1(0x{:x});", secondary);
        }
    }

    fn generate_rex_prefix(&self, f: &mut Formatter, rex: &dsl::Rex) {
        use dsl::Location::*;
        use dsl::OperandKind::{FixedReg, Imm, Mem, Reg, RegMem};
        f.empty_line();
        f.comment("Possibly emit REX prefix.");

        let find_8bit_registers =
            |l: &dsl::Location| l.bits() == 8 && matches!(l.kind(), Reg(_) | RegMem(_));
        let uses_8bit = self.locations().any(find_8bit_registers);
        fmtln!(f, "let uses_8bit = {uses_8bit};");
        fmtln!(f, "let w_bit = {};", rex.w);
        let bits = "w_bit, uses_8bit";

        match self.operands_by_kind().as_slice() {
            [FixedReg(dst), FixedReg(_)] | [FixedReg(dst)] | [FixedReg(dst), Imm(_)] => {
                // TODO: don't emit REX byte here.
                assert_eq!(rex.digit, None);
                fmtln!(f, "let digit = 0;");
                fmtln!(f, "let dst = self.{dst}.enc();");
                fmtln!(f, "let rex = RexPrefix::with_digit(digit, dst, {bits});");
            }
            [Reg(dst)] => {
                assert_eq!(rex.digit, None);
                assert!(rex.opcode_mod.is_some());
                fmtln!(f, "let dst = self.{dst}.enc();");
                fmtln!(f, "let rex = RexPrefix::one_op(dst, {bits});");
            }
            [Reg(dst), Imm(_)] => {
                let digit = rex.digit.unwrap();
                fmtln!(f, "let digit = 0x{digit:x};");
                fmtln!(f, "let dst = self.{dst}.enc();");
                fmtln!(f, "let rex = RexPrefix::two_op(digit, dst, {bits});");
            }
            [FixedReg(_), RegMem(mem)]
            | [FixedReg(_), FixedReg(_), RegMem(mem)]
            | [RegMem(mem), FixedReg(_)] => {
                let digit = rex.digit.unwrap();
                fmtln!(f, "let digit = 0x{digit:x};");
                fmtln!(f, "let rex = self.{mem}.as_rex_prefix(digit, {bits});");
            }
            [Mem(dst), Imm(_)] | [RegMem(dst), Imm(_)] | [RegMem(dst)] => {
                let digit = rex.digit.unwrap();
                fmtln!(f, "let digit = 0x{digit:x};");
                fmtln!(f, "let rex = self.{dst}.as_rex_prefix(digit, {bits});");
            }
            [Reg(dst), RegMem(src)] | [Reg(dst), RegMem(src), Imm(_)] => {
                fmtln!(f, "let dst = self.{dst}.enc();");
                fmtln!(f, "let rex = self.{src}.as_rex_prefix(dst, {bits});");
            }
            [Mem(dst), Reg(src)] => {
                fmtln!(f, "let src = self.{src}.enc();");
                fmtln!(f, "let rex = self.{dst}.as_rex_prefix(src, {bits});");
            }
            [RegMem(dst), Reg(src)]
            | [RegMem(dst), Reg(src), Imm(_)]
            | [RegMem(dst), Reg(src), FixedReg(_)] => {
                fmtln!(f, "let src = self.{src}.enc();");
                fmtln!(f, "let rex = self.{dst}.as_rex_prefix(src, {bits});");
            }

            [Reg(dst), Reg(xmm2), Imm(_)] | [Reg(dst), Reg(xmm2)] => {
                fmtln!(f, "let reg = self.{dst}.enc();");
                fmtln!(f, "let rm = self.xmm2.enc();");
                fmtln!(f, "let rex = RexPrefix::two_op(reg, rm, {bits});");
            }

            unknown => unimplemented!("unknown pattern: {unknown:?}"),
        }

        fmtln!(f, "rex.encode(buf);");
    }

    fn generate_modrm_byte(&self, f: &mut Formatter, rex: &dsl::Rex) {
        use dsl::Location::*;
        use dsl::OperandKind::{FixedReg, Imm, Mem, Reg, RegMem};

        // Some instructions will never emit a ModR/M byte.
        let operands = self.operands_by_kind();
        if rex.opcode_mod.is_some()
            || matches!(
                operands.as_slice(),
                [FixedReg(_)] | [FixedReg(_), FixedReg(_)] | [FixedReg(_), Imm(_)]
            )
        {
            f.empty_line();
            f.comment("No need to emit a ModRM byte.");
            return;
        }

        // If we must, emit the ModR/M byte and the SIB byte (if necessary).
        f.empty_line();
        f.comment("Emit ModR/M byte.");
        let bytes_at_end = match operands.as_slice() {
            [.., Imm(imm)] => imm.bytes(),
            _ => 0,
        };
        match operands.as_slice() {
            [Reg(reg), Imm(_)] => {
                let digit = rex.digit.unwrap();
                fmtln!(f, "let digit = 0x{digit:x};");
                fmtln!(f, "self.{reg}.encode_modrm(buf, digit);");
            }
            [Mem(mem), Imm(_)]
            | [RegMem(mem), Imm(_)]
            | [RegMem(mem)]
            | [FixedReg(_), RegMem(mem)]
            | [RegMem(mem), FixedReg(_)]
            | [FixedReg(_), FixedReg(_), RegMem(mem)] => {
                let digit = rex.digit.unwrap();
                fmtln!(f, "let digit = 0x{digit:x};");
                fmtln!(
                    f,
                    "self.{mem}.encode_rex_suffixes(buf, off, digit, {bytes_at_end});"
                );
            }
            [Reg(reg), RegMem(mem)]
            | [Reg(reg), RegMem(mem), Imm(_)]
            | [Mem(mem), Reg(reg)]
            | [RegMem(mem), Reg(reg)]
            | [RegMem(mem), Reg(reg), Imm(_)]
            | [RegMem(mem), Reg(reg), FixedReg(_)] => {
                fmtln!(f, "let reg = self.{reg}.enc();");
                fmtln!(
                    f,
                    "self.{mem}.encode_rex_suffixes(buf, off, reg, {bytes_at_end});"
                );
            }

            [Reg(dst), Reg(xmm2), Imm(_)] | [Reg(dst), Reg(xmm2)] => {
                fmtln!(f, "self.xmm2.encode_modrm(buf, self.{dst}.enc());");
            }

            unknown => unimplemented!("unknown pattern: {unknown:?}"),
        }
    }

    fn generate_immediate(&self, f: &mut Formatter) {
        use dsl::OperandKind::Imm;
        match self.operands_by_kind().as_slice() {
            [prefix @ .., Imm(imm)] => {
                assert!(!prefix.iter().any(|o| matches!(o, Imm(_))));

                f.empty_line();
                f.comment("Emit immediate.");
                fmtln!(f, "self.{imm}.encode(buf);");
            }
            unknown => {
                // Do nothing: no immediates expected.
                assert!(!unknown.iter().any(|o| matches!(o, Imm(_))));
            }
        }
    }
}
