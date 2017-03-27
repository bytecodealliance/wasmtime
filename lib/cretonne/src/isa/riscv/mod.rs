//! RISC-V Instruction Set Architecture.

pub mod settings;
mod abi;
mod binemit;
mod enc_tables;
mod registers;

use super::super::settings as shared_settings;
use binemit::CodeSink;
use isa::enc_tables::{self as shared_enc_tables, lookup_enclist, general_encoding};
use isa::Builder as IsaBuilder;
use isa::{TargetIsa, RegInfo, Encoding, Legalize, RecipeConstraints};
use ir::{Function, Inst, InstructionData, DataFlowGraph, Signature};

#[allow(dead_code)]
struct Isa {
    shared_flags: shared_settings::Flags,
    isa_flags: settings::Flags,
    cpumode: &'static [shared_enc_tables::Level1Entry<u16>],
}

/// Get an ISA builder for creating RISC-V targets.
pub fn isa_builder() -> IsaBuilder {
    IsaBuilder {
        setup: settings::builder(),
        constructor: isa_constructor,
    }
}

fn isa_constructor(shared_flags: shared_settings::Flags,
                   builder: &shared_settings::Builder)
                   -> Box<TargetIsa> {
    let level1 = if shared_flags.is_64bit() {
        &enc_tables::LEVEL1_RV64[..]
    } else {
        &enc_tables::LEVEL1_RV32[..]
    };
    Box::new(Isa {
                 isa_flags: settings::Flags::new(&shared_flags, builder),
                 shared_flags: shared_flags,
                 cpumode: level1,
             })
}

impl TargetIsa for Isa {
    fn name(&self) -> &'static str {
        "riscv"
    }

    fn flags(&self) -> &shared_settings::Flags {
        &self.shared_flags
    }

    fn register_info(&self) -> RegInfo {
        registers::INFO.clone()
    }

    fn encode(&self, dfg: &DataFlowGraph, inst: &InstructionData) -> Result<Encoding, Legalize> {
        lookup_enclist(inst.ctrl_typevar(dfg),
                       inst.opcode(),
                       self.cpumode,
                       &enc_tables::LEVEL2[..])
                .and_then(|enclist_offset| {
                    general_encoding(enclist_offset,
                                     &enc_tables::ENCLISTS[..],
                                     |instp| enc_tables::check_instp(inst, instp),
                                     |isap| self.isa_flags.numbered_predicate(isap as usize))
                            .ok_or(Legalize::Expand)
                })
    }

    fn recipe_names(&self) -> &'static [&'static str] {
        &enc_tables::RECIPE_NAMES[..]
    }

    fn recipe_constraints(&self) -> &'static [RecipeConstraints] {
        &enc_tables::RECIPE_CONSTRAINTS
    }

    fn legalize_signature(&self, sig: &mut Signature) {
        // We can pass in `self.isa_flags` too, if we need it.
        abi::legalize_signature(sig, &self.shared_flags)
    }

    fn emit_inst(&self, func: &Function, inst: Inst, sink: &mut CodeSink) {
        binemit::emit_inst(func, inst, sink)
    }
}

#[cfg(test)]
mod tests {
    use settings::{self, Configurable};
    use isa;
    use ir::{DataFlowGraph, InstructionData, Opcode};
    use ir::{types, immediates};

    fn encstr(isa: &isa::TargetIsa, enc: isa::Encoding) -> String {
        isa.display_enc(enc).to_string()
    }

    #[test]
    fn test_64bitenc() {
        let mut shared_builder = settings::builder();
        shared_builder.set_bool("is_64bit", true).unwrap();
        let shared_flags = settings::Flags::new(&shared_builder);
        let isa = isa::lookup("riscv").unwrap().finish(shared_flags);

        let mut dfg = DataFlowGraph::new();
        let ebb = dfg.make_ebb();
        let arg64 = dfg.append_ebb_arg(ebb, types::I64);
        let arg32 = dfg.append_ebb_arg(ebb, types::I32);

        // Try to encode iadd_imm.i64 vx1, -10.
        let inst64 = InstructionData::BinaryImm {
            opcode: Opcode::IaddImm,
            ty: types::I64,
            arg: arg64,
            imm: immediates::Imm64::new(-10),
        };

        // ADDI is I/0b00100
        assert_eq!(encstr(&*isa, isa.encode(&dfg, &inst64).unwrap()), "I#04");

        // Try to encode iadd_imm.i64 vx1, -10000.
        let inst64_large = InstructionData::BinaryImm {
            opcode: Opcode::IaddImm,
            ty: types::I64,
            arg: arg64,
            imm: immediates::Imm64::new(-10000),
        };

        // Immediate is out of range for ADDI.
        assert_eq!(isa.encode(&dfg, &inst64_large), Err(isa::Legalize::Expand));

        // Create an iadd_imm.i32 which is encodable in RV64.
        let inst32 = InstructionData::BinaryImm {
            opcode: Opcode::IaddImm,
            ty: types::I32,
            arg: arg32,
            imm: immediates::Imm64::new(10),
        };

        // ADDIW is I/0b00110
        assert_eq!(encstr(&*isa, isa.encode(&dfg, &inst32).unwrap()), "I#06");
    }

    // Same as above, but for RV32.
    #[test]
    fn test_32bitenc() {
        let mut shared_builder = settings::builder();
        shared_builder.set_bool("is_64bit", false).unwrap();
        let shared_flags = settings::Flags::new(&shared_builder);
        let isa = isa::lookup("riscv").unwrap().finish(shared_flags);

        let mut dfg = DataFlowGraph::new();
        let ebb = dfg.make_ebb();
        let arg64 = dfg.append_ebb_arg(ebb, types::I64);
        let arg32 = dfg.append_ebb_arg(ebb, types::I32);

        // Try to encode iadd_imm.i64 vx1, -10.
        let inst64 = InstructionData::BinaryImm {
            opcode: Opcode::IaddImm,
            ty: types::I64,
            arg: arg64,
            imm: immediates::Imm64::new(-10),
        };

        // In 32-bit mode, an i64 bit add should be narrowed.
        assert_eq!(isa.encode(&dfg, &inst64), Err(isa::Legalize::Narrow));

        // Try to encode iadd_imm.i64 vx1, -10000.
        let inst64_large = InstructionData::BinaryImm {
            opcode: Opcode::IaddImm,
            ty: types::I64,
            arg: arg64,
            imm: immediates::Imm64::new(-10000),
        };

        // In 32-bit mode, an i64 bit add should be narrowed.
        assert_eq!(isa.encode(&dfg, &inst64_large), Err(isa::Legalize::Narrow));

        // Create an iadd_imm.i32 which is encodable in RV32.
        let inst32 = InstructionData::BinaryImm {
            opcode: Opcode::IaddImm,
            ty: types::I32,
            arg: arg32,
            imm: immediates::Imm64::new(10),
        };

        // ADDI is I/0b00100
        assert_eq!(encstr(&*isa, isa.encode(&dfg, &inst32).unwrap()), "I#04");

        // Create an imul.i32 which is encodable in RV32, but only when use_m is true.
        let mul32 = InstructionData::Binary {
            opcode: Opcode::Imul,
            ty: types::I32,
            args: [arg32, arg32],
        };

        assert_eq!(isa.encode(&dfg, &mul32), Err(isa::Legalize::Expand));
    }

    #[test]
    fn test_rv32m() {
        let mut shared_builder = settings::builder();
        shared_builder.set_bool("is_64bit", false).unwrap();
        let shared_flags = settings::Flags::new(&shared_builder);

        // Set the supports_m stting which in turn enables the use_m predicate that unlocks
        // encodings for imul.
        let mut isa_builder = isa::lookup("riscv").unwrap();
        isa_builder.set_bool("supports_m", true).unwrap();

        let isa = isa_builder.finish(shared_flags);

        let mut dfg = DataFlowGraph::new();
        let ebb = dfg.make_ebb();
        let arg32 = dfg.append_ebb_arg(ebb, types::I32);

        // Create an imul.i32 which is encodable in RV32M.
        let mul32 = InstructionData::Binary {
            opcode: Opcode::Imul,
            ty: types::I32,
            args: [arg32, arg32],
        };
        assert_eq!(encstr(&*isa, isa.encode(&dfg, &mul32).unwrap()), "R#10c");
    }
}
