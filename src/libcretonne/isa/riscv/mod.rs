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
}
