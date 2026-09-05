//! Initial, opt-in Nixe leaf-fragment ABI.
//!
//! This is not a system calling convention. The caller owns every register
//! and places a 64-byte-aligned spill area at offset zero of NativeFrame,
//! addressed by r15 (x86-64) or x21 (AArch64). Canonical external entries define
//! their own inputs. Production gateways, physical fast-entry contracts and
//! boundary state maps are not implemented here yet.

use crate::{CodegenError, CodegenResult, ir, isa::TargetIsa};
use alloc::format;

/// Bytes reserved for boundary transfers; never allocated to backend spills.
pub const TRANSFER_BYTES: u32 = 2048;
/// Total fixed storage, including boundary transfers.
pub const FRAME_BYTES: u32 = 16384;

/// Declare canonical external entries into a single compiled unit.
///
/// Each entry defines its own inputs (no block parameters). An analysis-only
/// root makes every entry reachable to dominance and regalloc; it and its
/// outgoing critical edges are never emitted. The selector has no runtime
/// meaning. Final offsets are exported by `MachBufferFinalized::nixe_entries`.
/// Call once after constructing the body, before optimization.
pub fn set_entries(func: &mut ir::Function, entries: &[ir::Block]) -> CodegenResult<()> {
    use crate::cursor::{Cursor, FuncCursor};
    use ir::{InstBuilder, types};
    let fail = || CodegenError::Unsupported("Nixe ABI: invalid canonical entries".into());
    if entries.is_empty() || !func.nixe_entries.is_empty() {
        return Err(fail());
    }
    for (index, &block) in entries.iter().enumerate() {
        if !func.layout.is_block_inserted(block)
            || !func.dfg.block_params(block).is_empty()
            || entries[..index].contains(&block)
        {
            return Err(fail());
        }
    }
    let root = func.dfg.make_block();
    let first = func.layout.entry_block().unwrap();
    func.layout.insert_block(root, first);
    let targets: alloc::vec::Vec<_> = entries
        .iter()
        .map(|&block| func.dfg.block_call(block, &[]))
        .collect();
    let table = func.create_jump_table(ir::JumpTableData::new(targets[0], &targets[1..]));
    let mut c = FuncCursor::new(func).at_bottom(root);
    let selector = c.ins().get_pinned_reg(types::I64);
    let selector = c.ins().ireduce(types::I32, selector);
    c.ins().br_table(selector, table);
    c.func.nixe_entries.extend_from_slice(entries);
    Ok(())
}

/// Validate the analysis-root boundary whenever optimization/lowering consumes
/// it. No computation defined there may become an external entry's live-in.
pub(crate) fn validate_entries(func: &ir::Function, isa: &dyn TargetIsa) -> CodegenResult<()> {
    if func.nixe_entries.is_empty() {
        return Ok(());
    }
    let fail = |detail| CodegenError::Unsupported(format!("Nixe entries: {detail}"));
    if !isa.flags().enable_nixe_abi() {
        return Err(fail("enable_nixe_abi is required"));
    }
    let root = func
        .layout
        .entry_block()
        .ok_or_else(|| fail("missing analysis root"))?;
    for &entry in &func.nixe_entries {
        if entry == root
            || !func.layout.is_block_inserted(entry)
            || !func.dfg.block_params(entry).is_empty()
        {
            return Err(fail("entries must exist and define their own inputs"));
        }
    }
    for block in func.layout.blocks() {
        for inst in func.layout.block_insts(block) {
            if block == root
                && !matches!(
                    func.dfg.insts[inst].opcode(),
                    ir::Opcode::GetPinnedReg
                        | ir::Opcode::Ireduce
                        | ir::Opcode::BrTable
                        | ir::Opcode::Jump
                )
            {
                return Err(fail("unexpected computation in analysis root"));
            }
            for dest in func.dfg.insts[inst]
                .branch_destination(&func.dfg.jump_tables, &func.dfg.exception_tables)
            {
                let dest = dest.block(&func.dfg.value_lists);
                if dest == root || (block == root && !func.nixe_entries.contains(&dest)) {
                    return Err(fail("invalid analysis-root edge"));
                }
            }
            if block != root {
                for value in func.dfg.inst_values(inst) {
                    if let ir::ValueDef::Result(def, _) = func.dfg.value_def(value) {
                        if func.layout.inst_block(def) == Some(root) {
                            return Err(fail("entry depends on an analysis-root value"));
                        }
                    }
                }
            }
        }
    }
    let last = func
        .layout
        .last_inst(root)
        .ok_or_else(|| fail("empty analysis root"))?;
    let destinations =
        func.dfg.insts[last].branch_destination(&func.dfg.jump_tables, &func.dfg.exception_tables);
    if func.nixe_entries.iter().any(|entry| {
        !destinations
            .iter()
            .any(|dest| dest.block(&func.dfg.value_lists) == *entry)
    }) {
        return Err(fail("analysis root must reach every external entry"));
    }
    Ok(())
}

#[cfg(all(test, feature = "x86", feature = "arm64"))]
mod multi_entry;

pub(crate) fn validate(func: &ir::Function, isa: &dyn TargetIsa) -> CodegenResult<()> {
    validate_entries(func, isa)?;
    if !isa.flags().enable_nixe_abi() {
        return Ok(());
    }
    let unsupported = |detail: &str| CodegenError::Unsupported(format!("Nixe ABI: {detail}"));
    if !matches!(isa.name(), "x64" | "aarch64")
        || isa.triple().operating_system != target_lexicon::OperatingSystem::Linux
    {
        return Err(unsupported("only Linux x86-64 and AArch64 are supported"));
    }
    if !isa.flags().enable_pinned_reg() {
        return Err(unsupported("enable_pinned_reg is required"));
    }
    if !func.signature.params.is_empty() || !func.signature.returns.is_empty() {
        return Err(unsupported(
            "system-ABI parameters and results are not supported",
        ));
    }
    if func.stack_limit.is_some() || !func.dynamic_stack_slots.is_empty() {
        return Err(unsupported(
            "stack limits and dynamic stack slots are not supported",
        ));
    }
    if func
        .dfg
        .values_labels
        .as_ref()
        .is_some_and(|labels| !labels.is_empty())
    {
        return Err(unsupported(
            "debug value locations are not Nixe physical state maps",
        ));
    }
    for slot in func.sized_stack_slots.values() {
        if slot.align_shift > 6 || slot.size > FRAME_BYTES - TRANSFER_BYTES {
            return Err(unsupported(
                "explicit stack slot exceeds fixed frame bounds",
            ));
        }
    }
    for block in func.layout.blocks() {
        for inst in func.layout.block_insts(block) {
            let op = func.dfg.insts[inst].opcode();
            if op.is_call()
                || op.is_return()
                || matches!(
                    op,
                    ir::Opcode::SetPinnedReg
                        | ir::Opcode::GetFramePointer
                        | ir::Opcode::GetReturnAddress
                        | ir::Opcode::GetStackPointer
                        | ir::Opcode::StackSwitch
                        | ir::Opcode::TlsValue
                )
            {
                return Err(unsupported(&format!(
                    "{op} requires an unimplemented native boundary"
                )));
            }
        }
    }
    Ok(())
}

#[cfg(all(test, feature = "x86", feature = "arm64"))]
mod tests {
    use super::*;
    use crate::cursor::{Cursor, FuncCursor};
    use crate::ir::{
        InstBuilder, MemFlagsData as MemFlags, StackSlotData, StackSlotKind, TrapCode, types,
    };
    use crate::settings::{self, Configurable};
    use crate::{Context, isa};
    use alloc::{string::String, vec::Vec};
    use cranelift_control::ControlPlane;

    pub(super) fn target(
        triple: &str,
        allocator: &str,
        nixe: bool,
    ) -> alloc::sync::Arc<dyn TargetIsa> {
        let mut flags = settings::builder();
        flags.set("enable_pinned_reg", "true").unwrap();
        flags
            .set("enable_nixe_abi", if nixe { "true" } else { "false" })
            .unwrap();
        flags.set("regalloc_algorithm", allocator).unwrap();
        flags.set("machine_code_cfg_info", "true").unwrap();
        flags
            .set(
                "opt_level",
                if allocator == "single_pass" {
                    "none"
                } else {
                    "speed"
                },
            )
            .unwrap();
        isa::lookup(triple.parse().unwrap())
            .unwrap()
            .finish(settings::Flags::new(flags))
            .unwrap()
    }

    fn fragment(count: usize, slot_bytes: u32) -> ir::Function {
        let mut f = ir::Function::new();
        let slot = f.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            slot_bytes,
            4,
        ));
        let block = f.dfg.make_block();
        f.layout.append_block(block);
        let mut c = FuncCursor::new(&mut f).at_bottom(block);
        let frame = c.ins().get_pinned_reg(types::I64);
        let data = c.ins().load(types::I64, MemFlags::trusted(), frame, 0);
        let mut ints = Vec::new();
        let mut vectors = Vec::new();
        for i in 0..count {
            ints.push(
                c.ins()
                    .load(types::I64, MemFlags::new(), data, (i * 8) as i32),
            );
            vectors.push(
                c.ins()
                    .load(types::I64X2, MemFlags::new(), data, (4096 + i * 16) as i32),
            );
        }
        for i in (0..count).rev() {
            c.ins()
                .store(MemFlags::new(), ints[i], data, (8192 + i * 8) as i32);
            c.ins()
                .store(MemFlags::new(), vectors[i], data, (12288 + i * 16) as i32);
        }
        let addr = c.ins().stack_addr(types::I64, slot, 0);
        c.ins().store(MemFlags::trusted(), data, addr, 0);
        // Also make the slot address escape, exercising LEA/LoadAddr instead
        // of only the folded stack-slot memory operand.
        c.ins().store(MemFlags::new(), addr, data, 2048);
        c.ins().trap(TrapCode::unwrap_user(1));
        f
    }

    pub(super) fn compile(
        f: ir::Function,
        isa: &dyn TargetIsa,
    ) -> Result<crate::CompiledCode, String> {
        let mut cx = Context::for_function(f);
        cx.set_disasm(true);
        cx.compile(isa, &mut ControlPlane::default())
            .map_err(|e| format!("{e:?}"))?;
        Ok(cx.take_compiled_code().unwrap())
    }

    #[test]
    fn pressure_uses_external_frame_on_both_targets_and_allocators() {
        for triple in ["x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu"] {
            for allocator in ["single_pass", "backtracking"] {
                let isa = target(triple, allocator, true);
                for count in [40, 57, 72] {
                    let code = compile(fragment(count, 32), &*isa).unwrap();
                    let frame = code.buffer.frame_layout().unwrap();
                    let extent = frame.nixe_frame_size.unwrap();
                    assert!(extent > TRANSFER_BYTES + 32, "pressure must actually spill");
                    assert!(extent <= FRAME_BYTES);
                    for slot in frame.stackslots.values() {
                        assert!(slot.offset >= TRANSFER_BYTES && slot.offset + 32 <= extent);
                    }
                    #[cfg(feature = "disas")]
                    let native_asm = code.disassemble(None, &isa.to_capstone().unwrap()).unwrap();
                    #[cfg(feature = "disas")]
                    let asm = &native_asm;
                    #[cfg(not(feature = "disas"))]
                    let asm = code.vcode.as_ref().unwrap();
                    let forbidden: &[&str] = if triple.starts_with("x86") {
                        &[
                            "rsp", "rbp", "esp", "ebp", "r11", "r11d", "r13", "r13d", "r14",
                            "r14d", "ret", "call", "push", "pop",
                        ]
                    } else {
                        &[
                            "sp", "x29", "w29", "x19", "w19", "x20", "w20", "ret", "bl", "blr",
                        ]
                    };
                    for token in asm.split(|c: char| !c.is_ascii_alphanumeric()) {
                        assert!(
                            !forbidden.contains(&token),
                            "{triple}/{allocator}: {token}\n{asm}"
                        );
                    }
                    assert!(
                        asm.contains(if triple.starts_with("x86") {
                            "r15"
                        } else {
                            "x21"
                        }),
                        "{asm}"
                    );
                }
            }
        }
    }

    #[test]
    fn oversized_frames_and_system_returns_are_rejected() {
        for triple in ["x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu"] {
            for allocator in ["single_pass", "backtracking"] {
                let isa = target(triple, allocator, true);
                let full = compile(fragment(0, FRAME_BYTES - TRANSFER_BYTES), &*isa).unwrap();
                assert_eq!(
                    full.buffer.frame_layout().unwrap().nixe_frame_size,
                    Some(FRAME_BYTES)
                );
                assert!(compile(fragment(72, FRAME_BYTES - TRANSFER_BYTES), &*isa).is_err());
                let mut f = ir::Function::new();
                let block = f.dfg.make_block();
                f.layout.append_block(block);
                FuncCursor::new(&mut f).at_bottom(block).ins().return_(&[]);
                assert!(
                    compile(f.clone(), &*isa)
                        .unwrap_err()
                        .contains("unimplemented native boundary")
                );
                // The new mode must not suppress ordinary ABI prologues/returns.
                let ordinary = target(triple, allocator, false);
                assert!(
                    compile(f, &*ordinary)
                        .unwrap()
                        .vcode
                        .unwrap()
                        .contains("ret")
                );
            }
        }
    }

    #[test]
    fn invalid_configuration_and_calls_fail_before_producing_code() {
        let mut flags = settings::builder();
        flags.set("enable_nixe_abi", "true").unwrap();
        let isa = isa::lookup("x86_64-unknown-linux-gnu".parse().unwrap())
            .unwrap()
            .finish(settings::Flags::new(flags))
            .unwrap();
        assert!(
            compile(fragment(0, 16), &*isa)
                .unwrap_err()
                .contains("enable_pinned_reg")
        );

        for triple in ["x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu"] {
            let isa = target(triple, "backtracking", true);
            let mut f = fragment(0, 16);
            f.signature.params.push(ir::AbiParam::new(types::I64));
            let block = f.layout.entry_block().unwrap();
            f.dfg.append_block_param(block, types::I64);
            assert!(compile(f, &*isa).unwrap_err().contains("parameters"));
            let mut f = fragment(0, 16);
            let end = f.layout.last_inst(f.layout.entry_block().unwrap()).unwrap();
            let sig = f.import_signature(ir::Signature::new(isa::CallConv::SystemV));
            let mut c = FuncCursor::new(&mut f).at_inst(end);
            let addr = c.ins().iconst(types::I64, 0);
            c.ins().call_indirect(sig, addr, &[]);
            assert!(
                compile(f, &*isa)
                    .unwrap_err()
                    .contains("unimplemented native boundary")
            );
        }
    }

    #[test]
    fn zero_byte_markers_preserve_selected_offsets_in_hot_and_cold_blocks() {
        for triple in ["x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu"] {
            for allocator in ["single_pass", "backtracking"] {
                let isa = target(triple, allocator, true);
                let mut f = ir::Function::new();
                let root = f.dfg.make_block();
                let hot = f.dfg.make_block();
                let cold = f.dfg.make_block();
                for block in [root, hot, cold] {
                    f.layout.append_block(block);
                }
                f.layout.set_cold(cold);
                let mut c = FuncCursor::new(&mut f).at_bottom(root);
                let condition = c.ins().get_pinned_reg(types::I64);
                c.ins().brif(condition, hot, &[], cold, &[]);
                for (block, id) in [(hot, 11), (cold, 22)] {
                    let mut c = FuncCursor::new(&mut f).at_bottom(block);
                    let marker = c.ins().sequence_point();
                    c.func.debug_tags.set(marker, [ir::DebugTag::User(id)]);
                    c.ins().trap(TrapCode::unwrap_user(id as u8));
                }
                let code = compile(f, &*isa).unwrap();
                let tags: Vec<_> = code.buffer.debug_tags().collect();
                assert_eq!(tags.len(), 2);
                for tag in tags {
                    let [ir::DebugTag::User(id)] = tag.tags else {
                        panic!("missing entry identity")
                    };
                    assert!(
                        code.buffer
                            .traps()
                            .iter()
                            .any(|trap| trap.offset == tag.offset
                                && trap.code == TrapCode::unwrap_user(*id as u8))
                    );
                }
            }
        }
    }

    /// Probe regalloc's actual operand API, not its optional debug ranges.
    /// The virtual root is analysis-only here: this test does not implement
    /// Cranelift multi-entry lowering or demonstrate safe root removal.
    #[test]
    fn fixed_entry_defs_and_exact_boundary_allocations_need_no_new_allocator() {
        use regalloc2::{
            Algorithm, Allocation, Block, Inst, InstRange, MachineEnv, Operand, PReg, PRegSet,
            RegClass, RegallocOptions, VReg,
        };
        const VALUES: usize = 12;
        const STRIDE: usize = VALUES + 1;
        struct Probe {
            operands: Vec<Vec<Operand>>,
            children: [Block; 2],
            root: [Block; 1],
        }
        impl regalloc2::Function for Probe {
            fn num_insts(&self) -> usize {
                self.operands.len()
            }
            fn num_blocks(&self) -> usize {
                3
            }
            fn entry_block(&self) -> Block {
                Block::new(0)
            }
            fn block_insns(&self, b: Block) -> InstRange {
                let (start, end) = if b.index() == 0 {
                    (0, 1)
                } else {
                    (1 + (b.index() - 1) * STRIDE, 1 + b.index() * STRIDE)
                };
                InstRange::new(Inst::new(start), Inst::new(end))
            }
            fn block_succs(&self, b: Block) -> &[Block] {
                if b.index() == 0 { &self.children } else { &[] }
            }
            fn block_preds(&self, b: Block) -> &[Block] {
                if b.index() == 0 { &[] } else { &self.root }
            }
            fn block_params(&self, _: Block) -> &[VReg] {
                &[]
            }
            fn is_ret(&self, i: Inst) -> bool {
                i.index() == STRIDE || i.index() == 2 * STRIDE
            }
            fn is_branch(&self, i: Inst) -> bool {
                i.index() == 0
            }
            fn branch_blockparams(&self, _: Block, _: Inst, _: usize) -> &[VReg] {
                &[]
            }
            fn inst_operands(&self, i: Inst) -> &[Operand] {
                &self.operands[i.index()]
            }
            fn inst_clobbers(&self, _: Inst) -> PRegSet {
                PRegSet::empty()
            }
            fn num_vregs(&self) -> usize {
                2 * VALUES
            }
            fn spillslot_size(&self, _: RegClass) -> usize {
                1
            }
        }
        let mut probe = Probe {
            operands: alloc::vec![alloc::vec![]],
            children: [Block::new(1), Block::new(2)],
            root: [Block::new(0)],
        };
        for entry in 0..2 {
            for v in 0..VALUES {
                let vr = VReg::new(entry * VALUES + v, RegClass::Int);
                let op = if v == 0 {
                    Operand::reg_fixed_def(vr, PReg::new(entry, RegClass::Int))
                } else {
                    Operand::reg_def(vr)
                };
                probe.operands.push(alloc::vec![op]);
            }
            probe.operands.push(
                (0..VALUES)
                    .map(|v| Operand::any_use(VReg::new(entry * VALUES + v, RegClass::Int)))
                    .collect(),
            );
        }
        let mut regs = PRegSet::empty();
        for reg in 0..4 {
            regs.add(PReg::new(reg, RegClass::Int));
        }
        let env = MachineEnv {
            preferred_regs_by_class: [regs, PRegSet::empty(), PRegSet::empty()],
            non_preferred_regs_by_class: [PRegSet::empty(); 3],
            scratch_by_class: [None; 3],
            fixed_stack_slots: alloc::vec![],
        };
        for algorithm in [Algorithm::Ion, Algorithm::Fastalloc] {
            let result = regalloc2::run(
                &probe,
                &env,
                &RegallocOptions {
                    algorithm,
                    validate_ssa: true,
                    ..RegallocOptions::default()
                },
            )
            .unwrap();
            let mut checker = regalloc2::checker::Checker::new(&probe, &env);
            checker.prepare(&result);
            checker.run().unwrap();
            for entry in 0..2 {
                assert_eq!(
                    result.inst_allocs(Inst::new(1 + entry * STRIDE)),
                    &[Allocation::reg(PReg::new(entry, RegClass::Int))]
                );
                let locations = result.inst_allocs(Inst::new((entry + 1) * STRIDE));
                assert_eq!(locations.len(), VALUES);
                assert!(locations.iter().any(|l| l.as_stack().is_some()));
                for loc in locations {
                    if let Some(slot) = loc.as_stack() {
                        assert!(TRANSFER_BYTES + (slot.index() as u32 + 1) * 8 <= FRAME_BYTES);
                    } else {
                        assert!(loc.as_reg().is_some());
                    }
                }
            }
        }
    }
}
