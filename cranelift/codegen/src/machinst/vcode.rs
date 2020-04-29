//! This implements the VCode container: a CFG of Insts that have been lowered.
//!
//! VCode is virtual-register code. An instruction in VCode is almost a machine
//! instruction; however, its register slots can refer to virtual registers in
//! addition to real machine registers.
//!
//! VCode is structured with traditional basic blocks, and
//! each block must be terminated by an unconditional branch (one target), a
//! conditional branch (two targets), or a return (no targets). Note that this
//! slightly differs from the machine code of most ISAs: in most ISAs, a
//! conditional branch has one target (and the not-taken case falls through).
//! However, we expect that machine backends will elide branches to the following
//! block (i.e., zero-offset jumps), and will be able to codegen a branch-cond /
//! branch-uncond pair if *both* targets are not fallthrough. This allows us to
//! play with layout prior to final binary emission, as well, if we want.
//!
//! See the main module comment in `mod.rs` for more details on the VCode-based
//! backend pipeline.

use crate::entity::SecondaryMap;
use crate::ir;
use crate::ir::SourceLoc;
use crate::machinst::*;
use crate::settings;

use regalloc::Function as RegallocFunction;
use regalloc::Set as RegallocSet;
use regalloc::{BlockIx, InstIx, Range, RegAllocResult, RegClass, RegUsageCollector};

use alloc::boxed::Box;
use alloc::vec::Vec;
use log::debug;
use smallvec::SmallVec;
use std::fmt;
use std::iter;
use std::string::String;

/// Index referring to an instruction in VCode.
pub type InsnIndex = u32;
/// Index referring to a basic block in VCode.
pub type BlockIndex = u32;

/// VCodeInst wraps all requirements for a MachInst to be in VCode: it must be
/// a `MachInst` and it must be able to emit itself at least to a `SizeCodeSink`.
pub trait VCodeInst: MachInst + MachInstEmit<MachSection> + MachInstEmit<MachSectionSize> {}
impl<I: MachInst + MachInstEmit<MachSection> + MachInstEmit<MachSectionSize>> VCodeInst for I {}

/// A function in "VCode" (virtualized-register code) form, after lowering.
/// This is essentially a standard CFG of basic blocks, where each basic block
/// consists of lowered instructions produced by the machine-specific backend.
pub struct VCode<I: VCodeInst> {
    /// Function liveins.
    liveins: RegallocSet<RealReg>,

    /// Function liveouts.
    liveouts: RegallocSet<RealReg>,

    /// VReg IR-level types.
    vreg_types: Vec<Type>,

    /// Lowered machine instructions in order corresponding to the original IR.
    insts: Vec<I>,

    /// Source locations for each instruction. (`SourceLoc` is a `u32`, so it is
    /// reasonable to keep one of these per instruction.)
    srclocs: Vec<SourceLoc>,

    /// Entry block.
    entry: BlockIndex,

    /// Block instruction indices.
    block_ranges: Vec<(InsnIndex, InsnIndex)>,

    /// Block successors: index range in the successor-list below.
    block_succ_range: Vec<(usize, usize)>,

    /// Block successor lists, concatenated into one Vec. The `block_succ_range`
    /// list of tuples above gives (start, end) ranges within this list that
    /// correspond to each basic block's successors.
    block_succs: Vec<BlockIndex>,

    /// Block indices by IR block.
    block_by_bb: SecondaryMap<ir::Block, BlockIndex>,

    /// IR block for each VCode Block. The length of this Vec will likely be
    /// less than the total number of Blocks, because new Blocks (for edge
    /// splits, for example) are appended during lowering.
    bb_by_block: Vec<ir::Block>,

    /// Order of block IDs in final generated code.
    final_block_order: Vec<BlockIndex>,

    /// Final block offsets. Computed during branch finalization and used
    /// during emission.
    final_block_offsets: Vec<CodeOffset>,

    /// Size of code, accounting for block layout / alignment.
    code_size: CodeOffset,

    /// ABI object.
    abi: Box<dyn ABIBody<I = I>>,
}

/// A builder for a VCode function body. This builder is designed for the
/// lowering approach that we take: we traverse basic blocks in forward
/// (original IR) order, but within each basic block, we generate code from
/// bottom to top; and within each IR instruction that we visit in this reverse
/// order, we emit machine instructions in *forward* order again.
///
/// Hence, to produce the final instructions in proper order, we perform two
/// swaps.  First, the machine instructions (`I` instances) are produced in
/// forward order for an individual IR instruction. Then these are *reversed*
/// and concatenated to `bb_insns` at the end of the IR instruction lowering.
/// The `bb_insns` vec will thus contain all machine instructions for a basic
/// block, in reverse order. Finally, when we're done with a basic block, we
/// reverse the whole block's vec of instructions again, and concatenate onto
/// the VCode's insts.
pub struct VCodeBuilder<I: VCodeInst> {
    /// In-progress VCode.
    vcode: VCode<I>,

    /// Current basic block instructions, in reverse order (because blocks are
    /// built bottom-to-top).
    bb_insns: SmallVec<[(I, SourceLoc); 32]>,

    /// Current IR-inst instructions, in forward order.
    ir_inst_insns: SmallVec<[(I, SourceLoc); 4]>,

    /// Start of succs for the current block in the concatenated succs list.
    succ_start: usize,

    /// Current source location.
    cur_srcloc: SourceLoc,
}

impl<I: VCodeInst> VCodeBuilder<I> {
    /// Create a new VCodeBuilder.
    pub fn new(abi: Box<dyn ABIBody<I = I>>) -> VCodeBuilder<I> {
        let vcode = VCode::new(abi);
        VCodeBuilder {
            vcode,
            bb_insns: SmallVec::new(),
            ir_inst_insns: SmallVec::new(),
            succ_start: 0,
            cur_srcloc: SourceLoc::default(),
        }
    }

    /// Access the ABI object.
    pub fn abi(&mut self) -> &mut dyn ABIBody<I = I> {
        &mut *self.vcode.abi
    }

    /// Set the type of a VReg.
    pub fn set_vreg_type(&mut self, vreg: VirtualReg, ty: Type) {
        while self.vcode.vreg_types.len() <= vreg.get_index() {
            self.vcode.vreg_types.push(ir::types::I8); // Default type.
        }
        self.vcode.vreg_types[vreg.get_index()] = ty;
    }

    /// Return the underlying bb-to-BlockIndex map.
    pub fn blocks_by_bb(&self) -> &SecondaryMap<ir::Block, BlockIndex> {
        &self.vcode.block_by_bb
    }

    /// Initialize the bb-to-BlockIndex map. Returns the first free
    /// BlockIndex.
    pub fn init_bb_map(&mut self, blocks: &[ir::Block]) -> BlockIndex {
        let mut bindex: BlockIndex = 0;
        for bb in blocks.iter() {
            self.vcode.block_by_bb[*bb] = bindex;
            self.vcode.bb_by_block.push(*bb);
            bindex += 1;
        }
        bindex
    }

    /// Get the BlockIndex for an IR block.
    pub fn bb_to_bindex(&self, bb: ir::Block) -> BlockIndex {
        self.vcode.block_by_bb[bb]
    }

    /// Set the current block as the entry block.
    pub fn set_entry(&mut self, block: BlockIndex) {
        self.vcode.entry = block;
    }

    /// End the current IR instruction. Must be called after pushing any
    /// instructions and prior to ending the basic block.
    pub fn end_ir_inst(&mut self) {
        while let Some(pair) = self.ir_inst_insns.pop() {
            self.bb_insns.push(pair);
        }
    }

    /// End the current basic block. Must be called after emitting vcode insts
    /// for IR insts and prior to ending the function (building the VCode).
    pub fn end_bb(&mut self) -> BlockIndex {
        assert!(self.ir_inst_insns.is_empty());
        let block_num = self.vcode.block_ranges.len() as BlockIndex;
        // Push the instructions.
        let start_idx = self.vcode.insts.len() as InsnIndex;
        while let Some((i, loc)) = self.bb_insns.pop() {
            self.vcode.insts.push(i);
            self.vcode.srclocs.push(loc);
        }
        let end_idx = self.vcode.insts.len() as InsnIndex;
        // Add the instruction index range to the list of blocks.
        self.vcode.block_ranges.push((start_idx, end_idx));
        // End the successors list.
        let succ_end = self.vcode.block_succs.len();
        self.vcode
            .block_succ_range
            .push((self.succ_start, succ_end));
        self.succ_start = succ_end;

        block_num
    }

    /// Push an instruction for the current BB and current IR inst within the BB.
    pub fn push(&mut self, insn: I) {
        match insn.is_term() {
            MachTerminator::None | MachTerminator::Ret => {}
            MachTerminator::Uncond(target) => {
                self.vcode.block_succs.push(target);
            }
            MachTerminator::Cond(true_branch, false_branch) => {
                self.vcode.block_succs.push(true_branch);
                self.vcode.block_succs.push(false_branch);
            }
            MachTerminator::Indirect(targets) => {
                for target in targets {
                    self.vcode.block_succs.push(*target);
                }
            }
        }
        self.ir_inst_insns.push((insn, self.cur_srcloc));
    }

    /// Set the current source location.
    pub fn set_srcloc(&mut self, srcloc: SourceLoc) {
        self.cur_srcloc = srcloc;
    }

    /// Build the final VCode.
    pub fn build(self) -> VCode<I> {
        assert!(self.ir_inst_insns.is_empty());
        assert!(self.bb_insns.is_empty());
        self.vcode
    }
}

fn block_ranges(indices: &[InstIx], len: usize) -> Vec<(usize, usize)> {
    let v = indices
        .iter()
        .map(|iix| iix.get() as usize)
        .chain(iter::once(len))
        .collect::<Vec<usize>>();
    v.windows(2).map(|p| (p[0], p[1])).collect()
}

fn is_redundant_move<I: VCodeInst>(insn: &I) -> bool {
    if let Some((to, from)) = insn.is_move() {
        to.to_reg() == from
    } else {
        false
    }
}

fn is_trivial_jump_block<I: VCodeInst>(vcode: &VCode<I>, block: BlockIndex) -> Option<BlockIndex> {
    let range = vcode.block_insns(BlockIx::new(block));

    debug!(
        "is_trivial_jump_block: block {} has len {}",
        block,
        range.len()
    );

    if range.len() != 1 {
        return None;
    }
    let insn = range.first();

    debug!(
        " -> only insn is: {:?} with terminator {:?}",
        vcode.get_insn(insn),
        vcode.get_insn(insn).is_term()
    );

    match vcode.get_insn(insn).is_term() {
        MachTerminator::Uncond(target) => Some(target),
        _ => None,
    }
}

impl<I: VCodeInst> VCode<I> {
    /// New empty VCode.
    fn new(abi: Box<dyn ABIBody<I = I>>) -> VCode<I> {
        VCode {
            liveins: abi.liveins(),
            liveouts: abi.liveouts(),
            vreg_types: vec![],
            insts: vec![],
            srclocs: vec![],
            entry: 0,
            block_ranges: vec![],
            block_succ_range: vec![],
            block_succs: vec![],
            block_by_bb: SecondaryMap::with_default(0),
            bb_by_block: vec![],
            final_block_order: vec![],
            final_block_offsets: vec![],
            code_size: 0,
            abi,
        }
    }

    /// Returns the flags controlling this function's compilation.
    pub fn flags(&self) -> &settings::Flags {
        self.abi.flags()
    }

    /// Get the IR-level type of a VReg.
    pub fn vreg_type(&self, vreg: VirtualReg) -> Type {
        self.vreg_types[vreg.get_index()]
    }

    /// Get the entry block.
    pub fn entry(&self) -> BlockIndex {
        self.entry
    }

    /// Get the number of blocks. Block indices will be in the range `0 ..
    /// (self.num_blocks() - 1)`.
    pub fn num_blocks(&self) -> usize {
        self.block_ranges.len()
    }

    /// Stack frame size for the full function's body.
    pub fn frame_size(&self) -> u32 {
        self.abi.frame_size()
    }

    /// Get the successors for a block.
    pub fn succs(&self, block: BlockIndex) -> &[BlockIndex] {
        let (start, end) = self.block_succ_range[block as usize];
        &self.block_succs[start..end]
    }

    /// Take the results of register allocation, with a sequence of
    /// instructions including spliced fill/reload/move instructions, and replace
    /// the VCode with them.
    pub fn replace_insns_from_regalloc(&mut self, result: RegAllocResult<Self>) {
        self.final_block_order = compute_final_block_order(self);

        // Record the spillslot count and clobbered registers for the ABI/stack
        // setup code.
        self.abi.set_num_spillslots(result.num_spill_slots as usize);
        self.abi
            .set_clobbered(result.clobbered_registers.map(|r| Writable::from_reg(*r)));

        // We want to move instructions over in final block order, using the new
        // block-start map given by the regalloc.
        let block_ranges: Vec<(usize, usize)> =
            block_ranges(result.target_map.elems(), result.insns.len());
        let mut final_insns = vec![];
        let mut final_block_ranges = vec![(0, 0); self.num_blocks()];
        let mut final_srclocs = vec![];

        for block in &self.final_block_order {
            let (start, end) = block_ranges[*block as usize];
            let final_start = final_insns.len() as InsnIndex;

            if *block == self.entry {
                // Start with the prologue.
                let prologue = self.abi.gen_prologue();
                let len = prologue.len();
                final_insns.extend(prologue.into_iter());
                final_srclocs.extend(iter::repeat(SourceLoc::default()).take(len));
            }

            for i in start..end {
                let insn = &result.insns[i];

                // Elide redundant moves at this point (we only know what is
                // redundant once registers are allocated).
                if is_redundant_move(insn) {
                    continue;
                }

                // Is there a srcloc associated with this insn? Look it up based on original
                // instruction index (if new insn corresponds to some original insn, i.e., is not
                // an inserted load/spill/move).
                let orig_iix = result.orig_insn_map[InstIx::new(i as u32)];
                let srcloc = if orig_iix.is_invalid() {
                    SourceLoc::default()
                } else {
                    self.srclocs[orig_iix.get() as usize]
                };

                // Whenever encountering a return instruction, replace it
                // with the epilogue.
                let is_ret = insn.is_term() == MachTerminator::Ret;
                if is_ret {
                    let epilogue = self.abi.gen_epilogue();
                    let len = epilogue.len();
                    final_insns.extend(epilogue.into_iter());
                    final_srclocs.extend(iter::repeat(srcloc).take(len));
                } else {
                    final_insns.push(insn.clone());
                    final_srclocs.push(srcloc);
                }
            }

            let final_end = final_insns.len() as InsnIndex;
            final_block_ranges[*block as usize] = (final_start, final_end);
        }

        debug_assert!(final_insns.len() == final_srclocs.len());

        self.insts = final_insns;
        self.srclocs = final_srclocs;
        self.block_ranges = final_block_ranges;
    }

    /// Removes redundant branches, rewriting targets to point directly to the
    /// ultimate block at the end of a chain of trivial one-target jumps.
    pub fn remove_redundant_branches(&mut self) {
        // For each block, compute the actual target block, looking through up to one
        // block with single-target jumps (this will remove empty edge blocks inserted
        // by phi-lowering).
        let block_rewrites: Vec<BlockIndex> = (0..self.num_blocks() as u32)
            .map(|bix| is_trivial_jump_block(self, bix).unwrap_or(bix))
            .collect();
        let mut refcounts: Vec<usize> = vec![0; self.num_blocks()];

        debug!(
            "remove_redundant_branches: block_rewrites = {:?}",
            block_rewrites
        );

        refcounts[self.entry as usize] = 1;

        for block in 0..self.num_blocks() as u32 {
            for insn in self.block_insns(BlockIx::new(block)) {
                self.get_insn_mut(insn)
                    .with_block_rewrites(&block_rewrites[..]);
                match self.get_insn(insn).is_term() {
                    MachTerminator::Uncond(bix) => {
                        refcounts[bix as usize] += 1;
                    }
                    MachTerminator::Cond(bix1, bix2) => {
                        refcounts[bix1 as usize] += 1;
                        refcounts[bix2 as usize] += 1;
                    }
                    MachTerminator::Indirect(blocks) => {
                        for block in blocks {
                            refcounts[*block as usize] += 1;
                        }
                    }
                    _ => {}
                }
            }
        }

        let deleted: Vec<bool> = refcounts.iter().map(|r| *r == 0).collect();

        let block_order = std::mem::replace(&mut self.final_block_order, vec![]);
        self.final_block_order = block_order
            .into_iter()
            .filter(|b| !deleted[*b as usize])
            .collect();

        // Rewrite successor information based on the block-rewrite map.
        for succ in &mut self.block_succs {
            let new_succ = block_rewrites[*succ as usize];
            *succ = new_succ;
        }
    }

    /// Mutate branch instructions to (i) lower two-way condbrs to one-way,
    /// depending on fallthrough; and (ii) use concrete offsets.
    pub fn finalize_branches(&mut self)
    where
        I: MachInstEmit<MachSectionSize>,
    {
        // Compute fallthrough block, indexed by block.
        let num_final_blocks = self.final_block_order.len();
        let mut block_fallthrough: Vec<Option<BlockIndex>> = vec![None; self.num_blocks()];
        for i in 0..(num_final_blocks - 1) {
            let from = self.final_block_order[i];
            let to = self.final_block_order[i + 1];
            block_fallthrough[from as usize] = Some(to);
        }

        // Pass over VCode instructions and finalize two-way branches into
        // one-way branches with fallthrough.
        for block in 0..self.num_blocks() {
            let next_block = block_fallthrough[block];
            let (start, end) = self.block_ranges[block];

            for iix in start..end {
                let insn = &mut self.insts[iix as usize];
                insn.with_fallthrough_block(next_block);
            }
        }

        let flags = self.abi.flags();

        // Compute block offsets.
        let mut code_section = MachSectionSize::new(0);
        let mut block_offsets = vec![0; self.num_blocks()];
        for &block in &self.final_block_order {
            code_section.offset = I::align_basic_block(code_section.offset);
            block_offsets[block as usize] = code_section.offset;
            let (start, end) = self.block_ranges[block as usize];
            for iix in start..end {
                self.insts[iix as usize].emit(&mut code_section, flags);
            }
        }

        // We now have the section layout.
        self.final_block_offsets = block_offsets;
        self.code_size = code_section.size();

        // Update branches with known block offsets. This looks like the
        // traversal above, but (i) does not update block_offsets, rather uses
        // it (so forward references are now possible), and (ii) mutates the
        // instructions.
        let mut code_section = MachSectionSize::new(0);
        for &block in &self.final_block_order {
            code_section.offset = I::align_basic_block(code_section.offset);
            let (start, end) = self.block_ranges[block as usize];
            for iix in start..end {
                self.insts[iix as usize]
                    .with_block_offsets(code_section.offset, &self.final_block_offsets[..]);
                self.insts[iix as usize].emit(&mut code_section, flags);
            }
        }
    }

    /// Emit the instructions to a list of sections.
    pub fn emit(&self) -> MachSections
    where
        I: MachInstEmit<MachSection>,
    {
        let mut sections = MachSections::new();
        let code_idx = sections.add_section(0, self.code_size);
        let code_section = sections.get_section(code_idx);

        let flags = self.abi.flags();
        let mut cur_srcloc = None;
        for &block in &self.final_block_order {
            let new_offset = I::align_basic_block(code_section.cur_offset_from_start());
            while new_offset > code_section.cur_offset_from_start() {
                // Pad with NOPs up to the aligned block offset.
                let nop = I::gen_nop((new_offset - code_section.cur_offset_from_start()) as usize);
                nop.emit(code_section, flags);
            }
            assert_eq!(code_section.cur_offset_from_start(), new_offset);

            let (start, end) = self.block_ranges[block as usize];
            for iix in start..end {
                let srcloc = self.srclocs[iix as usize];
                if cur_srcloc != Some(srcloc) {
                    if cur_srcloc.is_some() {
                        code_section.end_srcloc();
                    }
                    code_section.start_srcloc(srcloc);
                    cur_srcloc = Some(srcloc);
                }

                self.insts[iix as usize].emit(code_section, flags);
            }

            if cur_srcloc.is_some() {
                code_section.end_srcloc();
                cur_srcloc = None;
            }
        }

        sections
    }

    /// Get the IR block for a BlockIndex, if one exists.
    pub fn bindex_to_bb(&self, block: BlockIndex) -> Option<ir::Block> {
        if (block as usize) < self.bb_by_block.len() {
            Some(self.bb_by_block[block as usize])
        } else {
            None
        }
    }
}

impl<I: VCodeInst> RegallocFunction for VCode<I> {
    type Inst = I;

    fn insns(&self) -> &[I] {
        &self.insts[..]
    }

    fn insns_mut(&mut self) -> &mut [I] {
        &mut self.insts[..]
    }

    fn get_insn(&self, insn: InstIx) -> &I {
        &self.insts[insn.get() as usize]
    }

    fn get_insn_mut(&mut self, insn: InstIx) -> &mut I {
        &mut self.insts[insn.get() as usize]
    }

    fn blocks(&self) -> Range<BlockIx> {
        Range::new(BlockIx::new(0), self.block_ranges.len())
    }

    fn entry_block(&self) -> BlockIx {
        BlockIx::new(self.entry)
    }

    fn block_insns(&self, block: BlockIx) -> Range<InstIx> {
        let (start, end) = self.block_ranges[block.get() as usize];
        Range::new(InstIx::new(start), (end - start) as usize)
    }

    fn block_succs(&self, block: BlockIx) -> Vec<BlockIx> {
        let (start, end) = self.block_succ_range[block.get() as usize];
        self.block_succs[start..end]
            .iter()
            .cloned()
            .map(BlockIx::new)
            .collect()
    }

    fn is_ret(&self, insn: InstIx) -> bool {
        match self.insts[insn.get() as usize].is_term() {
            MachTerminator::Ret => true,
            _ => false,
        }
    }

    fn get_regs(insn: &I, collector: &mut RegUsageCollector) {
        insn.get_regs(collector)
    }

    fn map_regs(
        insn: &mut I,
        pre_map: &RegallocMap<VirtualReg, RealReg>,
        post_map: &RegallocMap<VirtualReg, RealReg>,
    ) {
        insn.map_regs(pre_map, post_map);
    }

    fn is_move(&self, insn: &I) -> Option<(Writable<Reg>, Reg)> {
        insn.is_move()
    }

    fn get_spillslot_size(&self, regclass: RegClass, vreg: VirtualReg) -> u32 {
        let ty = self.vreg_type(vreg);
        self.abi.get_spillslot_size(regclass, ty)
    }

    fn gen_spill(&self, to_slot: SpillSlot, from_reg: RealReg, vreg: VirtualReg) -> I {
        let ty = self.vreg_type(vreg);
        self.abi.gen_spill(to_slot, from_reg, ty)
    }

    fn gen_reload(&self, to_reg: Writable<RealReg>, from_slot: SpillSlot, vreg: VirtualReg) -> I {
        let ty = self.vreg_type(vreg);
        self.abi.gen_reload(to_reg, from_slot, ty)
    }

    fn gen_move(&self, to_reg: Writable<RealReg>, from_reg: RealReg, vreg: VirtualReg) -> I {
        let ty = self.vreg_type(vreg);
        I::gen_move(to_reg.map(|r| r.to_reg()), from_reg.to_reg(), ty)
    }

    fn gen_zero_len_nop(&self) -> I {
        I::gen_zero_len_nop()
    }

    fn maybe_direct_reload(&self, insn: &I, reg: VirtualReg, slot: SpillSlot) -> Option<I> {
        insn.maybe_direct_reload(reg, slot)
    }

    fn func_liveins(&self) -> RegallocSet<RealReg> {
        self.liveins.clone()
    }

    fn func_liveouts(&self) -> RegallocSet<RealReg> {
        self.liveouts.clone()
    }
}

impl<I: VCodeInst> fmt::Debug for VCode<I> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "VCode_Debug {{")?;
        writeln!(f, "  Entry block: {}", self.entry)?;
        writeln!(f, "  Final block order: {:?}", self.final_block_order)?;

        for block in 0..self.num_blocks() {
            writeln!(f, "Block {}:", block,)?;
            for succ in self.succs(block as BlockIndex) {
                writeln!(f, "  (successor: Block {})", succ)?;
            }
            let (start, end) = self.block_ranges[block];
            writeln!(f, "  (instruction range: {} .. {})", start, end)?;
            for inst in start..end {
                writeln!(f, "  Inst {}: {:?}", inst, self.insts[inst as usize])?;
            }
        }

        writeln!(f, "}}")?;
        Ok(())
    }
}

/// Pretty-printing with `RealRegUniverse` context.
impl<I: VCodeInst + ShowWithRRU> ShowWithRRU for VCode<I> {
    fn show_rru(&self, mb_rru: Option<&RealRegUniverse>) -> String {
        use std::fmt::Write;

        // Calculate an order in which to display the blocks.  This is the same
        // as final_block_order, but also includes blocks which are in the
        // representation but not in final_block_order.
        let mut display_order = Vec::<usize>::new();
        // First display blocks in `final_block_order`
        for bix in &self.final_block_order {
            assert!((*bix as usize) < self.num_blocks());
            display_order.push(*bix as usize);
        }
        // Now also take care of those not listed in `final_block_order`.
        // This is quadratic, but it's also debug-only code.
        for bix in 0..self.num_blocks() {
            if display_order.contains(&bix) {
                continue;
            }
            display_order.push(bix);
        }

        let mut s = String::new();
        write!(&mut s, "VCode_ShowWithRRU {{{{\n").unwrap();
        write!(&mut s, "  Entry block: {}\n", self.entry).unwrap();
        write!(
            &mut s,
            "  Final block order: {:?}\n",
            self.final_block_order
        )
        .unwrap();

        for i in 0..self.num_blocks() {
            let block = display_order[i];

            let omitted = if !self.final_block_order.is_empty() && i >= self.final_block_order.len()
            {
                "** OMITTED **"
            } else {
                ""
            };

            write!(&mut s, "Block {}: {}\n", block, omitted).unwrap();
            if let Some(bb) = self.bindex_to_bb(block as BlockIndex) {
                write!(&mut s, "  (original IR block: {})\n", bb).unwrap();
            }
            for succ in self.succs(block as BlockIndex) {
                write!(&mut s, "  (successor: Block {})\n", succ).unwrap();
            }
            let (start, end) = self.block_ranges[block];
            write!(&mut s, "  (instruction range: {} .. {})\n", start, end).unwrap();
            for inst in start..end {
                write!(
                    &mut s,
                    "  Inst {}:   {}\n",
                    inst,
                    self.insts[inst as usize].show_rru(mb_rru)
                )
                .unwrap();
            }
        }

        write!(&mut s, "}}}}\n").unwrap();

        s
    }
}
