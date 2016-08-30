//! RISC-V Instruction Set Architecture.

pub mod settings;
mod encoding;

use super::super::settings as shared_settings;
use isa::encoding as shared_encoding;
use super::Builder as IsaBuilder;
use super::{TargetIsa, Encoding};
use ir::{InstructionData, DataFlowGraph};

#[allow(dead_code)]
struct Isa {
    shared_flags: shared_settings::Flags,
    isa_flags: settings::Flags,
    cpumode: &'static [shared_encoding::Level1Entry<u16>],
}

pub fn isa_builder() -> IsaBuilder {
    IsaBuilder {
        setup: settings::builder(),
        constructor: isa_constructor,
    }
}

fn isa_constructor(shared_flags: shared_settings::Flags,
                   builder: shared_settings::Builder)
                   -> Box<TargetIsa> {
    let level1 = if shared_flags.is_64bit() {
        &encoding::LEVEL1_RV64[..]
    } else {
        &encoding::LEVEL1_RV32[..]
    };
    Box::new(Isa {
        isa_flags: settings::Flags::new(&shared_flags, builder),
        shared_flags: shared_flags,
        cpumode: level1,
    })
}

impl TargetIsa for Isa {
    fn encode(&self, _: &DataFlowGraph, inst: &InstructionData) -> Option<Encoding> {
        shared_encoding::lookup_enclist(inst.first_type(),
                                        inst.opcode(),
                                        self.cpumode,
                                        &encoding::LEVEL2[..])
            .and_then(|enclist_offset| {
                shared_encoding::general_encoding(enclist_offset,
                                                  &encoding::ENCLISTS[..],
                                                  |instp| encoding::check_instp(inst, instp),
                                                  // TODO: Implement ISA predicates properly.
                                                  |isap| isap != 17)
            })
    }

    fn recipe_names(&self) -> &'static [&'static str] {
        &encoding::RECIPE_NAMES[..]
    }
}

#[cfg(test)]
mod tests {
    use settings::{self, Configurable};
    use isa;
    use ir::{DataFlowGraph, InstructionData, Opcode};
    use ir::{types, immediates};

    fn encstr(isa: &isa::TargetIsa, enc: isa::Encoding) -> String {
        format!("{}/{:02x}", isa.recipe_names()[enc.recipe()], enc.bits())
    }

    #[test]
    fn test_64bitenc() {
        let mut shared_builder = settings::builder();
        shared_builder.set_bool("is_64bit", true).unwrap();
        let shared_flags = settings::Flags::new(shared_builder);
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
        assert_eq!(encstr(&*isa, isa.encode(&dfg, &inst64).unwrap()), "I/04");

        // Try to encode iadd_imm.i64 vx1, -10000.
        let inst64_large = InstructionData::BinaryImm {
            opcode: Opcode::IaddImm,
            ty: types::I64,
            arg: arg64,
            imm: immediates::Imm64::new(-10000),
        };

        // Immediate is out of range for ADDI.
        assert_eq!(isa.encode(&dfg, &inst64_large), None);

        // Create an iadd_imm.i32 which is encodable in RV64.
        let inst32 = InstructionData::BinaryImm {
            opcode: Opcode::IaddImm,
            ty: types::I32,
            arg: arg32,
            imm: immediates::Imm64::new(10),
        };

        // ADDIW is I/0b00110
        assert_eq!(encstr(&*isa, isa.encode(&dfg, &inst32).unwrap()), "I/06");
    }

    // Same as above, but for RV32.
    #[test]
    fn test_32bitenc() {
        let mut shared_builder = settings::builder();
        shared_builder.set_bool("is_64bit", false).unwrap();
        let shared_flags = settings::Flags::new(shared_builder);
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
        assert_eq!(isa.encode(&dfg, &inst64), None);

        // Try to encode iadd_imm.i64 vx1, -10000.
        let inst64_large = InstructionData::BinaryImm {
            opcode: Opcode::IaddImm,
            ty: types::I64,
            arg: arg64,
            imm: immediates::Imm64::new(-10000),
        };

        // Immediate is out of range for ADDI.
        assert_eq!(isa.encode(&dfg, &inst64_large), None);

        // Create an iadd_imm.i32 which is encodable in RV32.
        let inst32 = InstructionData::BinaryImm {
            opcode: Opcode::IaddImm,
            ty: types::I32,
            arg: arg32,
            imm: immediates::Imm64::new(10),
        };

        // ADDI is I/0b00100
        assert_eq!(encstr(&*isa, isa.encode(&dfg, &inst32).unwrap()), "I/04");
    }
}
