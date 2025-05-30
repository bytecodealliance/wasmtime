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
        rex.generate_opcodes(f, self.locations().next());
        self.generate_modrm_byte(f, rex.opcode_mod, rex.modrm);
        self.generate_immediate(f);
    }

    pub fn generate_vex_encoding(&self, f: &mut Formatter, vex: &dsl::Vex) {
        self.generate_vex_prefix(f, vex);
        vex.generate_opcode(f);
        self.generate_modrm_byte(f, None, vex.modrm);
        self.generate_immediate(f);
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

    fn generate_rex_prefix(&self, f: &mut Formatter, rex: &dsl::Rex) {
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
                assert_eq!(rex.unwrap_digit(), None);
                fmtln!(f, "let digit = 0;");
                fmtln!(f, "let dst = self.{dst}.enc();");
                fmtln!(f, "let rex = RexPrefix::with_digit(digit, dst, {bits});");
            }
            [Reg(dst)] => {
                assert_eq!(rex.unwrap_digit(), None);
                assert!(rex.opcode_mod.is_some());
                fmtln!(f, "let dst = self.{dst}.enc();");
                fmtln!(f, "let rex = RexPrefix::one_op(dst, {bits});");
            }
            [Reg(dst), Imm(_)] => match rex.unwrap_digit() {
                Some(digit) => {
                    fmtln!(f, "let digit = 0x{digit:x};");
                    fmtln!(f, "let dst = self.{dst}.enc();");
                    fmtln!(f, "let rex = RexPrefix::two_op(digit, dst, {bits});");
                }
                None => {
                    assert!(rex.opcode_mod.is_some());
                    fmtln!(f, "let dst = self.{dst}.enc();");
                    fmtln!(f, "let rex = RexPrefix::one_op(dst, {bits});");
                }
            },
            [FixedReg(_), RegMem(mem)]
            | [FixedReg(_), FixedReg(_), RegMem(mem)]
            | [RegMem(mem), FixedReg(_)] => {
                let digit = rex.unwrap_digit().unwrap();
                fmtln!(f, "let digit = 0x{digit:x};");
                fmtln!(f, "let rex = self.{mem}.as_rex_prefix(digit, {bits});");
            }
            [Mem(dst), Imm(_)] | [RegMem(dst), Imm(_)] | [RegMem(dst)] => {
                let digit = rex.unwrap_digit().unwrap();
                fmtln!(f, "let digit = 0x{digit:x};");
                fmtln!(f, "let rex = self.{dst}.as_rex_prefix(digit, {bits});");
            }
            [Reg(dst), RegMem(src)] | [Reg(dst), RegMem(src), Imm(_)] | [Reg(dst), Mem(src)] => {
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
                fmtln!(f, "let rm = self.{xmm2}.enc();");
                fmtln!(f, "let rex = RexPrefix::two_op(reg, rm, {bits});");
            }
            unknown => unimplemented!("unknown pattern: {unknown:?}"),
        }

        fmtln!(f, "rex.encode(buf);");
    }

    fn generate_vex_prefix(&self, f: &mut Formatter, vex: &dsl::Vex) {
        use dsl::OperandKind::{Imm, Reg, RegMem};
        f.empty_line();
        f.comment("Emit VEX prefix.");
        fmtln!(f, "let len = {:#03b};", vex.length.bits());
        fmtln!(f, "let pp = {:#04b};", vex.pp.map_or(0b00, |pp| pp.bits()));
        fmtln!(f, "let mmmmm = {:#07b};", vex.mmmmm.unwrap().bits());
        fmtln!(f, "let w = {};", vex.w.as_bool());
        let bits = "len, pp, mmmmm, w";

        match self.operands_by_kind().as_slice() {
            [Reg(reg), Reg(vvvv), RegMem(rm)] => {
                fmtln!(f, "let reg = self.{reg}.enc();");
                fmtln!(f, "let vvvv = self.{vvvv}.enc();");
                fmtln!(f, "let rm = self.{rm}.encode_bx_regs();");
                fmtln!(f, "let vex = VexPrefix::three_op(reg, vvvv, rm, {bits});");
            }
            [Reg(reg), RegMem(rm), Imm(_imm8)] => {
                fmtln!(f, "let reg = self.{reg}.enc();");
                fmtln!(f, "let vvvv = {};", "0b0");
                fmtln!(f, "let rm = self.{rm}.encode_bx_regs();");
                fmtln!(f, "let vex = VexPrefix::three_op(reg, vvvv, rm, {bits});");
            }
            [Reg(reg), Reg(vvvv), RegMem(rm), Imm(_imm8)] => {
                fmtln!(f, "let reg = self.{reg}.enc();");
                fmtln!(f, "let vvvv = self.{vvvv}.enc();");
                fmtln!(f, "let rm = self.{rm}.encode_bx_regs();");
                fmtln!(f, "let vex = VexPrefix::three_op(reg, vvvv, rm, {bits});");
            }
            unknown => unimplemented!("unknown pattern: {unknown:?}"),
        }

        fmtln!(f, "vex.encode(buf);");
    }

    fn generate_modrm_byte(
        &self,
        f: &mut Formatter,
        opcode_mod: Option<dsl::OpcodeMod>,
        digit: Option<dsl::ModRmKind>,
    ) {
        use dsl::OperandKind::{FixedReg, Imm, Mem, Reg, RegMem};

        // Some instructions will never emit a ModR/M byte.
        let operands = self.operands_by_kind();
        if opcode_mod.is_some()
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
                let digit = digit.unwrap().unwrap_digit();
                fmtln!(f, "let digit = 0x{digit:x};");
                fmtln!(f, "self.{reg}.encode_modrm(buf, digit);");
            }
            [Mem(mem), Imm(_)]
            | [RegMem(mem), Imm(_)]
            | [RegMem(mem)]
            | [FixedReg(_), RegMem(mem)]
            | [RegMem(mem), FixedReg(_)]
            | [FixedReg(_), FixedReg(_), RegMem(mem)] => {
                let digit = digit.unwrap().unwrap_digit();
                fmtln!(f, "let digit = 0x{digit:x};");
                fmtln!(
                    f,
                    "self.{mem}.encode_rex_suffixes(buf, off, digit, {bytes_at_end});"
                );
            }
            [Reg(reg), RegMem(mem)]
            | [Reg(reg), RegMem(mem), Imm(_)]
            | [Mem(mem), Reg(reg)]
            | [Reg(reg), Reg(_), RegMem(mem)]
            | [RegMem(mem), Reg(reg)]
            | [RegMem(mem), Reg(reg), Imm(_)]
            | [RegMem(mem), Reg(reg), FixedReg(_)]
            | [Reg(reg), Reg(_), RegMem(mem), Imm(_)]
            | [Reg(reg), Mem(mem)] => {
                fmtln!(f, "let reg = self.{reg}.enc();");
                fmtln!(
                    f,
                    "self.{mem}.encode_rex_suffixes(buf, off, reg, {bytes_at_end});"
                );
            }
            [Reg(dst), Reg(xmm2), Imm(_)] | [Reg(dst), Reg(xmm2)] => {
                fmtln!(f, "self.{xmm2}.encode_modrm(buf, self.{dst}.enc());");
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
