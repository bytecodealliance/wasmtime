//! RISC-V Instruction Set Architecture.

pub mod settings;

use super::super::settings as shared_settings;
use super::Builder as IsaBuilder;
use super::{TargetIsa, Encoding};
use ir::dfg::DataFlowGraph;
use ir::entities::Inst;

#[allow(dead_code)]
struct Isa {
    shared_flags: shared_settings::Flags,
    isa_flags: settings::Flags,
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
    Box::new(Isa {
        isa_flags: settings::Flags::new(&shared_flags, builder),
        shared_flags: shared_flags,
    })
}

impl TargetIsa for Isa {
    fn encode(&self, _: &DataFlowGraph, _: &Inst) -> Option<Encoding> {
        unimplemented!()
    }
}
