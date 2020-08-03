//! Implementation of the standard x64 ABI.

use crate::binemit::Stackmap;
use crate::ir::{self, types, ArgumentExtension, StackSlot, Type};
use crate::isa::{x64::inst::*, CallConv};
use crate::machinst::*;
use crate::settings;
use crate::{CodegenError, CodegenResult};
use alloc::boxed::Box;
use alloc::vec::Vec;
use args::*;
use log::trace;
use regalloc::{RealReg, Reg, RegClass, Set, SpillSlot, Writable};
use std::mem;

/// This is the limit for the size of argument and return-value areas on the
/// stack. We place a reasonable limit here to avoid integer overflow issues
/// with 32-bit arithmetic: for now, 128 MB.
static STACK_ARG_RET_SIZE_LIMIT: u64 = 128 * 1024 * 1024;

#[derive(Clone, Debug)]
enum ABIArg {
    Reg(RealReg, ir::Type, ir::ArgumentExtension),
    Stack(i64, ir::Type, ir::ArgumentExtension),
}

/// X64 ABI information shared between body (callee) and caller.
struct ABISig {
    /// Argument locations (regs or stack slots). Stack offsets are relative to
    /// SP on entry to function.
    args: Vec<ABIArg>,
    /// Return-value locations. Stack offsets are relative to the return-area
    /// pointer.
    rets: Vec<ABIArg>,
    /// Space on stack used to store arguments.
    stack_arg_space: i64,
    /// Space on stack used to store return values.
    stack_ret_space: i64,
    /// Index in `args` of the stack-return-value-area argument.
    stack_ret_arg: Option<usize>,
    /// Calling convention used.
    call_conv: CallConv,
}

pub(crate) struct X64ABIBody {
    sig: ABISig,

    /// Offsets to each stack slot.
    stack_slots: Vec<usize>,

    /// Total stack size of all the stack slots.
    stack_slots_size: usize,

    /// The register holding the return-area pointer, if needed.
    ret_area_ptr: Option<Writable<Reg>>,

    /// Clobbered registers, as indicated by regalloc.
    clobbered: Set<Writable<RealReg>>,

    /// Total number of spill slots, as indicated by regalloc.
    num_spill_slots: Option<usize>,

    /// Calculated while creating the prologue, and used when creating the epilogue. Amount by
    /// which RSP is adjusted downwards to allocate the spill area.
    frame_size_bytes: Option<usize>,

    call_conv: CallConv,

    /// The settings controlling this function's compilation.
    flags: settings::Flags,
}

fn in_int_reg(ty: types::Type) -> bool {
    match ty {
        types::I8
        | types::I16
        | types::I32
        | types::I64
        | types::B1
        | types::B8
        | types::B16
        | types::B32
        | types::B64
        | types::R64 => true,
        types::R32 => panic!("unexpected 32-bits refs on x64!"),
        _ => false,
    }
}

fn in_vec_reg(ty: types::Type) -> bool {
    match ty {
        types::F32 | types::F64 => true,
        _ if ty.is_vector() => true,
        _ => false,
    }
}

fn get_intreg_for_arg_systemv(call_conv: &CallConv, idx: usize) -> Option<Reg> {
    match call_conv {
        CallConv::Fast | CallConv::Cold | CallConv::SystemV | CallConv::BaldrdashSystemV => {}
        _ => panic!("int args only supported for SysV calling convention"),
    };
    match idx {
        0 => Some(regs::rdi()),
        1 => Some(regs::rsi()),
        2 => Some(regs::rdx()),
        3 => Some(regs::rcx()),
        4 => Some(regs::r8()),
        5 => Some(regs::r9()),
        _ => None,
    }
}

fn get_fltreg_for_arg_systemv(call_conv: &CallConv, idx: usize) -> Option<Reg> {
    match call_conv {
        CallConv::Fast | CallConv::Cold | CallConv::SystemV | CallConv::BaldrdashSystemV => {}
        _ => panic!("float args only supported for SysV calling convention"),
    };
    match idx {
        0 => Some(regs::xmm0()),
        1 => Some(regs::xmm1()),
        2 => Some(regs::xmm2()),
        3 => Some(regs::xmm3()),
        4 => Some(regs::xmm4()),
        5 => Some(regs::xmm5()),
        6 => Some(regs::xmm6()),
        7 => Some(regs::xmm7()),
        _ => None,
    }
}

fn get_intreg_for_retval_systemv(
    call_conv: &CallConv,
    intreg_idx: usize,
    retval_idx: usize,
) -> Option<Reg> {
    match call_conv {
        CallConv::Fast | CallConv::Cold | CallConv::SystemV => match intreg_idx {
            0 => Some(regs::rax()),
            1 => Some(regs::rdx()),
            _ => None,
        },
        CallConv::BaldrdashSystemV => {
            if intreg_idx == 0 && retval_idx == 0 {
                Some(regs::rax())
            } else {
                None
            }
        }
        CallConv::WindowsFastcall | CallConv::BaldrdashWindows | CallConv::Probestack => todo!(),
    }
}

fn get_fltreg_for_retval_systemv(
    call_conv: &CallConv,
    fltreg_idx: usize,
    retval_idx: usize,
) -> Option<Reg> {
    match call_conv {
        CallConv::Fast | CallConv::Cold | CallConv::SystemV => match fltreg_idx {
            0 => Some(regs::xmm0()),
            1 => Some(regs::xmm1()),
            _ => None,
        },
        CallConv::BaldrdashSystemV => {
            if fltreg_idx == 0 && retval_idx == 0 {
                Some(regs::xmm0())
            } else {
                None
            }
        }
        CallConv::WindowsFastcall | CallConv::BaldrdashWindows | CallConv::Probestack => todo!(),
    }
}

fn is_callee_save_systemv(r: RealReg) -> bool {
    use regs::*;
    match r.get_class() {
        RegClass::I64 => match r.get_hw_encoding() as u8 {
            ENC_RBX | ENC_RBP | ENC_R12 | ENC_R13 | ENC_R14 | ENC_R15 => true,
            _ => false,
        },
        RegClass::V128 => false,
        _ => unimplemented!(),
    }
}

fn is_callee_save_baldrdash(r: RealReg) -> bool {
    use regs::*;
    match r.get_class() {
        RegClass::I64 => {
            if r.get_hw_encoding() as u8 == ENC_R14 {
                // r14 is the WasmTlsReg and is preserved implicitly.
                false
            } else {
                // Defer to native for the other ones.
                is_callee_save_systemv(r)
            }
        }
        RegClass::V128 => false,
        _ => unimplemented!(),
    }
}

fn get_callee_saves(call_conv: &CallConv, regs: Vec<Writable<RealReg>>) -> Vec<Writable<RealReg>> {
    match call_conv {
        CallConv::BaldrdashSystemV => regs
            .into_iter()
            .filter(|r| is_callee_save_baldrdash(r.to_reg()))
            .collect(),
        CallConv::BaldrdashWindows => {
            todo!("baldrdash windows");
        }
        CallConv::Fast | CallConv::Cold | CallConv::SystemV => regs
            .into_iter()
            .filter(|r| is_callee_save_systemv(r.to_reg()))
            .collect(),
        CallConv::WindowsFastcall => todo!("windows fastcall"),
        CallConv::Probestack => todo!("probestack?"),
    }
}

impl X64ABIBody {
    /// Create a new body ABI instance.
    pub(crate) fn new(f: &ir::Function, flags: settings::Flags) -> CodegenResult<Self> {
        let sig = ABISig::from_func_sig(&f.signature)?;

        let call_conv = f.signature.call_conv;
        debug_assert!(
            call_conv == CallConv::SystemV || call_conv.extends_baldrdash(),
            "unsupported or unimplemented calling convention {}",
            call_conv
        );

        // Compute stackslot locations and total stackslot size.
        let mut stack_offset: usize = 0;
        let mut stack_slots = vec![];
        for (stackslot, data) in f.stack_slots.iter() {
            let off = stack_offset;
            stack_offset += data.size as usize;
            stack_offset = (stack_offset + 7) & !7;
            debug_assert_eq!(stackslot.as_u32() as usize, stack_slots.len());
            stack_slots.push(off);
        }

        Ok(Self {
            sig,
            stack_slots,
            stack_slots_size: stack_offset,
            ret_area_ptr: None,
            clobbered: Set::empty(),
            num_spill_slots: None,
            frame_size_bytes: None,
            call_conv: f.signature.call_conv.clone(),
            flags,
        })
    }

    /// Returns the offset from FP to the argument area, i.e., jumping over the saved FP, return
    /// address, and maybe other standard elements depending on ABI (e.g. Wasm TLS reg).
    fn fp_to_arg_offset(&self) -> i64 {
        if self.call_conv.extends_baldrdash() {
            let num_words = self.flags.baldrdash_prologue_words() as i64;
            debug_assert!(num_words > 0, "baldrdash must set baldrdash_prologue_words");
            num_words * 8
        } else {
            16 // frame pointer + return address.
        }
    }
}

impl ABIBody for X64ABIBody {
    type I = Inst;

    fn temp_needed(&self) -> bool {
        self.sig.stack_ret_arg.is_some()
    }

    fn init(&mut self, maybe_tmp: Option<Writable<Reg>>) {
        if self.sig.stack_ret_arg.is_some() {
            assert!(maybe_tmp.is_some());
            self.ret_area_ptr = maybe_tmp;
        }
    }

    fn flags(&self) -> &settings::Flags {
        &self.flags
    }

    fn num_args(&self) -> usize {
        self.sig.args.len()
    }
    fn num_retvals(&self) -> usize {
        self.sig.rets.len()
    }
    fn num_stackslots(&self) -> usize {
        self.stack_slots.len()
    }

    fn liveins(&self) -> Set<RealReg> {
        let mut set: Set<RealReg> = Set::empty();
        for arg in &self.sig.args {
            if let &ABIArg::Reg(r, ..) = arg {
                set.insert(r);
            }
        }
        set
    }

    fn liveouts(&self) -> Set<RealReg> {
        let mut set: Set<RealReg> = Set::empty();
        for ret in &self.sig.rets {
            if let &ABIArg::Reg(r, ..) = ret {
                set.insert(r);
            }
        }
        set
    }

    fn gen_copy_arg_to_reg(&self, idx: usize, to_reg: Writable<Reg>) -> Inst {
        match &self.sig.args[idx] {
            ABIArg::Reg(from_reg, ty, _) => Inst::gen_move(to_reg, from_reg.to_reg(), *ty),
            &ABIArg::Stack(off, ty, _) => {
                assert!(
                    self.fp_to_arg_offset() + off <= u32::max_value() as i64,
                    "large offset nyi"
                );
                load_stack(
                    Amode::imm_reg((self.fp_to_arg_offset() + off) as u32, regs::rbp()),
                    to_reg,
                    ty,
                )
            }
        }
    }

    fn gen_retval_area_setup(&self) -> Option<Inst> {
        if let Some(i) = self.sig.stack_ret_arg {
            let inst = self.gen_copy_arg_to_reg(i, self.ret_area_ptr.unwrap());
            trace!(
                "gen_retval_area_setup: inst {:?}; ptr reg is {:?}",
                inst,
                self.ret_area_ptr.unwrap().to_reg()
            );
            Some(inst)
        } else {
            trace!("gen_retval_area_setup: not needed");
            None
        }
    }

    fn gen_copy_reg_to_retval(&self, idx: usize, from_reg: Writable<Reg>) -> Vec<Inst> {
        let mut ret = Vec::new();
        match &self.sig.rets[idx] {
            &ABIArg::Reg(r, ty, ext) => {
                let from_bits = ty.bits() as u8;
                let ext_mode = match from_bits {
                    1 | 8 => Some(ExtMode::BQ),
                    16 => Some(ExtMode::WQ),
                    32 => Some(ExtMode::LQ),
                    64 | 128 => None,
                    _ => unreachable!(),
                };

                let dest_reg = Writable::from_reg(r.to_reg());
                match (ext, ext_mode) {
                    (ArgumentExtension::Uext, Some(ext_mode)) => {
                        ret.push(Inst::movzx_rm_r(
                            ext_mode,
                            RegMem::reg(from_reg.to_reg()),
                            dest_reg,
                            /* infallible load */ None,
                        ));
                    }
                    (ArgumentExtension::Sext, Some(ext_mode)) => {
                        ret.push(Inst::movsx_rm_r(
                            ext_mode,
                            RegMem::reg(from_reg.to_reg()),
                            dest_reg,
                            /* infallible load */ None,
                        ));
                    }
                    _ => ret.push(Inst::gen_move(dest_reg, from_reg.to_reg(), ty)),
                };
            }

            &ABIArg::Stack(off, ty, ext) => {
                let from_bits = ty.bits() as u8;
                let ext_mode = match from_bits {
                    1 | 8 => Some(ExtMode::BQ),
                    16 => Some(ExtMode::WQ),
                    32 => Some(ExtMode::LQ),
                    64 => None,
                    _ => unreachable!(),
                };

                // Trash the from_reg; it should be its last use.
                match (ext, ext_mode) {
                    (ArgumentExtension::Uext, Some(ext_mode)) => {
                        ret.push(Inst::movzx_rm_r(
                            ext_mode,
                            RegMem::reg(from_reg.to_reg()),
                            from_reg,
                            /* infallible load */ None,
                        ));
                    }
                    (ArgumentExtension::Sext, Some(ext_mode)) => {
                        ret.push(Inst::movsx_rm_r(
                            ext_mode,
                            RegMem::reg(from_reg.to_reg()),
                            from_reg,
                            /* infallible load */ None,
                        ));
                    }
                    _ => {}
                };

                assert!(
                    off < u32::max_value() as i64,
                    "large stack return offset nyi"
                );

                let mem = Amode::imm_reg(off as u32, self.ret_area_ptr.unwrap().to_reg());
                ret.push(store_stack(mem, from_reg.to_reg(), ty))
            }
        }

        ret
    }

    fn gen_ret(&self) -> Inst {
        Inst::ret()
    }

    fn gen_epilogue_placeholder(&self) -> Inst {
        Inst::epilogue_placeholder()
    }

    fn set_num_spillslots(&mut self, slots: usize) {
        self.num_spill_slots = Some(slots);
    }

    fn set_clobbered(&mut self, clobbered: Set<Writable<RealReg>>) {
        self.clobbered = clobbered;
    }

    fn stackslot_addr(&self, slot: StackSlot, offset: u32, dst: Writable<Reg>) -> Inst {
        let stack_off = self.stack_slots[slot.as_u32() as usize] as i64;
        let sp_off: i64 = stack_off + (offset as i64);
        Inst::lea(SyntheticAmode::nominal_sp_offset(sp_off as u32), dst)
    }

    fn load_stackslot(
        &self,
        _slot: StackSlot,
        _offset: u32,
        _ty: Type,
        _into_reg: Writable<Reg>,
    ) -> Inst {
        unimplemented!("load_stackslot")
    }

    fn store_stackslot(&self, _slot: StackSlot, _offset: u32, _ty: Type, _from_reg: Reg) -> Inst {
        unimplemented!("store_stackslot")
    }

    fn load_spillslot(&self, slot: SpillSlot, ty: Type, into_reg: Writable<Reg>) -> Inst {
        // Offset from beginning of spillslot area, which is at nominal-SP + stackslots_size.
        let islot = slot.get() as i64;
        let spill_off = islot * 8;
        let sp_off = self.stack_slots_size as i64 + spill_off;
        debug_assert!(sp_off <= u32::max_value() as i64, "large spill offsets NYI");
        trace!("load_spillslot: slot {:?} -> sp_off {}", slot, sp_off);
        load_stack(
            SyntheticAmode::nominal_sp_offset(sp_off as u32),
            into_reg,
            ty,
        )
    }

    fn store_spillslot(&self, slot: SpillSlot, ty: Type, from_reg: Reg) -> Inst {
        // Offset from beginning of spillslot area, which is at nominal-SP + stackslots_size.
        let islot = slot.get() as i64;
        let spill_off = islot * 8;
        let sp_off = self.stack_slots_size as i64 + spill_off;
        debug_assert!(sp_off <= u32::max_value() as i64, "large spill offsets NYI");
        trace!("store_spillslot: slot {:?} -> sp_off {}", slot, sp_off);
        store_stack(
            SyntheticAmode::nominal_sp_offset(sp_off as u32),
            from_reg,
            ty,
        )
    }

    fn spillslots_to_stackmap(&self, slots: &[SpillSlot], state: &EmitState) -> Stackmap {
        assert!(state.virtual_sp_offset >= 0);
        trace!(
            "spillslots_to_stackmap: slots = {:?}, state = {:?}",
            slots,
            state
        );
        let map_size = (state.virtual_sp_offset + state.nominal_sp_to_fp) as u32;
        let map_words = (map_size + 7) / 8;
        let mut bits = std::iter::repeat(false)
            .take(map_words as usize)
            .collect::<Vec<bool>>();

        let first_spillslot_word = (self.stack_slots_size + state.virtual_sp_offset as usize) / 8;
        for &slot in slots {
            let slot = slot.get() as usize;
            bits[first_spillslot_word + slot] = true;
        }

        Stackmap::from_slice(&bits[..])
    }

    fn gen_prologue(&mut self) -> Vec<Inst> {
        let r_rsp = regs::rsp();

        let mut insts = vec![];

        // Baldrdash generates its own prologue sequence, so we don't have to.
        if !self.call_conv.extends_baldrdash() {
            let r_rbp = regs::rbp();
            let w_rbp = Writable::from_reg(r_rbp);

            // The "traditional" pre-preamble
            // RSP before the call will be 0 % 16.  So here, it is 8 % 16.
            insts.push(Inst::push64(RegMemImm::reg(r_rbp)));
            // RSP is now 0 % 16
            insts.push(Inst::mov_r_r(true, r_rsp, w_rbp));
        }

        let clobbered = get_callee_saves(&self.call_conv, self.clobbered.to_vec());
        let callee_saved_used: usize = clobbered
            .iter()
            .map(|reg| match reg.to_reg().get_class() {
                RegClass::I64 => 8,
                _ => todo!(),
            })
            .sum();

        let mut total_stacksize = self.stack_slots_size + 8 * self.num_spill_slots.unwrap();
        if self.call_conv.extends_baldrdash() {
            // Baldrdash expects the stack to take at least the number of words set in
            // baldrdash_prologue_words; count them here.
            debug_assert!(
                !self.flags.enable_probestack(),
                "baldrdash does not expect cranelift to emit stack probes"
            );
            total_stacksize += self.flags.baldrdash_prologue_words() as usize * 8;
        }

        // Now make sure the frame stack is aligned, so RSP == 0 % 16 in the function's body.
        let padding = (16 - ((total_stacksize + callee_saved_used) % 16)) & 15;
        let frame_size = total_stacksize + padding;
        debug_assert!(
            frame_size <= u32::max_value() as usize,
            "gen_prologue(x86): total_stacksize >= 2G"
        );
        debug_assert_eq!((frame_size + callee_saved_used) % 16, 0, "misaligned stack");

        if !self.call_conv.extends_baldrdash() {
            // Explicitly allocate the frame.
            let w_rsp = Writable::from_reg(r_rsp);
            if frame_size > 0 {
                insts.push(Inst::alu_rmi_r(
                    true,
                    AluRmiROpcode::Sub,
                    RegMemImm::imm(frame_size as u32),
                    w_rsp,
                ));
            }
        }

        // Save callee saved registers that we trash. Keep track of how much space we've used, so
        // as to know what we have to do to get the base of the spill area 0 % 16.
        let clobbered = get_callee_saves(&self.call_conv, self.clobbered.to_vec());
        for reg in clobbered {
            let r_reg = reg.to_reg();
            match r_reg.get_class() {
                RegClass::I64 => {
                    insts.push(Inst::push64(RegMemImm::reg(r_reg.to_reg())));
                }
                _ => unimplemented!(),
            }
        }

        if callee_saved_used > 0 {
            insts.push(Inst::VirtualSPOffsetAdj {
                offset: callee_saved_used as i64,
            });
        }

        // Stash this value.  We'll need it for the epilogue.
        debug_assert!(self.frame_size_bytes.is_none());
        self.frame_size_bytes = Some(frame_size);

        insts
    }

    fn gen_epilogue(&self) -> Vec<Inst> {
        let mut insts = vec![];

        // Undo what we did in the prologue.

        // Restore regs.
        let clobbered = get_callee_saves(&self.call_conv, self.clobbered.to_vec());
        for wreg in clobbered.into_iter().rev() {
            let rreg = wreg.to_reg();
            match rreg.get_class() {
                RegClass::I64 => {
                    // TODO: make these conversion sequences less cumbersome.
                    insts.push(Inst::pop64(Writable::from_reg(rreg.to_reg())));
                }
                _ => unimplemented!(),
            }
        }

        // No need to adjust the virtual sp offset here:
        // - this would create issues when there's a return in the middle of a function,
        // - and nothing in this sequence may try to access stack slots from the nominal SP.

        // Clear the spill area and the 16-alignment padding below it.
        if !self.call_conv.extends_baldrdash() {
            let frame_size = self.frame_size_bytes.unwrap();
            if frame_size > 0 {
                let r_rsp = regs::rsp();
                let w_rsp = Writable::from_reg(r_rsp);
                insts.push(Inst::alu_rmi_r(
                    true,
                    AluRmiROpcode::Add,
                    RegMemImm::imm(frame_size as u32),
                    w_rsp,
                ));
            }
        }

        // Baldrdash generates its own preamble.
        if !self.call_conv.extends_baldrdash() {
            // Undo the "traditional" pre-preamble
            // RSP before the call will be 0 % 16.  So here, it is 8 % 16.
            insts.push(Inst::pop64(Writable::from_reg(regs::rbp())));
            insts.push(Inst::ret());
        }

        insts
    }

    fn frame_size(&self) -> u32 {
        self.frame_size_bytes
            .expect("frame size not computed before prologue generation") as u32
    }

    fn stack_args_size(&self) -> u32 {
        unimplemented!("I need to be computed!")
    }

    fn get_spillslot_size(&self, rc: RegClass, ty: Type) -> u32 {
        // We allocate in terms of 8-byte slots.
        match (rc, ty) {
            (RegClass::I64, _) => 1,
            (RegClass::V128, types::F32) | (RegClass::V128, types::F64) => 1,
            (RegClass::V128, _) => 2,
            _ => panic!("Unexpected register class!"),
        }
    }

    fn gen_spill(&self, to_slot: SpillSlot, from_reg: RealReg, ty: Option<Type>) -> Inst {
        let ty = ty_from_ty_hint_or_reg_class(from_reg.to_reg(), ty);
        self.store_spillslot(to_slot, ty, from_reg.to_reg())
    }

    fn gen_reload(
        &self,
        to_reg: Writable<RealReg>,
        from_slot: SpillSlot,
        ty: Option<Type>,
    ) -> Inst {
        let ty = ty_from_ty_hint_or_reg_class(to_reg.to_reg().to_reg(), ty);
        self.load_spillslot(from_slot, ty, to_reg.map(|r| r.to_reg()))
    }
}

/// Return a type either from an optional type hint, or if not, from the default
/// type associated with the given register's class. This is used to generate
/// loads/spills appropriately given the type of value loaded/stored (which may
/// be narrower than the spillslot). We usually have the type because the
/// regalloc usually provides the vreg being spilled/reloaded, and we know every
/// vreg's type. However, the regalloc *can* request a spill/reload without an
/// associated vreg when needed to satisfy a safepoint (which requires all
/// ref-typed values, even those in real registers in the original vcode, to be
/// in spillslots).
fn ty_from_ty_hint_or_reg_class(r: Reg, ty: Option<Type>) -> Type {
    match (ty, r.get_class()) {
        // If the type is provided
        (Some(t), _) => t,
        // If no type is provided, this should be a register spill for a
        // safepoint, so we only expect I64 (integer) registers.
        (None, RegClass::I64) => types::I64,
        _ => panic!("Unexpected register class!"),
    }
}

fn get_caller_saves(call_conv: CallConv) -> Vec<Writable<Reg>> {
    let mut caller_saved = Vec::new();

    // Systemv calling convention:
    // - GPR: all except RBX, RBP, R12 to R15 (which are callee-saved).
    caller_saved.push(Writable::from_reg(regs::rsi()));
    caller_saved.push(Writable::from_reg(regs::rdi()));
    caller_saved.push(Writable::from_reg(regs::rax()));
    caller_saved.push(Writable::from_reg(regs::rcx()));
    caller_saved.push(Writable::from_reg(regs::rdx()));
    caller_saved.push(Writable::from_reg(regs::r8()));
    caller_saved.push(Writable::from_reg(regs::r9()));
    caller_saved.push(Writable::from_reg(regs::r10()));
    caller_saved.push(Writable::from_reg(regs::r11()));

    if call_conv.extends_baldrdash() {
        caller_saved.push(Writable::from_reg(regs::r12()));
        caller_saved.push(Writable::from_reg(regs::r13()));
        // Not r14; implicitly preserved in the entry.
        caller_saved.push(Writable::from_reg(regs::r15()));
        caller_saved.push(Writable::from_reg(regs::rbx()));
    }

    // - XMM: all the registers!
    caller_saved.push(Writable::from_reg(regs::xmm0()));
    caller_saved.push(Writable::from_reg(regs::xmm1()));
    caller_saved.push(Writable::from_reg(regs::xmm2()));
    caller_saved.push(Writable::from_reg(regs::xmm3()));
    caller_saved.push(Writable::from_reg(regs::xmm4()));
    caller_saved.push(Writable::from_reg(regs::xmm5()));
    caller_saved.push(Writable::from_reg(regs::xmm6()));
    caller_saved.push(Writable::from_reg(regs::xmm7()));
    caller_saved.push(Writable::from_reg(regs::xmm8()));
    caller_saved.push(Writable::from_reg(regs::xmm9()));
    caller_saved.push(Writable::from_reg(regs::xmm10()));
    caller_saved.push(Writable::from_reg(regs::xmm11()));
    caller_saved.push(Writable::from_reg(regs::xmm12()));
    caller_saved.push(Writable::from_reg(regs::xmm13()));
    caller_saved.push(Writable::from_reg(regs::xmm14()));
    caller_saved.push(Writable::from_reg(regs::xmm15()));

    caller_saved
}

fn abisig_to_uses_and_defs(sig: &ABISig) -> (Vec<Reg>, Vec<Writable<Reg>>) {
    // Compute uses: all arg regs.
    let mut uses = Vec::new();
    for arg in &sig.args {
        match arg {
            &ABIArg::Reg(reg, ..) => uses.push(reg.to_reg()),
            _ => {}
        }
    }

    // Compute defs: all retval regs, and all caller-save (clobbered) regs.
    let mut defs = get_caller_saves(sig.call_conv);
    for ret in &sig.rets {
        match ret {
            &ABIArg::Reg(reg, ..) => defs.push(Writable::from_reg(reg.to_reg())),
            _ => {}
        }
    }

    (uses, defs)
}

/// Try to fill a Baldrdash register, returning it if it was found.
fn try_fill_baldrdash_reg(call_conv: CallConv, param: &ir::AbiParam) -> Option<ABIArg> {
    if call_conv.extends_baldrdash() {
        match &param.purpose {
            &ir::ArgumentPurpose::VMContext => {
                // This is SpiderMonkey's `WasmTlsReg`.
                Some(ABIArg::Reg(
                    regs::r14().to_real_reg(),
                    types::I64,
                    param.extension,
                ))
            }
            &ir::ArgumentPurpose::SignatureId => {
                // This is SpiderMonkey's `WasmTableCallSigReg`.
                Some(ABIArg::Reg(
                    regs::r10().to_real_reg(),
                    types::I64,
                    param.extension,
                ))
            }
            _ => None,
        }
    } else {
        None
    }
}

/// Are we computing information about arguments or return values? Much of the
/// handling is factored out into common routines; this enum allows us to
/// distinguish which case we're handling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ArgsOrRets {
    Args,
    Rets,
}

/// Process a list of parameters or return values and allocate them to X-regs,
/// V-regs, and stack slots.
///
/// Returns the list of argument locations, the stack-space used (rounded up
/// to a 16-byte-aligned boundary), and if `add_ret_area_ptr` was passed, the
/// index of the extra synthetic arg that was added.
fn compute_arg_locs(
    call_conv: CallConv,
    params: &[ir::AbiParam],
    args_or_rets: ArgsOrRets,
    add_ret_area_ptr: bool,
) -> CodegenResult<(Vec<ABIArg>, i64, Option<usize>)> {
    let is_baldrdash = call_conv.extends_baldrdash();

    let mut next_gpr = 0;
    let mut next_vreg = 0;
    let mut next_stack: u64 = 0;
    let mut ret = vec![];

    for i in 0..params.len() {
        // Process returns backward, according to the SpiderMonkey ABI (which we
        // adopt internally if `is_baldrdash` is set).
        let param = match (args_or_rets, is_baldrdash) {
            (ArgsOrRets::Args, _) => &params[i],
            (ArgsOrRets::Rets, false) => &params[i],
            (ArgsOrRets::Rets, true) => &params[params.len() - 1 - i],
        };

        // Validate "purpose".
        match &param.purpose {
            &ir::ArgumentPurpose::VMContext
            | &ir::ArgumentPurpose::Normal
            | &ir::ArgumentPurpose::StackLimit
            | &ir::ArgumentPurpose::SignatureId => {}
            _ => panic!(
                "Unsupported argument purpose {:?} in signature: {:?}",
                param.purpose, params
            ),
        }

        let intreg = in_int_reg(param.value_type);
        let vecreg = in_vec_reg(param.value_type);
        debug_assert!(intreg || vecreg);
        debug_assert!(!(intreg && vecreg));

        let (next_reg, candidate) = if intreg {
            let candidate = match args_or_rets {
                ArgsOrRets::Args => get_intreg_for_arg_systemv(&call_conv, next_gpr),
                ArgsOrRets::Rets => get_intreg_for_retval_systemv(&call_conv, next_gpr, i),
            };
            debug_assert!(candidate
                .map(|r| r.get_class() == RegClass::I64)
                .unwrap_or(true));
            (&mut next_gpr, candidate)
        } else {
            let candidate = match args_or_rets {
                ArgsOrRets::Args => get_fltreg_for_arg_systemv(&call_conv, next_vreg),
                ArgsOrRets::Rets => get_fltreg_for_retval_systemv(&call_conv, next_vreg, i),
            };
            debug_assert!(candidate
                .map(|r| r.get_class() == RegClass::V128)
                .unwrap_or(true));
            (&mut next_vreg, candidate)
        };

        if let Some(param) = try_fill_baldrdash_reg(call_conv, param) {
            assert!(intreg);
            ret.push(param);
        } else if let Some(reg) = candidate {
            ret.push(ABIArg::Reg(
                reg.to_real_reg(),
                param.value_type,
                param.extension,
            ));
            *next_reg += 1;
        } else {
            // Compute size. Every arg takes a minimum slot of 8 bytes. (16-byte
            // stack alignment happens separately after all args.)
            let size = (param.value_type.bits() / 8) as u64;
            let size = std::cmp::max(size, 8);
            // Align.
            debug_assert!(size.is_power_of_two());
            next_stack = (next_stack + size - 1) & !(size - 1);
            ret.push(ABIArg::Stack(
                next_stack as i64,
                param.value_type,
                param.extension,
            ));
            next_stack += size;
        }
    }

    if args_or_rets == ArgsOrRets::Rets && is_baldrdash {
        ret.reverse();
    }

    let extra_arg = if add_ret_area_ptr {
        debug_assert!(args_or_rets == ArgsOrRets::Args);
        if let Some(reg) = get_intreg_for_arg_systemv(&call_conv, next_gpr) {
            ret.push(ABIArg::Reg(
                reg.to_real_reg(),
                types::I64,
                ir::ArgumentExtension::None,
            ));
        } else {
            ret.push(ABIArg::Stack(
                next_stack as i64,
                types::I64,
                ir::ArgumentExtension::None,
            ));
            next_stack += 8;
        }
        Some(ret.len() - 1)
    } else {
        None
    };

    next_stack = (next_stack + 15) & !15;

    // To avoid overflow issues, limit the arg/return size to something reasonable.
    if next_stack > STACK_ARG_RET_SIZE_LIMIT {
        return Err(CodegenError::ImplLimitExceeded);
    }

    Ok((ret, next_stack as i64, extra_arg))
}

impl ABISig {
    fn from_func_sig(sig: &ir::Signature) -> CodegenResult<ABISig> {
        // Compute args and retvals from signature. Handle retvals first,
        // because we may need to add a return-area arg to the args.
        let (rets, stack_ret_space, _) = compute_arg_locs(
            sig.call_conv,
            &sig.returns,
            ArgsOrRets::Rets,
            /* extra ret-area ptr = */ false,
        )?;
        let need_stack_return_area = stack_ret_space > 0;
        let (args, stack_arg_space, stack_ret_arg) = compute_arg_locs(
            sig.call_conv,
            &sig.params,
            ArgsOrRets::Args,
            need_stack_return_area,
        )?;

        trace!(
            "ABISig: sig {:?} => args = {:?} rets = {:?} arg stack = {} ret stack = {} stack_ret_arg = {:?}",
            sig,
            args,
            rets,
            stack_arg_space,
            stack_ret_space,
            stack_ret_arg
        );

        Ok(ABISig {
            args,
            rets,
            stack_arg_space,
            stack_ret_space,
            stack_ret_arg,
            call_conv: sig.call_conv,
        })
    }
}

enum CallDest {
    ExtName(ir::ExternalName, RelocDistance),
    Reg(Reg),
}

fn adjust_stack<C: LowerCtx<I = Inst>>(ctx: &mut C, amount: u64, is_sub: bool) {
    if amount == 0 {
        return;
    }

    let (alu_op, sp_adjustment) = if is_sub {
        (AluRmiROpcode::Sub, amount as i64)
    } else {
        (AluRmiROpcode::Add, -(amount as i64))
    };

    ctx.emit(Inst::VirtualSPOffsetAdj {
        offset: sp_adjustment,
    });

    if amount <= u32::max_value() as u64 {
        ctx.emit(Inst::alu_rmi_r(
            true,
            alu_op,
            RegMemImm::imm(amount as u32),
            Writable::from_reg(regs::rsp()),
        ));
    } else {
        // TODO will require a scratch register.
        unimplemented!("adjust stack with large offset");
    }
}

fn load_stack(mem: impl Into<SyntheticAmode>, into_reg: Writable<Reg>, ty: Type) -> Inst {
    let (is_int, ext_mode) = match ty {
        types::B1 | types::B8 | types::I8 => (true, Some(ExtMode::BQ)),
        types::B16 | types::I16 => (true, Some(ExtMode::WQ)),
        types::B32 | types::I32 => (true, Some(ExtMode::LQ)),
        types::B64 | types::I64 | types::R64 => (true, None),
        types::F32 | types::F64 => (false, None),
        _ => panic!("load_stack({})", ty),
    };

    let mem = mem.into();

    if is_int {
        match ext_mode {
            Some(ext_mode) => Inst::movsx_rm_r(
                ext_mode,
                RegMem::mem(mem),
                into_reg,
                /* infallible load */ None,
            ),
            None => Inst::mov64_m_r(mem, into_reg, None /* infallible */),
        }
    } else {
        let sse_op = match ty {
            types::F32 => SseOpcode::Movss,
            types::F64 => SseOpcode::Movsd,
            _ => unreachable!(),
        };
        Inst::xmm_mov(
            sse_op,
            RegMem::mem(mem),
            into_reg,
            None, /* infallible */
        )
    }
}

fn store_stack(mem: impl Into<SyntheticAmode>, from_reg: Reg, ty: Type) -> Inst {
    let (is_int, size) = match ty {
        types::B1 | types::B8 | types::I8 => (true, 1),
        types::B16 | types::I16 => (true, 2),
        types::B32 | types::I32 => (true, 4),
        types::B64 | types::I64 | types::R64 => (true, 8),
        types::F32 => (false, 4),
        types::F64 => (false, 8),
        _ => unimplemented!("store_stack({})", ty),
    };
    let mem = mem.into();
    if is_int {
        Inst::mov_r_m(size, from_reg, mem, /* infallible store */ None)
    } else {
        let sse_op = match size {
            4 => SseOpcode::Movss,
            8 => SseOpcode::Movsd,
            _ => unreachable!(),
        };
        Inst::xmm_mov_r_m(sse_op, from_reg, mem, /* infallible store */ None)
    }
}

/// X64 ABI object for a function call.
pub struct X64ABICall {
    sig: ABISig,
    uses: Vec<Reg>,
    defs: Vec<Writable<Reg>>,
    dest: CallDest,
    loc: ir::SourceLoc,
    opcode: ir::Opcode,
}

impl X64ABICall {
    /// Create a callsite ABI object for a call directly to the specified function.
    pub fn from_func(
        sig: &ir::Signature,
        extname: &ir::ExternalName,
        dist: RelocDistance,
        loc: ir::SourceLoc,
    ) -> CodegenResult<Self> {
        let sig = ABISig::from_func_sig(sig)?;
        let (uses, defs) = abisig_to_uses_and_defs(&sig);
        Ok(Self {
            sig,
            uses,
            defs,
            dest: CallDest::ExtName(extname.clone(), dist),
            loc,
            opcode: ir::Opcode::Call,
        })
    }

    /// Create a callsite ABI object for a call to a function pointer with the
    /// given signature.
    pub fn from_ptr(
        sig: &ir::Signature,
        ptr: Reg,
        loc: ir::SourceLoc,
        opcode: ir::Opcode,
    ) -> CodegenResult<Self> {
        let sig = ABISig::from_func_sig(sig)?;
        let (uses, defs) = abisig_to_uses_and_defs(&sig);
        Ok(Self {
            sig,
            uses,
            defs,
            dest: CallDest::Reg(ptr),
            loc,
            opcode,
        })
    }
}

impl ABICall for X64ABICall {
    type I = Inst;

    fn num_args(&self) -> usize {
        if self.sig.stack_ret_arg.is_some() {
            self.sig.args.len() - 1
        } else {
            self.sig.args.len()
        }
    }

    fn emit_stack_pre_adjust<C: LowerCtx<I = Self::I>>(&self, ctx: &mut C) {
        let off = self.sig.stack_arg_space + self.sig.stack_ret_space;
        adjust_stack(ctx, off as u64, /* is_sub = */ true)
    }

    fn emit_stack_post_adjust<C: LowerCtx<I = Self::I>>(&self, ctx: &mut C) {
        let off = self.sig.stack_arg_space + self.sig.stack_ret_space;
        adjust_stack(ctx, off as u64, /* is_sub = */ false)
    }

    fn emit_copy_reg_to_arg<C: LowerCtx<I = Self::I>>(
        &self,
        ctx: &mut C,
        idx: usize,
        from_reg: Reg,
    ) {
        match &self.sig.args[idx] {
            &ABIArg::Reg(reg, ty, ext) if ext != ir::ArgumentExtension::None && ty.bits() < 64 => {
                assert_eq!(RegClass::I64, reg.get_class());
                let dest_reg = Writable::from_reg(reg.to_reg());
                let ext_mode = match ty.bits() {
                    1 | 8 => ExtMode::BQ,
                    16 => ExtMode::WQ,
                    32 => ExtMode::LQ,
                    _ => unreachable!(),
                };
                match ext {
                    ir::ArgumentExtension::Uext => {
                        ctx.emit(Inst::movzx_rm_r(
                            ext_mode,
                            RegMem::reg(from_reg),
                            dest_reg,
                            /* infallible load */ None,
                        ));
                    }
                    ir::ArgumentExtension::Sext => {
                        ctx.emit(Inst::movsx_rm_r(
                            ext_mode,
                            RegMem::reg(from_reg),
                            dest_reg,
                            /* infallible load */ None,
                        ));
                    }
                    _ => unreachable!(),
                };
            }
            &ABIArg::Reg(reg, ty, _) => ctx.emit(Inst::gen_move(
                Writable::from_reg(reg.to_reg()),
                from_reg,
                ty,
            )),
            &ABIArg::Stack(off, ty, ext) => {
                if ext != ir::ArgumentExtension::None && ty.bits() < 64 {
                    assert_eq!(RegClass::I64, from_reg.get_class());
                    let dest_reg = Writable::from_reg(from_reg);
                    let ext_mode = match ty.bits() {
                        1 | 8 => ExtMode::BQ,
                        16 => ExtMode::WQ,
                        32 => ExtMode::LQ,
                        _ => unreachable!(),
                    };
                    // Extend in place in the source register. Our convention is to
                    // treat high bits as undefined for values in registers, so this
                    // is safe, even for an argument that is nominally read-only.
                    match ext {
                        ir::ArgumentExtension::Uext => {
                            ctx.emit(Inst::movzx_rm_r(
                                ext_mode,
                                RegMem::reg(from_reg),
                                dest_reg,
                                /* infallible load */ None,
                            ));
                        }
                        ir::ArgumentExtension::Sext => {
                            ctx.emit(Inst::movsx_rm_r(
                                ext_mode,
                                RegMem::reg(from_reg),
                                dest_reg,
                                /* infallible load */ None,
                            ));
                        }
                        _ => unreachable!(),
                    };
                }

                debug_assert!(off <= u32::max_value() as i64);
                debug_assert!(off >= 0);
                ctx.emit(store_stack(
                    Amode::imm_reg(off as u32, regs::rsp()),
                    from_reg,
                    ty,
                ))
            }
        }
    }

    fn emit_copy_retval_to_reg<C: LowerCtx<I = Self::I>>(
        &self,
        ctx: &mut C,
        idx: usize,
        into_reg: Writable<Reg>,
    ) {
        match &self.sig.rets[idx] {
            &ABIArg::Reg(reg, ty, _) => ctx.emit(Inst::gen_move(into_reg, reg.to_reg(), ty)),
            &ABIArg::Stack(off, ty, _) => {
                let ret_area_base = self.sig.stack_arg_space;
                let sp_offset = off + ret_area_base;
                // TODO handle offsets bigger than u32::max
                debug_assert!(sp_offset >= 0);
                debug_assert!(sp_offset <= u32::max_value() as i64);
                ctx.emit(load_stack(
                    Amode::imm_reg(sp_offset as u32, regs::rsp()),
                    into_reg,
                    ty,
                ));
            }
        }
    }

    fn emit_call<C: LowerCtx<I = Self::I>>(&mut self, ctx: &mut C) {
        let (uses, defs) = (
            mem::replace(&mut self.uses, Default::default()),
            mem::replace(&mut self.defs, Default::default()),
        );

        if let Some(i) = self.sig.stack_ret_arg {
            let dst = ctx.alloc_tmp(RegClass::I64, types::I64);
            let ret_area_base = self.sig.stack_arg_space;
            debug_assert!(
                ret_area_base <= u32::max_value() as i64,
                "large offset for ret area NYI"
            );
            ctx.emit(Inst::lea(
                Amode::imm_reg(ret_area_base as u32, regs::rsp()),
                dst,
            ));
            self.emit_copy_reg_to_arg(ctx, i, dst.to_reg());
        }

        match &self.dest {
            &CallDest::ExtName(ref name, RelocDistance::Near) => ctx.emit_safepoint(
                Inst::call_known(name.clone(), uses, defs, self.loc, self.opcode),
            ),
            &CallDest::ExtName(ref name, RelocDistance::Far) => {
                let tmp = ctx.alloc_tmp(RegClass::I64, types::I64);
                ctx.emit(Inst::LoadExtName {
                    dst: tmp,
                    name: Box::new(name.clone()),
                    offset: 0,
                    srcloc: self.loc,
                });
                ctx.emit_safepoint(Inst::call_unknown(
                    RegMem::reg(tmp.to_reg()),
                    uses,
                    defs,
                    self.loc,
                    self.opcode,
                ));
            }
            &CallDest::Reg(reg) => ctx.emit_safepoint(Inst::call_unknown(
                RegMem::reg(reg),
                uses,
                defs,
                self.loc,
                self.opcode,
            )),
        }
    }
}
