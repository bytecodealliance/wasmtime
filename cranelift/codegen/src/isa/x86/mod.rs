//! x86 Instruction Set Architectures.

mod abi;
mod binemit;
mod enc_tables;
mod registers;
pub mod settings;
#[cfg(feature = "unwind")]
pub mod unwind;

use super::super::settings as shared_settings;
#[cfg(feature = "testing_hooks")]
use crate::binemit::CodeSink;
use crate::binemit::{emit_function, MemoryCodeSink};
use crate::ir;
use crate::isa::enc_tables::{self as shared_enc_tables, lookup_enclist, Encodings};
use crate::isa::Builder as IsaBuilder;
#[cfg(feature = "unwind")]
use crate::isa::{unwind::systemv::RegisterMappingError, RegUnit};
use crate::isa::{EncInfo, RegClass, RegInfo, TargetIsa};
use crate::regalloc;
use crate::result::CodegenResult;
use crate::timing;
use alloc::borrow::Cow;
use alloc::boxed::Box;
use core::any::Any;
use core::fmt;
use core::hash::{Hash, Hasher};
use target_lexicon::{PointerWidth, Triple};

#[allow(dead_code)]
struct Isa {
    triple: Triple,
    shared_flags: shared_settings::Flags,
    isa_flags: settings::Flags,
    cpumode: &'static [shared_enc_tables::Level1Entry<u16>],
}

/// Get an ISA builder for creating x86 targets.
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
        PointerWidth::U16 => unimplemented!("x86-16"),
        PointerWidth::U32 => &enc_tables::LEVEL1_I32[..],
        PointerWidth::U64 => &enc_tables::LEVEL1_I64[..],
    };

    let isa_flags = settings::Flags::new(&shared_flags, builder);

    Box::new(Isa {
        triple,
        isa_flags,
        shared_flags,
        cpumode: level1,
    })
}

impl TargetIsa for Isa {
    fn name(&self) -> &'static str {
        "x86"
    }

    fn triple(&self) -> &Triple {
        &self.triple
    }

    fn flags(&self) -> &shared_settings::Flags {
        &self.shared_flags
    }

    fn hash_all_flags(&self, mut hasher: &mut dyn Hasher) {
        self.shared_flags.hash(&mut hasher);
        self.isa_flags.hash(&mut hasher);
    }

    fn uses_cpu_flags(&self) -> bool {
        true
    }

    fn uses_complex_addresses(&self) -> bool {
        true
    }

    fn register_info(&self) -> RegInfo {
        registers::INFO.clone()
    }

    #[cfg(feature = "unwind")]
    fn map_dwarf_register(&self, reg: RegUnit) -> Result<u16, RegisterMappingError> {
        unwind::systemv::map_reg(self, reg).map(|r| r.0)
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
        abi::legalize_signature(
            sig,
            &self.triple,
            current,
            &self.shared_flags,
            &self.isa_flags,
        )
    }

    fn regclass_for_abi_type(&self, ty: ir::Type) -> RegClass {
        abi::regclass_for_abi_type(ty)
    }

    fn allocatable_registers(&self, _func: &ir::Function) -> regalloc::RegisterSet {
        abi::allocatable_registers(&self.triple, &self.shared_flags)
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

    fn prologue_epilogue(&self, func: &mut ir::Function) -> CodegenResult<()> {
        let _tt = timing::prologue_epilogue();
        abi::prologue_epilogue(func, self)
    }

    fn unsigned_add_overflow_condition(&self) -> ir::condcodes::IntCC {
        ir::condcodes::IntCC::UnsignedLessThan
    }

    fn unsigned_sub_overflow_condition(&self) -> ir::condcodes::IntCC {
        ir::condcodes::IntCC::UnsignedLessThan
    }

    #[cfg(feature = "unwind")]
    fn create_unwind_info(
        &self,
        func: &ir::Function,
    ) -> CodegenResult<Option<super::unwind::UnwindInfo>> {
        abi::create_unwind_info(func, self)
    }

    #[cfg(feature = "unwind")]
    fn create_systemv_cie(&self) -> Option<gimli::write::CommonInformationEntry> {
        Some(unwind::systemv::create_cie())
    }

    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }
}

impl fmt::Display for Isa {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}\n{}", self.shared_flags, self.isa_flags)
    }
}
