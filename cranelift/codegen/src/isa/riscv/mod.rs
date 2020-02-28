//! RISC-V Instruction Set Architecture.

mod abi;
mod binemit;
mod enc_tables;
mod registers;
pub mod settings;

use super::super::settings as shared_settings;
#[cfg(feature = "testing_hooks")]
use crate::binemit::CodeSink;
use crate::binemit::{emit_function, MemoryCodeSink};
use crate::ir;
use crate::isa::enc_tables::{self as shared_enc_tables, lookup_enclist, Encodings};
use crate::isa::Builder as IsaBuilder;
use crate::isa::{EncInfo, RegClass, RegInfo, TargetIsa};
use crate::regalloc;
use alloc::borrow::Cow;
use alloc::boxed::Box;
use core::fmt;
use target_lexicon::{PointerWidth, Triple};

#[allow(dead_code)]
struct Isa {
    triple: Triple,
    shared_flags: shared_settings::Flags,
    isa_flags: settings::Flags,
    cpumode: &'static [shared_enc_tables::Level1Entry<u16>],
}

/// Get an ISA builder for creating RISC-V targets.
pub fn isa_builder(triple: Triple) -> IsaBuilder {
    IsaBuilder {
        triple,
        setup: settings::builder(),
        constructor: isa_constructor,
    }
}

fn isa_constructor(
    triple: Triple,
    shared_flags: shared_settings::Flags,
    builder: shared_settings::Builder,
) -> Box<dyn TargetIsa> {
    let level1 = match triple.pointer_width().unwrap() {
        PointerWidth::U16 => panic!("16-bit RISC-V unrecognized"),
        PointerWidth::U32 => &enc_tables::LEVEL1_RV32[..],
        PointerWidth::U64 => &enc_tables::LEVEL1_RV64[..],
    };
    Box::new(Isa {
        triple,
        isa_flags: settings::Flags::new(&shared_flags, builder),
        shared_flags,
        cpumode: level1,
    })
}

impl TargetIsa for Isa {
    fn name(&self) -> &'static str {
        "riscv"
    }

    fn triple(&self) -> &Triple {
        &self.triple
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
        func: &'a ir::Function,
        inst: &'a ir::InstructionData,
        ctrl_typevar: ir::Type,
    ) -> Encodings<'a> {
        lookup_enclist(
            ctrl_typevar,
            inst,
            func,
            self.cpumode,
            &enc_tables::LEVEL2[..],
            &enc_tables::ENCLISTS[..],
            &enc_tables::LEGALIZE_ACTIONS[..],
            &enc_tables::RECIPE_PREDICATES[..],
            &enc_tables::INST_PREDICATES[..],
            self.isa_flags.predicate_view(),
        )
    }

    fn legalize_signature(&self, sig: &mut Cow<ir::Signature>, current: bool) {
        abi::legalize_signature(sig, &self.triple, &self.isa_flags, current)
    }

    fn regclass_for_abi_type(&self, ty: ir::Type) -> RegClass {
        abi::regclass_for_abi_type(ty)
    }

    fn allocatable_registers(&self, func: &ir::Function) -> regalloc::RegisterSet {
        abi::allocatable_registers(func, &self.isa_flags)
    }

    #[cfg(feature = "testing_hooks")]
    fn emit_inst(
        &self,
        func: &ir::Function,
        inst: ir::Inst,
        divert: &mut regalloc::RegDiversions,
        sink: &mut dyn CodeSink,
    ) {
        binemit::emit_inst(func, inst, divert, sink, self)
    }

    fn emit_function_to_memory(&self, func: &ir::Function, sink: &mut MemoryCodeSink) {
        emit_function(func, binemit::emit_inst, sink, self)
    }

    fn unsigned_add_overflow_condition(&self) -> ir::condcodes::IntCC {
        unimplemented!()
    }

    fn unsigned_sub_overflow_condition(&self) -> ir::condcodes::IntCC {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use crate::ir::{immediates, types};
    use crate::ir::{Function, InstructionData, Opcode};
    use crate::isa;
    use crate::settings::{self, Configurable};
    use alloc::string::{String, ToString};
    use core::str::FromStr;
    use target_lexicon::triple;

    fn encstr(isa: &dyn isa::TargetIsa, enc: Result<isa::Encoding, isa::Legalize>) -> String {
        match enc {
            Ok(e) => isa.encoding_info().display(e).to_string(),
            Err(_) => "no encoding".to_string(),
        }
    }

    #[test]
    fn test_64bitenc() {
        let shared_builder = settings::builder();
        let shared_flags = settings::Flags::new(shared_builder);
        let isa = isa::lookup(triple!("riscv64"))
            .unwrap()
            .finish(shared_flags);

        let mut func = Function::new();
        let block = func.dfg.make_block();
        let arg64 = func.dfg.append_block_param(block, types::I64);
        let arg32 = func.dfg.append_block_param(block, types::I32);

        // Try to encode iadd_imm.i64 v1, -10.
        let inst64 = InstructionData::BinaryImm {
            opcode: Opcode::IaddImm,
            arg: arg64,
            imm: immediates::Imm64::new(-10),
        };

        // ADDI is I/0b00100
        assert_eq!(
            encstr(&*isa, isa.encode(&func, &inst64, types::I64)),
            "Ii#04"
        );

        // Try to encode iadd_imm.i64 v1, -10000.
        let inst64_large = InstructionData::BinaryImm {
            opcode: Opcode::IaddImm,
            arg: arg64,
            imm: immediates::Imm64::new(-10000),
        };

        // Immediate is out of range for ADDI.
        assert!(isa.encode(&func, &inst64_large, types::I64).is_err());

        // Create an iadd_imm.i32 which is encodable in RV64.
        let inst32 = InstructionData::BinaryImm {
            opcode: Opcode::IaddImm,
            arg: arg32,
            imm: immediates::Imm64::new(10),
        };

        // ADDIW is I/0b00110
        assert_eq!(
            encstr(&*isa, isa.encode(&func, &inst32, types::I32)),
            "Ii#06"
        );
    }

    // Same as above, but for RV32.
    #[test]
    fn test_32bitenc() {
        let shared_builder = settings::builder();
        let shared_flags = settings::Flags::new(shared_builder);
        let isa = isa::lookup(triple!("riscv32"))
            .unwrap()
            .finish(shared_flags);

        let mut func = Function::new();
        let block = func.dfg.make_block();
        let arg64 = func.dfg.append_block_param(block, types::I64);
        let arg32 = func.dfg.append_block_param(block, types::I32);

        // Try to encode iadd_imm.i64 v1, -10.
        let inst64 = InstructionData::BinaryImm {
            opcode: Opcode::IaddImm,
            arg: arg64,
            imm: immediates::Imm64::new(-10),
        };

        // In 32-bit mode, an i64 bit add should be narrowed.
        assert!(isa.encode(&func, &inst64, types::I64).is_err());

        // Try to encode iadd_imm.i64 v1, -10000.
        let inst64_large = InstructionData::BinaryImm {
            opcode: Opcode::IaddImm,
            arg: arg64,
            imm: immediates::Imm64::new(-10000),
        };

        // In 32-bit mode, an i64 bit add should be narrowed.
        assert!(isa.encode(&func, &inst64_large, types::I64).is_err());

        // Create an iadd_imm.i32 which is encodable in RV32.
        let inst32 = InstructionData::BinaryImm {
            opcode: Opcode::IaddImm,
            arg: arg32,
            imm: immediates::Imm64::new(10),
        };

        // ADDI is I/0b00100
        assert_eq!(
            encstr(&*isa, isa.encode(&func, &inst32, types::I32)),
            "Ii#04"
        );

        // Create an imul.i32 which is encodable in RV32, but only when use_m is true.
        let mul32 = InstructionData::Binary {
            opcode: Opcode::Imul,
            args: [arg32, arg32],
        };

        assert!(isa.encode(&func, &mul32, types::I32).is_err());
    }

    #[test]
    fn test_rv32m() {
        let shared_builder = settings::builder();
        let shared_flags = settings::Flags::new(shared_builder);

        // Set the supports_m stting which in turn enables the use_m predicate that unlocks
        // encodings for imul.
        let mut isa_builder = isa::lookup(triple!("riscv32")).unwrap();
        isa_builder.enable("supports_m").unwrap();

        let isa = isa_builder.finish(shared_flags);

        let mut func = Function::new();
        let block = func.dfg.make_block();
        let arg32 = func.dfg.append_block_param(block, types::I32);

        // Create an imul.i32 which is encodable in RV32M.
        let mul32 = InstructionData::Binary {
            opcode: Opcode::Imul,
            args: [arg32, arg32],
        };
        assert_eq!(
            encstr(&*isa, isa.encode(&func, &mul32, types::I32)),
            "R#10c"
        );
    }
}

impl fmt::Display for Isa {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}\n{}", self.shared_flags, self.isa_flags)
    }
}
