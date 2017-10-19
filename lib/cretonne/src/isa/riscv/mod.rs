//! RISC-V Instruction Set Architecture.

pub mod settings;
mod abi;
mod binemit;
mod enc_tables;
mod registers;

use super::super::settings as shared_settings;
use binemit::{CodeSink, MemoryCodeSink, emit_function};
use isa::enc_tables::{self as shared_enc_tables, lookup_enclist, Encodings};
use isa::Builder as IsaBuilder;
use isa::{TargetIsa, RegInfo, RegClass, EncInfo};
use ir;
use regalloc;

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

fn isa_constructor(
    shared_flags: shared_settings::Flags,
    builder: &shared_settings::Builder,
) -> Box<TargetIsa> {
    let level1 = if shared_flags.is_64bit() {
        &enc_tables::LEVEL1_RV64[..]
    } else {
        &enc_tables::LEVEL1_RV32[..]
    };
    Box::new(Isa {
        isa_flags: settings::Flags::new(&shared_flags, builder),
        shared_flags,
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

    fn encoding_info(&self) -> EncInfo {
        enc_tables::INFO.clone()
    }

    fn legal_encodings<'a>(
        &'a self,
        dfg: &'a ir::DataFlowGraph,
        inst: &'a ir::InstructionData,
        ctrl_typevar: ir::Type,
    ) -> Encodings<'a> {
        lookup_enclist(
            ctrl_typevar,
            inst,
            dfg,
            self.cpumode,
            &enc_tables::LEVEL2[..],
            &enc_tables::ENCLISTS[..],
            &enc_tables::LEGALIZE_ACTIONS[..],
            &enc_tables::RECIPE_PREDICATES[..],
            &enc_tables::INST_PREDICATES[..],
            self.isa_flags.predicate_view(),
        )
    }

    fn legalize_signature(&self, sig: &mut ir::Signature, current: bool) {
        abi::legalize_signature(sig, &self.shared_flags, &self.isa_flags, current)
    }

    fn regclass_for_abi_type(&self, ty: ir::Type) -> RegClass {
        abi::regclass_for_abi_type(ty)
    }

    fn allocatable_registers(&self, func: &ir::Function) -> regalloc::AllocatableSet {
        abi::allocatable_registers(func, &self.isa_flags)
    }

    fn emit_inst(
        &self,
        func: &ir::Function,
        inst: ir::Inst,
        divert: &mut regalloc::RegDiversions,
        sink: &mut CodeSink,
    ) {
        binemit::emit_inst(func, inst, divert, sink)
    }

    fn emit_function(&self, func: &ir::Function, sink: &mut MemoryCodeSink) {
        emit_function(func, binemit::emit_inst, sink)
    }

    fn reloc_names(&self) -> &'static [&'static str] {
        &binemit::RELOC_NAMES
    }
}

#[cfg(test)]
mod tests {
    use settings::{self, Configurable};
    use isa;
    use ir::{DataFlowGraph, InstructionData, Opcode};
    use ir::{types, immediates};

    fn encstr(isa: &isa::TargetIsa, enc: Result<isa::Encoding, isa::Legalize>) -> String {
        match enc {
            Ok(e) => isa.encoding_info().display(e).to_string(),
            Err(_) => "no encoding".to_string(),
        }
    }

    #[test]
    fn test_64bitenc() {
        let mut shared_builder = settings::builder();
        shared_builder.enable("is_64bit").unwrap();
        let shared_flags = settings::Flags::new(&shared_builder);
        let isa = isa::lookup("riscv").unwrap().finish(shared_flags);

        let mut dfg = DataFlowGraph::new();
        let ebb = dfg.make_ebb();
        let arg64 = dfg.append_ebb_param(ebb, types::I64);
        let arg32 = dfg.append_ebb_param(ebb, types::I32);

        // Try to encode iadd_imm.i64 v1, -10.
        let inst64 = InstructionData::BinaryImm {
            opcode: Opcode::IaddImm,
            arg: arg64,
            imm: immediates::Imm64::new(-10),
        };

        // ADDI is I/0b00100
        assert_eq!(encstr(&*isa, isa.encode(&dfg, &inst64, types::I64)), "I#04");

        // Try to encode iadd_imm.i64 v1, -10000.
        let inst64_large = InstructionData::BinaryImm {
            opcode: Opcode::IaddImm,
            arg: arg64,
            imm: immediates::Imm64::new(-10000),
        };

        // Immediate is out of range for ADDI.
        assert!(isa.encode(&dfg, &inst64_large, types::I64).is_err());

        // Create an iadd_imm.i32 which is encodable in RV64.
        let inst32 = InstructionData::BinaryImm {
            opcode: Opcode::IaddImm,
            arg: arg32,
            imm: immediates::Imm64::new(10),
        };

        // ADDIW is I/0b00110
        assert_eq!(encstr(&*isa, isa.encode(&dfg, &inst32, types::I32)), "I#06");
    }

    // Same as above, but for RV32.
    #[test]
    fn test_32bitenc() {
        let mut shared_builder = settings::builder();
        shared_builder.set("is_64bit", "false").unwrap();
        let shared_flags = settings::Flags::new(&shared_builder);
        let isa = isa::lookup("riscv").unwrap().finish(shared_flags);

        let mut dfg = DataFlowGraph::new();
        let ebb = dfg.make_ebb();
        let arg64 = dfg.append_ebb_param(ebb, types::I64);
        let arg32 = dfg.append_ebb_param(ebb, types::I32);

        // Try to encode iadd_imm.i64 v1, -10.
        let inst64 = InstructionData::BinaryImm {
            opcode: Opcode::IaddImm,
            arg: arg64,
            imm: immediates::Imm64::new(-10),
        };

        // In 32-bit mode, an i64 bit add should be narrowed.
        assert!(isa.encode(&dfg, &inst64, types::I64).is_err());

        // Try to encode iadd_imm.i64 v1, -10000.
        let inst64_large = InstructionData::BinaryImm {
            opcode: Opcode::IaddImm,
            arg: arg64,
            imm: immediates::Imm64::new(-10000),
        };

        // In 32-bit mode, an i64 bit add should be narrowed.
        assert!(isa.encode(&dfg, &inst64_large, types::I64).is_err());

        // Create an iadd_imm.i32 which is encodable in RV32.
        let inst32 = InstructionData::BinaryImm {
            opcode: Opcode::IaddImm,
            arg: arg32,
            imm: immediates::Imm64::new(10),
        };

        // ADDI is I/0b00100
        assert_eq!(encstr(&*isa, isa.encode(&dfg, &inst32, types::I32)), "I#04");

        // Create an imul.i32 which is encodable in RV32, but only when use_m is true.
        let mul32 = InstructionData::Binary {
            opcode: Opcode::Imul,
            args: [arg32, arg32],
        };

        assert!(isa.encode(&dfg, &mul32, types::I32).is_err());
    }

    #[test]
    fn test_rv32m() {
        let mut shared_builder = settings::builder();
        shared_builder.set("is_64bit", "false").unwrap();
        let shared_flags = settings::Flags::new(&shared_builder);

        // Set the supports_m stting which in turn enables the use_m predicate that unlocks
        // encodings for imul.
        let mut isa_builder = isa::lookup("riscv").unwrap();
        isa_builder.enable("supports_m").unwrap();

        let isa = isa_builder.finish(shared_flags);

        let mut dfg = DataFlowGraph::new();
        let ebb = dfg.make_ebb();
        let arg32 = dfg.append_ebb_param(ebb, types::I32);

        // Create an imul.i32 which is encodable in RV32M.
        let mul32 = InstructionData::Binary {
            opcode: Opcode::Imul,
            args: [arg32, arg32],
        };
        assert_eq!(encstr(&*isa, isa.encode(&dfg, &mul32, types::I32)), "R#10c");
    }
}
