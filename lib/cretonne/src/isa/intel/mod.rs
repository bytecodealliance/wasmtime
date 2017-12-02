//! Intel Instruction Set Architectures.

pub mod settings;
mod abi;
mod binemit;
mod enc_tables;
mod registers;

use binemit::{CodeSink, MemoryCodeSink, emit_function};
use super::super::settings as shared_settings;
use isa::enc_tables::{self as shared_enc_tables, lookup_enclist, Encodings};
use isa::Builder as IsaBuilder;
use isa::{TargetIsa, RegInfo, RegClass, EncInfo, RegUnit};
use self::registers::RU;
use ir;
use regalloc;
use result;
use ir::{InstBuilder, InstructionData, Opcode};
use ir::immediates::Imm64;
use stack_layout::layout_stack;
use cursor::{Cursor, EncCursor};


#[allow(dead_code)]
struct Isa {
    shared_flags: shared_settings::Flags,
    isa_flags: settings::Flags,
    cpumode: &'static [shared_enc_tables::Level1Entry<u16>],
}

/// Get an ISA builder for creating Intel targets.
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
        &enc_tables::LEVEL1_I64[..]
    } else {
        &enc_tables::LEVEL1_I32[..]
    };
    Box::new(Isa {
        isa_flags: settings::Flags::new(&shared_flags, builder),
        shared_flags,
        cpumode: level1,
    })
}

impl TargetIsa for Isa {
    fn name(&self) -> &'static str {
        "intel"
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
        abi::legalize_signature(sig, &self.shared_flags, current)
    }

    fn regclass_for_abi_type(&self, ty: ir::Type) -> RegClass {
        abi::regclass_for_abi_type(ty)
    }

    fn allocatable_registers(&self, func: &ir::Function) -> regalloc::AllocatableSet {
        abi::allocatable_registers(func, &self.shared_flags)
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

    fn prologue_epilogue(&self, func: &mut ir::Function) -> result::CtonResult {
        let word_size = if self.flags().is_64bit() { 8 } else { 4 };
        let csr_type = if self.flags().is_64bit() {
            ir::types::I64
        } else {
            ir::types::I32
        };
        let csrs = abi::callee_saved_registers(&self.shared_flags);

        let mut csr_stack_size = word_size; // Size of RBP to start with
        for _reg in &csrs {
            csr_stack_size += word_size;
        }

        let stack_offset = -(csr_stack_size as i32);
        let slot = ir::StackSlotData {
            kind: ir::StackSlotKind::IncomingArg,
            size: csr_stack_size,
            offset: stack_offset,
        };
        func.create_stack_slot(slot);

        let total_stack_size = layout_stack(&mut func.stack_slots, word_size)?;
        let local_stack_size = (total_stack_size - csr_stack_size) as i64;

        // Build up list of args, which we'll append forwards to the params and
        // backwards to the returns.
        let mut csr_args = Vec::new();
        csr_args.push(ir::AbiParam::special_reg(
            csr_type,
            ir::ArgumentPurpose::FramePointer,
            RU::rbp as RegUnit,
        ));
        for reg in &csrs {
            csr_args.push(ir::AbiParam::special_reg(
                csr_type,
                ir::ArgumentPurpose::CalleeSaved,
                *reg as RegUnit,
            ));
        }

        for csr_arg in &csr_args {
            func.signature.params.push(*csr_arg);
        }
        for csr_arg in csr_args.iter().rev() {
            func.signature.returns.push(*csr_arg);
        }

        // Append param to entry EBB
        let entry_ebb = func.layout.entry_block().expect("missing entry block");
        func.dfg.append_ebb_param(entry_ebb, csr_type);

        // Find our frame pointer parameter Value
        let fp = func.special_param(ir::ArgumentPurpose::FramePointer)
            .expect("missing frame pointer");

        // Assign it a location
        func.locations[fp] = ir::ValueLoc::Reg(RU::rbp as RegUnit);

        let mut csr_vals = Vec::new();
        for reg in &csrs {
            // Append param to entry EBB
            func.dfg.append_ebb_param(entry_ebb, csr_type);

            let csr_arg = func.dfg.ebb_params(entry_ebb).last().expect(
                "no last argument",
            );

            // Assign it a location
            func.locations[*csr_arg] = ir::ValueLoc::Reg(*reg as RegUnit);

            // Remember it so we can push it momentarily
            csr_vals.push(*csr_arg);
        }


        // Insert prologue
        {
            let mut pos = EncCursor::new(func, self).at_first_insertion_point(entry_ebb);
            pos.ins().x86_push(fp);
            pos.ins().copy_special(
                RU::rsp as RegUnit,
                RU::rbp as RegUnit,
            );
            if local_stack_size > 0 {
                pos.ins().adjust_sp_imm(Imm64::new(-local_stack_size));
            }

            for csr_arg in csr_vals {
                pos.ins().x86_push(csr_arg);
            }
        }

        // Find all 'return' instructions
        let mut return_insts = Vec::new();
        for ebb in func.layout.ebbs() {
            for inst in func.layout.ebb_insts(ebb) {
                if let InstructionData::MultiAry { opcode, .. } = func.dfg[inst] {
                    if opcode == Opcode::Return {
                        return_insts.push(inst);
                    }
                }

            }
        }

        // Insert an epilogue directly before every 'return'
        for inst in return_insts {
            self.insert_epilogue(inst, local_stack_size, func, &csrs, csr_type);
        }


        Ok(())
    }
}

impl Isa {
    fn insert_epilogue(
        &self,
        inst: ir::Inst,
        stack_size: i64,
        func: &mut ir::Function,
        csrs: &Vec<RU>,
        csr_type: ir::types::Type,
    ) {
        let mut return_values = Vec::new();

        let mut pos = EncCursor::new(func, self).at_inst(inst);
        if stack_size > 0 {
            pos.ins().adjust_sp_imm(Imm64::new(stack_size));
        }
        for reg in csrs.iter().rev() {
            let csr_ret = pos.ins().x86_pop(csr_type);
            return_values.push((csr_ret, *reg));
        }
        let fp_ret = pos.ins().x86_pop(csr_type);
        return_values.push((fp_ret, RU::rbp));

        let func = pos.func;
        for (val, reg) in return_values {
            func.locations[val] = ir::ValueLoc::Reg(reg as RegUnit);
            func.dfg.append_inst_arg(inst, val);
        }
    }
}
