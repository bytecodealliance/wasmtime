//! Assembler library implementation for x64.

use crate::{
    isa::{reg::Reg, CallingConvention},
    masm::{
        DivKind, Extend, ExtendKind, ExtendType, IntCmpKind, MulWideKind, OperandSize, RemKind,
        RoundingMode, ShiftKind, Signed, V128ExtendKind, V128LoadExtendKind, Zero,
    },
    reg::writable,
    x64::regs::scratch,
};
use cranelift_codegen::{
    ir::{
        types, ConstantPool, ExternalName, LibCall, MemFlags, SourceLoc, TrapCode, Type,
        UserExternalNameRef,
    },
    isa::{
        unwind::UnwindInst,
        x64::{
            args::{
                self, AluRmiROpcode, Amode, Avx512Opcode, AvxOpcode, CmpOpcode, DivSignedness,
                ExtMode, FenceKind, FromWritableReg, Gpr, GprMem, GprMemImm, Imm8Gpr, Imm8Reg,
                RegMem, RegMemImm, ShiftKind as CraneliftShiftKind, SseOpcode, SyntheticAmode,
                WritableGpr, WritableXmm, Xmm, XmmMem, XmmMemAligned, XmmMemImm, CC,
            },
            encoding::rex::{encode_modrm, RexFlags},
            settings as x64_settings, AtomicRmwSeqOp, EmitInfo, EmitState, Inst,
        },
    },
    settings, CallInfo, Final, MachBuffer, MachBufferFinalized, MachInstEmit, MachInstEmitState,
    MachLabel, PatchRegion, RelocDistance, VCodeConstantData, VCodeConstants, Writable,
};

use crate::reg::WritableReg;

use super::address::Address;
use smallvec::SmallVec;

// Conversions between winch-codegen x64 types and cranelift-codegen x64 types.

impl From<Reg> for RegMemImm {
    fn from(reg: Reg) -> Self {
        RegMemImm::reg(reg.into())
    }
}

impl From<Reg> for RegMem {
    fn from(value: Reg) -> Self {
        RegMem::Reg { reg: value.into() }
    }
}

impl From<Reg> for WritableGpr {
    fn from(reg: Reg) -> Self {
        let writable = Writable::from_reg(reg.into());
        WritableGpr::from_writable_reg(writable).expect("valid writable gpr")
    }
}

impl From<Reg> for WritableXmm {
    fn from(reg: Reg) -> Self {
        let writable = Writable::from_reg(reg.into());
        WritableXmm::from_writable_reg(writable).expect("valid writable xmm")
    }
}

impl From<Reg> for Gpr {
    fn from(reg: Reg) -> Self {
        Gpr::unwrap_new(reg.into())
    }
}

impl From<Reg> for GprMem {
    fn from(value: Reg) -> Self {
        GprMem::unwrap_new(value.into())
    }
}

impl From<Reg> for GprMemImm {
    fn from(reg: Reg) -> Self {
        GprMemImm::unwrap_new(reg.into())
    }
}

impl From<Reg> for Imm8Gpr {
    fn from(value: Reg) -> Self {
        Imm8Gpr::unwrap_new(Imm8Reg::Reg { reg: value.into() })
    }
}

impl From<Reg> for Xmm {
    fn from(reg: Reg) -> Self {
        Xmm::unwrap_new(reg.into())
    }
}

impl From<Reg> for XmmMem {
    fn from(value: Reg) -> Self {
        XmmMem::unwrap_new(value.into())
    }
}

impl From<Reg> for XmmMemImm {
    fn from(value: Reg) -> Self {
        XmmMemImm::unwrap_new(value.into())
    }
}

impl From<OperandSize> for args::OperandSize {
    fn from(size: OperandSize) -> Self {
        match size {
            OperandSize::S8 => Self::Size8,
            OperandSize::S16 => Self::Size16,
            OperandSize::S32 => Self::Size32,
            OperandSize::S64 => Self::Size64,
            s => panic!("Invalid operand size {s:?}"),
        }
    }
}

impl From<DivKind> for DivSignedness {
    fn from(kind: DivKind) -> DivSignedness {
        match kind {
            DivKind::Signed => DivSignedness::Signed,
            DivKind::Unsigned => DivSignedness::Unsigned,
        }
    }
}

impl From<IntCmpKind> for CC {
    fn from(value: IntCmpKind) -> Self {
        match value {
            IntCmpKind::Eq => CC::Z,
            IntCmpKind::Ne => CC::NZ,
            IntCmpKind::LtS => CC::L,
            IntCmpKind::LtU => CC::B,
            IntCmpKind::GtS => CC::NLE,
            IntCmpKind::GtU => CC::NBE,
            IntCmpKind::LeS => CC::LE,
            IntCmpKind::LeU => CC::BE,
            IntCmpKind::GeS => CC::NL,
            IntCmpKind::GeU => CC::NB,
        }
    }
}

impl From<ShiftKind> for CraneliftShiftKind {
    fn from(value: ShiftKind) -> Self {
        match value {
            ShiftKind::Shl => CraneliftShiftKind::ShiftLeft,
            ShiftKind::ShrS => CraneliftShiftKind::ShiftRightArithmetic,
            ShiftKind::ShrU => CraneliftShiftKind::ShiftRightLogical,
            ShiftKind::Rotl => CraneliftShiftKind::RotateLeft,
            ShiftKind::Rotr => CraneliftShiftKind::RotateRight,
        }
    }
}

impl<T: ExtendType> From<Extend<T>> for ExtMode {
    fn from(value: Extend<T>) -> Self {
        match value {
            Extend::I32Extend8 => ExtMode::BL,
            Extend::I32Extend16 => ExtMode::WL,
            Extend::I64Extend8 => ExtMode::BQ,
            Extend::I64Extend16 => ExtMode::WQ,
            Extend::I64Extend32 => ExtMode::LQ,
            Extend::__Kind(_) => unreachable!(),
        }
    }
}

impl From<ExtendKind> for ExtMode {
    fn from(value: ExtendKind) -> Self {
        match value {
            ExtendKind::Signed(s) => s.into(),
            ExtendKind::Unsigned(u) => u.into(),
        }
    }
}

/// Kinds of extends supported by `vpmov`.
pub(super) enum VpmovKind {
    /// Sign extends 8 lanes of 8-bit integers to 8 lanes of 16-bit integers.
    E8x8S,
    /// Zero extends 8 lanes of 8-bit integers to 8 lanes of 16-bit integers.
    E8x8U,
    /// Sign extends 4 lanes of 16-bit integers to 4 lanes of 32-bit integers.
    E16x4S,
    /// Zero extends 4 lanes of 16-bit integers to 4 lanes of 32-bit integers.
    E16x4U,
    /// Sign extends 2 lanes of 32-bit integers to 2 lanes of 64-bit integers.
    E32x2S,
    /// Zero extends 2 lanes of 32-bit integers to 2 lanes of 64-bit integers.
    E32x2U,
}

impl From<VpmovKind> for AvxOpcode {
    fn from(value: VpmovKind) -> Self {
        match value {
            VpmovKind::E8x8S => AvxOpcode::Vpmovsxbw,
            VpmovKind::E8x8U => AvxOpcode::Vpmovzxbw,
            VpmovKind::E16x4S => AvxOpcode::Vpmovsxwd,
            VpmovKind::E16x4U => AvxOpcode::Vpmovzxwd,
            VpmovKind::E32x2S => AvxOpcode::Vpmovsxdq,
            VpmovKind::E32x2U => AvxOpcode::Vpmovzxdq,
        }
    }
}

impl From<V128LoadExtendKind> for VpmovKind {
    fn from(value: V128LoadExtendKind) -> Self {
        match value {
            V128LoadExtendKind::E8x8S => Self::E8x8S,
            V128LoadExtendKind::E8x8U => Self::E8x8U,
            V128LoadExtendKind::E16x4S => Self::E16x4S,
            V128LoadExtendKind::E16x4U => Self::E16x4U,
            V128LoadExtendKind::E32x2S => Self::E32x2S,
            V128LoadExtendKind::E32x2U => Self::E32x2U,
        }
    }
}

impl From<V128ExtendKind> for VpmovKind {
    fn from(value: V128ExtendKind) -> Self {
        match value {
            V128ExtendKind::LowI8x16S | V128ExtendKind::HighI8x16S => Self::E8x8S,
            V128ExtendKind::LowI8x16U => Self::E8x8U,
            V128ExtendKind::LowI16x8S | V128ExtendKind::HighI16x8S => Self::E16x4S,
            V128ExtendKind::LowI16x8U => Self::E16x4U,
            V128ExtendKind::LowI32x4S | V128ExtendKind::HighI32x4S => Self::E32x2S,
            V128ExtendKind::LowI32x4U => Self::E32x2U,
            _ => unimplemented!(),
        }
    }
}

/// Kinds of comparisons supported by `vcmp`.
pub(super) enum VcmpKind {
    /// Equal comparison.
    Eq,
    /// Not equal comparison.
    Ne,
    /// Less than comparison.
    Lt,
    /// Less than or equal comparison.
    Le,
}

/// Kinds of conversions supported by `vcvt`.
pub(super) enum VcvtKind {
    /// Converts 32-bit integers to 32-bit floats.
    I32ToF32,
    /// Converts doubleword integers to double precision floats.
    I32ToF64,
    /// Converts double precision floats to single precision floats.
    F64ToF32,
    // Converts double precision floats to 32-bit integers.
    F64ToI32,
    /// Converts single precision floats to double precision floats.
    F32ToF64,
    /// Converts single precision floats to 32-bit integers.
    F32ToI32,
}

/// Modes supported by `vround`.
pub(crate) enum VroundMode {
    /// Rounds toward zero.
    TowardZero,
}

/// Low level assembler implementation for x64.
pub(crate) struct Assembler {
    /// The machine instruction buffer.
    buffer: MachBuffer<Inst>,
    /// Constant emission information.
    emit_info: EmitInfo,
    /// Emission state.
    emit_state: EmitState,
    /// x64 flags.
    isa_flags: x64_settings::Flags,
    /// Constant pool.
    pool: ConstantPool,
    /// Constants that will be emitted separately by the MachBuffer.
    constants: VCodeConstants,
}

impl Assembler {
    /// Create a new x64 assembler.
    pub fn new(shared_flags: settings::Flags, isa_flags: x64_settings::Flags) -> Self {
        Self {
            buffer: MachBuffer::<Inst>::new(),
            emit_state: Default::default(),
            emit_info: EmitInfo::new(shared_flags, isa_flags.clone()),
            constants: Default::default(),
            pool: ConstantPool::new(),
            isa_flags,
        }
    }

    /// Get a mutable reference to underlying
    /// machine buffer.
    pub fn buffer_mut(&mut self) -> &mut MachBuffer<Inst> {
        &mut self.buffer
    }

    /// Get a reference to the underlying machine buffer.
    pub fn buffer(&self) -> &MachBuffer<Inst> {
        &self.buffer
    }

    /// Adds a constant to the constant pool and returns its address.
    pub fn add_constant(&mut self, constant: &[u8]) -> Address {
        let handle = self.pool.insert(constant.into());
        Address::constant(handle)
    }

    /// Return the emitted code.
    pub fn finalize(mut self, loc: Option<SourceLoc>) -> MachBufferFinalized<Final> {
        let stencil = self
            .buffer
            .finish(&self.constants, self.emit_state.ctrl_plane_mut());
        stencil.apply_base_srcloc(loc.unwrap_or_default())
    }

    fn emit(&mut self, inst: Inst) {
        inst.emit(&mut self.buffer, &self.emit_info, &mut self.emit_state);
    }

    fn to_synthetic_amode(
        addr: &Address,
        pool: &mut ConstantPool,
        constants: &mut VCodeConstants,
        buffer: &mut MachBuffer<Inst>,
        memflags: MemFlags,
    ) -> SyntheticAmode {
        match *addr {
            Address::Offset { base, offset } => {
                let amode = Amode::imm_reg(offset as i32, base.into()).with_flags(memflags);
                SyntheticAmode::real(amode)
            }
            Address::Const(c) => {
                // Defer the creation of the
                // `SyntheticAmode::ConstantOffset` addressing mode
                // until the address is referenced by an actual
                // instruction.
                let constant_data = pool.get(c);
                let data = VCodeConstantData::Pool(c, constant_data.clone());
                // If the constant data is not marked as used, it will be
                // inserted, therefore, it needs to be registered.
                let needs_registration = !constants.pool_uses(&data);
                let constant = constants.insert(VCodeConstantData::Pool(c, constant_data.clone()));

                if needs_registration {
                    buffer.register_constant(&constant, &data);
                }
                SyntheticAmode::ConstantOffset(constant)
            }
            Address::ImmRegRegShift {
                simm32,
                base,
                index,
                shift,
            } => SyntheticAmode::Real(Amode::ImmRegRegShift {
                simm32,
                base: base.into(),
                index: index.into(),
                shift,
                flags: memflags,
            }),
        }
    }

    /// Emit an unwind instruction.
    pub fn unwind_inst(&mut self, inst: UnwindInst) {
        self.emit(Inst::Unwind { inst })
    }

    /// Push register.
    pub fn push_r(&mut self, reg: Reg) {
        self.emit(Inst::Push64 { src: reg.into() });
    }

    /// Pop to register.
    pub fn pop_r(&mut self, dst: WritableReg) {
        let writable = dst.map(Into::into);
        let dst = WritableGpr::from_writable_reg(writable).expect("valid writable gpr");
        self.emit(Inst::Pop64 { dst });
    }

    /// Return instruction.
    pub fn ret(&mut self) {
        self.emit(Inst::Ret {
            stack_bytes_to_pop: 0,
        });
    }

    /// Register-to-register move.
    pub fn mov_rr(&mut self, src: Reg, dst: WritableReg, size: OperandSize) {
        self.emit(Inst::MovRR {
            src: src.into(),
            dst: dst.map(Into::into),
            size: size.into(),
        });
    }

    /// Register-to-memory move.
    pub fn mov_rm(&mut self, src: Reg, addr: &Address, size: OperandSize, flags: MemFlags) {
        assert!(addr.is_offset());
        let dst = Self::to_synthetic_amode(
            addr,
            &mut self.pool,
            &mut self.constants,
            &mut self.buffer,
            flags,
        );
        self.emit(Inst::MovRM {
            size: size.into(),
            src: src.into(),
            dst,
        });
    }

    /// Immediate-to-memory move.
    pub fn mov_im(&mut self, src: i32, addr: &Address, size: OperandSize, flags: MemFlags) {
        assert!(addr.is_offset());
        let dst = Self::to_synthetic_amode(
            addr,
            &mut self.pool,
            &mut self.constants,
            &mut self.buffer,
            flags,
        );
        self.emit(Inst::MovImmM {
            size: size.into(),
            simm32: src,
            dst,
        });
    }

    /// Immediate-to-register move.
    pub fn mov_ir(&mut self, imm: u64, dst: WritableReg, size: OperandSize) {
        self.emit(Inst::Imm {
            dst_size: size.into(),
            simm64: imm,
            dst: dst.map(Into::into),
        });
    }

    /// Zero-extend memory-to-register load.
    pub fn movzx_mr(
        &mut self,
        addr: &Address,
        dst: WritableReg,
        ext: Option<Extend<Zero>>,
        memflags: MemFlags,
    ) {
        let src = Self::to_synthetic_amode(
            addr,
            &mut self.pool,
            &mut self.constants,
            &mut self.buffer,
            memflags,
        );

        if let Some(ext) = ext {
            let reg_mem = RegMem::mem(src);
            self.emit(Inst::MovzxRmR {
                ext_mode: ext.into(),
                src: GprMem::unwrap_new(reg_mem),
                dst: dst.map(Into::into),
            });
        } else {
            self.emit(Inst::Mov64MR {
                src,
                dst: dst.map(Into::into),
            });
        }
    }

    // Sign-extend memory-to-register load.
    pub fn movsx_mr(
        &mut self,
        addr: &Address,
        dst: WritableReg,
        ext: Extend<Signed>,
        memflags: MemFlags,
    ) {
        let src = Self::to_synthetic_amode(
            addr,
            &mut self.pool,
            &mut self.constants,
            &mut self.buffer,
            memflags,
        );

        let reg_mem = RegMem::mem(src);
        self.emit(Inst::MovsxRmR {
            ext_mode: ext.into(),
            src: GprMem::unwrap_new(reg_mem),
            dst: dst.map(Into::into),
        })
    }

    /// Register-to-register move with zero extension.
    pub fn movzx_rr(&mut self, src: Reg, dst: WritableReg, kind: Extend<Zero>) {
        self.emit(Inst::MovzxRmR {
            ext_mode: kind.into(),
            src: src.into(),
            dst: dst.map(Into::into),
        })
    }

    /// Register-to-register move with sign extension.
    pub fn movsx_rr(&mut self, src: Reg, dst: WritableReg, kind: Extend<Signed>) {
        self.emit(Inst::MovsxRmR {
            ext_mode: kind.into(),
            src: src.into(),
            dst: dst.map(Into::into),
        });
    }

    /// Integer register conditional move.
    pub fn cmov(&mut self, src: Reg, dst: WritableReg, cc: IntCmpKind, size: OperandSize) {
        let dst: WritableGpr = dst.map(Into::into);
        self.emit(Inst::Cmove {
            size: size.into(),
            cc: cc.into(),
            consequent: src.into(),
            alternative: dst.to_reg().into(),
            dst,
        })
    }

    /// Single and double precision floating point
    /// register-to-register move.
    pub fn xmm_mov_rr(&mut self, src: Reg, dst: WritableReg, size: OperandSize) {
        use OperandSize::*;

        let op = match size {
            S32 => SseOpcode::Movaps,
            S64 => SseOpcode::Movapd,
            S128 => SseOpcode::Movdqa,
            S8 | S16 => unreachable!(),
        };

        self.emit(Inst::XmmUnaryRmRUnaligned {
            op,
            src: XmmMem::unwrap_new(src.into()),
            dst: dst.map(Into::into),
        });
    }

    /// Single and double precision floating point load.
    pub fn xmm_mov_mr(
        &mut self,
        src: &Address,
        dst: WritableReg,
        size: OperandSize,
        flags: MemFlags,
    ) {
        use OperandSize::*;

        assert!(dst.to_reg().is_float());
        let op = match size {
            S32 => SseOpcode::Movss,
            S64 => SseOpcode::Movsd,
            S128 => SseOpcode::Movdqu,
            S16 | S8 => unreachable!(),
        };

        let src = Self::to_synthetic_amode(
            src,
            &mut self.pool,
            &mut self.constants,
            &mut self.buffer,
            flags,
        );
        self.emit(Inst::XmmUnaryRmRUnaligned {
            op,
            src: XmmMem::unwrap_new(RegMem::mem(src)),
            dst: dst.map(Into::into),
        });
    }

    /// Vector load and extend.
    pub fn xmm_vpmov_mr(
        &mut self,
        src: &Address,
        dst: WritableReg,
        kind: VpmovKind,
        flags: MemFlags,
    ) {
        assert!(dst.to_reg().is_float());

        let src = Self::to_synthetic_amode(
            src,
            &mut self.pool,
            &mut self.constants,
            &mut self.buffer,
            flags,
        );

        self.emit(Inst::XmmUnaryRmRVex {
            op: kind.into(),
            src: XmmMem::unwrap_new(RegMem::mem(src)),
            dst: dst.to_reg().into(),
        });
    }

    /// Extends vector of integers in `src` and puts results in `dst`.
    pub fn xmm_vpmov_rr(&mut self, src: Reg, dst: WritableReg, kind: VpmovKind) {
        self.emit(Inst::XmmUnaryRmRVex {
            op: kind.into(),
            src: src.into(),
            dst: dst.to_reg().into(),
        });
    }

    /// Vector load and broadcast.
    pub fn xmm_vpbroadcast_mr(
        &mut self,
        src: &Address,
        dst: WritableReg,
        size: OperandSize,
        flags: MemFlags,
    ) {
        assert!(dst.to_reg().is_float());

        let src = Self::to_synthetic_amode(
            src,
            &mut self.pool,
            &mut self.constants,
            &mut self.buffer,
            flags,
        );

        let op = match size {
            OperandSize::S8 => AvxOpcode::Vpbroadcastb,
            OperandSize::S16 => AvxOpcode::Vpbroadcastw,
            OperandSize::S32 => AvxOpcode::Vpbroadcastd,
            _ => unimplemented!(),
        };

        self.emit(Inst::XmmUnaryRmRVex {
            op,
            src: XmmMem::unwrap_new(RegMem::mem(src)),
            dst: dst.to_reg().into(),
        });
    }

    /// Value in `src` is broadcast into lanes of `size` in `dst`.
    pub fn xmm_vpbroadcast_rr(&mut self, src: Reg, dst: WritableReg, size: OperandSize) {
        assert!(src.is_float() && dst.to_reg().is_float());

        let op = match size {
            OperandSize::S8 => AvxOpcode::Vpbroadcastb,
            OperandSize::S16 => AvxOpcode::Vpbroadcastw,
            OperandSize::S32 => AvxOpcode::Vpbroadcastd,
            _ => unimplemented!(),
        };

        self.emit(Inst::XmmUnaryRmRVex {
            op,
            src: XmmMem::unwrap_new(src.into()),
            dst: dst.to_reg().into(),
        });
    }

    /// Memory to register shuffle of bytes in vector.
    pub fn xmm_vpshuf_mr(
        &mut self,
        src: &Address,
        dst: WritableReg,
        mask: u8,
        size: OperandSize,
        flags: MemFlags,
    ) {
        assert!(dst.to_reg().is_float());

        let op = match size {
            OperandSize::S32 => AvxOpcode::Vpshufd,
            _ => unimplemented!(),
        };

        let src = Self::to_synthetic_amode(
            src,
            &mut self.pool,
            &mut self.constants,
            &mut self.buffer,
            flags,
        );
        self.emit(Inst::XmmUnaryRmRImmVex {
            op,
            src: XmmMem::unwrap_new(RegMem::Mem { addr: src }),
            dst: dst.to_reg().into(),
            imm: mask,
        });
    }

    /// Register to register shuffle of bytes in vector.
    pub fn xmm_vpshuf_rr(&mut self, src: Reg, dst: WritableReg, mask: u8, size: OperandSize) {
        assert!(src.is_float() && dst.to_reg().is_float());

        let op = match size {
            OperandSize::S16 => AvxOpcode::Vpshuflw,
            OperandSize::S32 => AvxOpcode::Vpshufd,
            _ => unimplemented!(),
        };

        self.emit(Inst::XmmUnaryRmRImmVex {
            op,
            src: XmmMem::from(Xmm::from(src)),
            imm: mask,
            dst: dst.to_reg().into(),
        });
    }

    /// Single and double precision floating point store.
    pub fn xmm_mov_rm(&mut self, src: Reg, dst: &Address, size: OperandSize, flags: MemFlags) {
        use OperandSize::*;

        assert!(src.is_float());

        let op = match size {
            S32 => SseOpcode::Movss,
            S64 => SseOpcode::Movsd,
            S128 => SseOpcode::Movdqu,
            S16 | S8 => unreachable!(),
        };

        let dst = Self::to_synthetic_amode(
            dst,
            &mut self.pool,
            &mut self.constants,
            &mut self.buffer,
            flags,
        );
        self.emit(Inst::XmmMovRM {
            op,
            src: src.into(),
            dst,
        });
    }

    /// Floating point register conditional move.
    pub fn xmm_cmov(&mut self, src: Reg, dst: WritableReg, cc: IntCmpKind, size: OperandSize) {
        let dst: WritableXmm = dst.map(Into::into);
        let ty = match size {
            OperandSize::S32 => types::F32,
            OperandSize::S64 => types::F64,
            // Move the entire 128 bits via movdqa.
            OperandSize::S128 => types::I32X4,
            OperandSize::S8 | OperandSize::S16 => unreachable!(),
        };

        self.emit(Inst::XmmCmove {
            ty,
            cc: cc.into(),
            consequent: Xmm::unwrap_new(src.into()),
            alternative: dst.to_reg().into(),
            dst,
        })
    }

    /// Subtract register and register
    pub fn sub_rr(&mut self, src: Reg, dst: WritableReg, size: OperandSize) {
        self.emit(Inst::AluRmiR {
            size: size.into(),
            op: AluRmiROpcode::Sub,
            src1: dst.to_reg().into(),
            src2: src.into(),
            dst: dst.map(Into::into),
        });
    }

    /// Subtact immediate register.
    pub fn sub_ir(&mut self, imm: i32, dst: WritableReg, size: OperandSize) {
        let imm = RegMemImm::imm(imm as u32);

        self.emit(Inst::AluRmiR {
            size: size.into(),
            op: AluRmiROpcode::Sub,
            src1: dst.to_reg().into(),
            src2: GprMemImm::unwrap_new(imm),
            dst: dst.map(Into::into),
        });
    }

    /// "and" two registers.
    pub fn and_rr(&mut self, src: Reg, dst: WritableReg, size: OperandSize) {
        self.emit(Inst::AluRmiR {
            size: size.into(),
            op: AluRmiROpcode::And,
            src1: dst.to_reg().into(),
            src2: src.into(),
            dst: dst.map(Into::into),
        });
    }

    pub fn and_ir(&mut self, imm: i32, dst: WritableReg, size: OperandSize) {
        let imm = RegMemImm::imm(imm as u32);
        self.emit(Inst::AluRmiR {
            size: size.into(),
            op: AluRmiROpcode::And,
            src1: dst.to_reg().into(),
            src2: GprMemImm::unwrap_new(imm),
            dst: dst.map(Into::into),
        });
    }

    /// "and" two float registers.
    pub fn xmm_and_rr(&mut self, src: Reg, dst: WritableReg, size: OperandSize) {
        let op = match size {
            OperandSize::S32 => SseOpcode::Andps,
            OperandSize::S64 => SseOpcode::Andpd,
            OperandSize::S8 | OperandSize::S16 | OperandSize::S128 => unreachable!(),
        };

        self.emit(Inst::XmmRmR {
            op,
            src1: dst.to_reg().into(),
            src2: XmmMemAligned::from(Xmm::from(src)),
            dst: dst.map(Into::into),
        });
    }

    /// "and not" two float registers.
    pub fn xmm_andn_rr(&mut self, src: Reg, dst: WritableReg, size: OperandSize) {
        let op = match size {
            OperandSize::S32 => SseOpcode::Andnps,
            OperandSize::S64 => SseOpcode::Andnpd,
            OperandSize::S8 | OperandSize::S16 | OperandSize::S128 => unreachable!(),
        };

        self.emit(Inst::XmmRmR {
            op,
            src1: dst.to_reg().into(),
            src2: Xmm::from(src).into(),
            dst: dst.map(Into::into),
        });
    }

    pub fn gpr_to_xmm(&mut self, src: Reg, dst: WritableReg, size: OperandSize) {
        let op = match size {
            OperandSize::S32 => SseOpcode::Movd,
            OperandSize::S64 => SseOpcode::Movq,
            OperandSize::S8 | OperandSize::S16 | OperandSize::S128 => unreachable!(),
        };

        self.emit(Inst::GprToXmm {
            op,
            src: src.into(),
            dst: dst.map(Into::into),
            src_size: size.into(),
        })
    }

    pub fn xmm_to_gpr(&mut self, src: Reg, dst: WritableReg, size: OperandSize) {
        let op = match size {
            OperandSize::S32 => SseOpcode::Movd,
            OperandSize::S64 => SseOpcode::Movq,
            OperandSize::S8 | OperandSize::S16 | OperandSize::S128 => unreachable!(),
        };

        self.emit(Inst::XmmToGpr {
            op,
            src: src.into(),
            dst: dst.map(Into::into),
            dst_size: size.into(),
        });
    }

    /// Convert float to signed int.
    pub fn cvt_float_to_sint_seq(
        &mut self,
        src: Reg,
        dst: WritableReg,
        tmp_gpr: Reg,
        tmp_xmm: Reg,
        src_size: OperandSize,
        dst_size: OperandSize,
        saturating: bool,
    ) {
        self.emit(Inst::CvtFloatToSintSeq {
            dst_size: dst_size.into(),
            src_size: src_size.into(),
            is_saturating: saturating,
            src: src.into(),
            dst: dst.map(Into::into),
            tmp_gpr: tmp_gpr.into(),
            tmp_xmm: tmp_xmm.into(),
        });
    }

    /// Convert float to unsigned int.
    pub fn cvt_float_to_uint_seq(
        &mut self,
        src: Reg,
        dst: WritableReg,
        tmp_gpr: Reg,
        tmp_xmm: Reg,
        tmp_xmm2: Reg,
        src_size: OperandSize,
        dst_size: OperandSize,
        saturating: bool,
    ) {
        self.emit(Inst::CvtFloatToUintSeq {
            dst_size: dst_size.into(),
            src_size: src_size.into(),
            is_saturating: saturating,
            src: src.into(),
            dst: dst.map(Into::into),
            tmp_gpr: tmp_gpr.into(),
            tmp_xmm: tmp_xmm.into(),
            tmp_xmm2: tmp_xmm2.into(),
        });
    }

    /// Convert signed int to float.
    pub fn cvt_sint_to_float(
        &mut self,
        src: Reg,
        dst: WritableReg,
        src_size: OperandSize,
        dst_size: OperandSize,
    ) {
        let op = match dst_size {
            OperandSize::S32 => SseOpcode::Cvtsi2ss,
            OperandSize::S64 => SseOpcode::Cvtsi2sd,
            OperandSize::S16 | OperandSize::S8 | OperandSize::S128 => unreachable!(),
        };
        self.emit(Inst::CvtIntToFloat {
            op,
            src1: dst.to_reg().into(),
            src2: src.into(),
            dst: dst.map(Into::into),
            src2_size: src_size.into(),
        });
    }

    /// Convert unsigned 64-bit int to float.
    pub fn cvt_uint64_to_float_seq(
        &mut self,
        src: Reg,
        dst: WritableReg,
        tmp_gpr1: Reg,
        tmp_gpr2: Reg,
        dst_size: OperandSize,
    ) {
        self.emit(Inst::CvtUint64ToFloatSeq {
            dst_size: dst_size.into(),
            src: src.into(),
            dst: dst.map(Into::into),
            tmp_gpr1: tmp_gpr1.into(),
            tmp_gpr2: tmp_gpr2.into(),
        });
    }

    /// Change precision of float.
    pub fn cvt_float_to_float(
        &mut self,
        src: Reg,
        dst: WritableReg,
        src_size: OperandSize,
        dst_size: OperandSize,
    ) {
        let op = match (src_size, dst_size) {
            (OperandSize::S32, OperandSize::S64) => SseOpcode::Cvtss2sd,
            (OperandSize::S64, OperandSize::S32) => SseOpcode::Cvtsd2ss,
            _ => unimplemented!(),
        };

        self.emit(Inst::XmmRmRUnaligned {
            op,
            src2: Xmm::unwrap_new(src.into()).into(),
            src1: dst.to_reg().into(),
            dst: dst.map(Into::into),
        });
    }

    pub fn or_rr(&mut self, src: Reg, dst: WritableReg, size: OperandSize) {
        self.emit(Inst::AluRmiR {
            size: size.into(),
            op: AluRmiROpcode::Or,
            src1: dst.to_reg().into(),
            src2: src.into(),
            dst: dst.map(Into::into),
        });
    }

    pub fn or_ir(&mut self, imm: i32, dst: WritableReg, size: OperandSize) {
        let imm = RegMemImm::imm(imm as u32);

        self.emit(Inst::AluRmiR {
            size: size.into(),
            op: AluRmiROpcode::Or,
            src1: dst.to_reg().into(),
            src2: GprMemImm::unwrap_new(imm),
            dst: dst.map(Into::into),
        });
    }

    pub fn xmm_or_rr(&mut self, src: Reg, dst: WritableReg, size: OperandSize) {
        let op = match size {
            OperandSize::S32 => SseOpcode::Orps,
            OperandSize::S64 => SseOpcode::Orpd,
            OperandSize::S8 | OperandSize::S16 | OperandSize::S128 => unreachable!(),
        };

        self.emit(Inst::XmmRmR {
            op,
            src1: dst.to_reg().into(),
            src2: XmmMemAligned::from(Xmm::from(src)),
            dst: dst.map(Into::into),
        });
    }

    /// Logical exclusive or with registers.
    pub fn xor_rr(&mut self, src: Reg, dst: WritableReg, size: OperandSize) {
        self.emit(Inst::AluRmiR {
            size: size.into(),
            op: AluRmiROpcode::Xor,
            src1: dst.to_reg().into(),
            src2: src.into(),
            dst: dst.map(Into::into),
        });
    }

    pub fn xor_ir(&mut self, imm: i32, dst: WritableReg, size: OperandSize) {
        let imm = RegMemImm::imm(imm as u32);

        self.emit(Inst::AluRmiR {
            size: size.into(),
            op: AluRmiROpcode::Xor,
            src1: dst.to_reg().into(),
            src2: GprMemImm::unwrap_new(imm),
            dst: dst.map(Into::into),
        });
    }

    /// Logical exclusive or with float registers.
    pub fn xmm_xor_rr(&mut self, src: Reg, dst: WritableReg, size: OperandSize) {
        let op = match size {
            OperandSize::S32 => SseOpcode::Xorps,
            OperandSize::S64 => SseOpcode::Xorpd,
            OperandSize::S8 | OperandSize::S16 | OperandSize::S128 => unreachable!(),
        };

        self.emit(Inst::XmmRmR {
            op,
            src1: dst.to_reg().into(),
            src2: XmmMemAligned::from(Xmm::from(src)),
            dst: dst.map(Into::into),
        });
    }

    /// Shift with register and register.
    pub fn shift_rr(&mut self, src: Reg, dst: WritableReg, kind: ShiftKind, size: OperandSize) {
        self.emit(Inst::ShiftR {
            size: size.into(),
            kind: kind.into(),
            src: dst.to_reg().into(),
            num_bits: src.into(),
            dst: dst.map(Into::into),
        });
    }

    /// Shift with immediate and register.
    pub fn shift_ir(&mut self, imm: u8, dst: WritableReg, kind: ShiftKind, size: OperandSize) {
        let imm = imm.into();

        self.emit(Inst::ShiftR {
            size: size.into(),
            kind: kind.into(),
            src: dst.to_reg().into(),
            num_bits: Imm8Gpr::unwrap_new(imm),
            dst: dst.map(Into::into),
        });
    }

    /// Signed/unsigned division.
    ///
    /// Emits a sequence of instructions to ensure the correctness of
    /// the division invariants.  This function assumes that the
    /// caller has correctly allocated the dividend as `(rdx:rax)` and
    /// accounted for the quotient to be stored in `rax`.
    pub fn div(&mut self, divisor: Reg, dst: (Reg, Reg), kind: DivKind, size: OperandSize) {
        let trap = match kind {
            // Signed division has two trapping conditions, integer overflow and
            // divide-by-zero. Check for divide-by-zero explicitly and let the
            // hardware detect overflow.
            //
            // The dividend is sign extended to initialize `rdx`.
            DivKind::Signed => {
                self.emit(Inst::CmpRmiR {
                    size: size.into(),
                    src1: divisor.into(),
                    src2: GprMemImm::unwrap_new(RegMemImm::imm(0)),
                    opcode: CmpOpcode::Cmp,
                });
                self.emit(Inst::TrapIf {
                    cc: CC::Z,
                    trap_code: TrapCode::INTEGER_DIVISION_BY_ZERO,
                });
                self.emit(Inst::SignExtendData {
                    size: size.into(),
                    src: dst.0.into(),
                    dst: dst.1.into(),
                });
                TrapCode::INTEGER_OVERFLOW
            }

            // Unsigned division only traps in one case, on divide-by-zero, so
            // defer that to the trap opcode.
            //
            // The divisor_hi reg is initialized with zero through an
            // xor-against-itself op.
            DivKind::Unsigned => {
                self.emit(Inst::AluRmiR {
                    size: size.into(),
                    op: AluRmiROpcode::Xor,
                    src1: dst.1.into(),
                    src2: dst.1.into(),
                    dst: dst.1.into(),
                });
                TrapCode::INTEGER_DIVISION_BY_ZERO
            }
        };
        self.emit(Inst::Div {
            sign: kind.into(),
            size: size.into(),
            trap,
            divisor: GprMem::unwrap_new(RegMem::reg(divisor.into())),
            dividend_lo: dst.0.into(),
            dividend_hi: dst.1.into(),
            dst_quotient: dst.0.into(),
            dst_remainder: dst.1.into(),
        });
    }

    /// Signed/unsigned remainder.
    ///
    /// Emits a sequence of instructions to ensure the correctness of the
    /// division invariants and ultimately calculate the remainder.
    /// This function assumes that the
    /// caller has correctly allocated the dividend as `(rdx:rax)` and
    /// accounted for the remainder to be stored in `rdx`.
    pub fn rem(&mut self, divisor: Reg, dst: (Reg, Reg), kind: RemKind, size: OperandSize) {
        match kind {
            // Signed remainder goes through a pseudo-instruction which has
            // some internal branching. The `dividend_hi`, or `rdx`, is
            // initialized here with a `SignExtendData` instruction.
            RemKind::Signed => {
                self.emit(Inst::SignExtendData {
                    size: size.into(),
                    src: dst.0.into(),
                    dst: dst.1.into(),
                });
                self.emit(Inst::CheckedSRemSeq {
                    size: size.into(),
                    divisor: divisor.into(),
                    dividend_lo: dst.0.into(),
                    dividend_hi: dst.1.into(),
                    dst_quotient: dst.0.into(),
                    dst_remainder: dst.1.into(),
                });
            }

            // Unsigned remainder initializes `dividend_hi` with zero and
            // then executes a normal `div` instruction.
            RemKind::Unsigned => {
                self.emit(Inst::AluRmiR {
                    size: size.into(),
                    op: AluRmiROpcode::Xor,
                    src1: dst.1.into(),
                    src2: dst.1.into(),
                    dst: dst.1.into(),
                });
                self.emit(Inst::Div {
                    sign: DivSignedness::Unsigned,
                    trap: TrapCode::INTEGER_DIVISION_BY_ZERO,
                    size: size.into(),
                    divisor: GprMem::unwrap_new(RegMem::reg(divisor.into())),
                    dividend_lo: dst.0.into(),
                    dividend_hi: dst.1.into(),
                    dst_quotient: dst.0.into(),
                    dst_remainder: dst.1.into(),
                });
            }
        }
    }

    /// Multiply immediate and register.
    pub fn mul_ir(&mut self, imm: i32, dst: WritableReg, size: OperandSize) {
        self.emit(Inst::IMulImm {
            size: size.into(),
            src1: dst.to_reg().into(),
            src2: imm,
            dst: dst.map(Into::into),
        });
    }

    /// Multiply register and register.
    pub fn mul_rr(&mut self, src: Reg, dst: WritableReg, size: OperandSize) {
        self.emit(Inst::IMul {
            size: size.into(),
            src1: dst.to_reg().into(),
            src2: src.into(),
            dst: dst.map(Into::into),
        });
    }

    /// Add immediate and register.
    pub fn add_ir(&mut self, imm: i32, dst: WritableReg, size: OperandSize) {
        let imm = RegMemImm::imm(imm as u32);

        self.emit(Inst::AluRmiR {
            size: size.into(),
            op: AluRmiROpcode::Add,
            src1: dst.to_reg().into(),
            src2: GprMemImm::unwrap_new(imm),
            dst: dst.map(Into::into),
        });
    }

    /// Add register and register.
    pub fn add_rr(&mut self, src: Reg, dst: WritableReg, size: OperandSize) {
        self.emit(Inst::AluRmiR {
            size: size.into(),
            op: AluRmiROpcode::Add,
            src1: dst.to_reg().into(),
            src2: src.into(),
            dst: dst.map(Into::into),
        });
    }

    pub fn lock_xadd(
        &mut self,
        addr: Address,
        operand: Reg,
        dst: WritableReg,
        size: OperandSize,
        flags: MemFlags,
    ) {
        assert!(addr.is_offset());
        let mem = Self::to_synthetic_amode(
            &addr,
            &mut self.pool,
            &mut self.constants,
            &mut self.buffer,
            flags,
        );

        self.emit(Inst::LockXadd {
            size: size.into(),
            operand: operand.into(),
            mem,
            dst_old: dst.map(Into::into),
        });
    }

    pub fn atomic_rmw_seq(
        &mut self,
        addr: Address,
        operand: Reg,
        dst: WritableReg,
        size: OperandSize,
        flags: MemFlags,
        op: AtomicRmwSeqOp,
    ) {
        assert!(addr.is_offset());
        let mem = Self::to_synthetic_amode(
            &addr,
            &mut self.pool,
            &mut self.constants,
            &mut self.buffer,
            flags,
        );
        self.emit(Inst::AtomicRmwSeq {
            ty: Type::int_with_byte_size(size.bytes() as _).unwrap(),
            mem,
            operand: operand.into(),
            temp: writable!(scratch().into()),
            dst_old: dst.map(Into::into),
            op,
        });
    }

    pub fn xchg(
        &mut self,
        addr: Address,
        operand: Reg,
        dst: WritableReg,
        size: OperandSize,
        flags: MemFlags,
    ) {
        assert!(addr.is_offset());
        let mem = Self::to_synthetic_amode(
            &addr,
            &mut self.pool,
            &mut self.constants,
            &mut self.buffer,
            flags,
        );

        self.emit(Inst::Xchg {
            size: size.into(),
            operand: operand.into(),
            mem,
            dst_old: dst.map(Into::into),
        });
    }
    pub fn cmpxchg(
        &mut self,
        addr: Address,
        expected: Reg,
        replacement: Reg,
        dst: WritableReg,
        size: OperandSize,
        flags: MemFlags,
    ) {
        assert!(addr.is_offset());
        let mem = Self::to_synthetic_amode(
            &addr,
            &mut self.pool,
            &mut self.constants,
            &mut self.buffer,
            flags,
        );

        self.emit(Inst::LockCmpxchg {
            ty: Type::int_with_byte_size(size.bytes() as _).unwrap(),
            replacement: replacement.into(),
            expected: expected.into(),
            mem,
            dst_old: dst.map(Into::into),
        })
    }

    pub fn cmp_ir(&mut self, src1: Reg, imm: i32, size: OperandSize) {
        let imm = RegMemImm::imm(imm as u32);

        self.emit(Inst::CmpRmiR {
            size: size.into(),
            opcode: CmpOpcode::Cmp,
            src1: src1.into(),
            src2: GprMemImm::unwrap_new(imm),
        });
    }

    pub fn cmp_rr(&mut self, src1: Reg, src2: Reg, size: OperandSize) {
        self.emit(Inst::CmpRmiR {
            size: size.into(),
            opcode: CmpOpcode::Cmp,
            src1: src1.into(),
            src2: src2.into(),
        });
    }

    /// Compares values in src1 and src2 and sets ZF, PF, and CF flags in EFLAGS
    /// register.
    pub fn ucomis(&mut self, src1: Reg, src2: Reg, size: OperandSize) {
        let op = match size {
            OperandSize::S32 => SseOpcode::Ucomiss,
            OperandSize::S64 => SseOpcode::Ucomisd,
            OperandSize::S8 | OperandSize::S16 | OperandSize::S128 => unreachable!(),
        };

        self.emit(Inst::XmmCmpRmR {
            op,
            src1: src1.into(),
            src2: Xmm::from(src2).into(),
        });
    }

    pub fn popcnt(&mut self, src: Reg, size: OperandSize) {
        assert!(
            self.isa_flags.has_popcnt() && self.isa_flags.has_sse42(),
            "Requires has_popcnt and has_sse42 flags"
        );
        self.emit(Inst::UnaryRmR {
            size: size.into(),
            op: args::UnaryRmROpcode::Popcnt,
            src: src.into(),
            dst: src.into(),
        });
    }

    /// Emit a test instruction with two register operands.
    pub fn test_rr(&mut self, src1: Reg, src2: Reg, size: OperandSize) {
        self.emit(Inst::CmpRmiR {
            size: size.into(),
            opcode: CmpOpcode::Test,
            src1: src1.into(),
            src2: src2.into(),
        })
    }

    /// Set value in dst to `0` or `1` based on flags in status register and
    /// [`CmpKind`].
    pub fn setcc(&mut self, kind: IntCmpKind, dst: WritableReg) {
        self.setcc_impl(kind.into(), dst);
    }

    /// Set value in dst to `1` if parity flag in status register is set, `0`
    /// otherwise.
    pub fn setp(&mut self, dst: WritableReg) {
        self.setcc_impl(CC::P, dst);
    }

    /// Set value in dst to `1` if parity flag in status register is not set,
    /// `0` otherwise.
    pub fn setnp(&mut self, dst: WritableReg) {
        self.setcc_impl(CC::NP, dst);
    }

    fn setcc_impl(&mut self, cc: CC, dst: WritableReg) {
        // Clear the dst register or bits 1 to 31 may be incorrectly set.
        // Don't use xor since it updates the status register.
        self.emit(Inst::Imm {
            dst_size: args::OperandSize::Size32, // Always going to be an i32 result.
            simm64: 0,
            dst: dst.map(Into::into),
        });
        // Copy correct bit from status register into dst register.
        self.emit(Inst::Setcc {
            cc,
            dst: dst.map(Into::into),
        });
    }

    /// Store the count of leading zeroes in src in dst.
    /// Requires `has_lzcnt` flag.
    pub fn lzcnt(&mut self, src: Reg, dst: WritableReg, size: OperandSize) {
        assert!(self.isa_flags.has_lzcnt(), "Requires has_lzcnt flag");
        self.emit(Inst::UnaryRmR {
            size: size.into(),
            op: args::UnaryRmROpcode::Lzcnt,
            src: src.into(),
            dst: dst.map(Into::into),
        });
    }

    /// Store the count of trailing zeroes in src in dst.
    /// Requires `has_bmi1` flag.
    pub fn tzcnt(&mut self, src: Reg, dst: WritableReg, size: OperandSize) {
        assert!(self.isa_flags.has_bmi1(), "Requires has_bmi1 flag");
        self.emit(Inst::UnaryRmR {
            size: size.into(),
            op: args::UnaryRmROpcode::Tzcnt,
            src: src.into(),
            dst: dst.map(Into::into),
        });
    }

    /// Stores position of the most significant bit set in src in dst.
    /// Zero flag is set if src is equal to 0.
    pub fn bsr(&mut self, src: Reg, dst: WritableReg, size: OperandSize) {
        self.emit(Inst::UnaryRmR {
            size: size.into(),
            op: args::UnaryRmROpcode::Bsr,
            src: src.into(),
            dst: dst.map(Into::into),
        });
    }

    /// Performs integer negation on src and places result in dst.
    pub fn neg(&mut self, src: Reg, dst: WritableReg, size: OperandSize) {
        self.emit(Inst::Neg {
            size: size.into(),
            src: src.into(),
            dst: dst.map(Into::into),
        });
    }

    /// Stores position of the least significant bit set in src in dst.
    /// Zero flag is set if src is equal to 0.
    pub fn bsf(&mut self, src: Reg, dst: WritableReg, size: OperandSize) {
        self.emit(Inst::UnaryRmR {
            size: size.into(),
            op: args::UnaryRmROpcode::Bsf,
            src: src.into(),
            dst: dst.map(Into::into),
        });
    }

    /// Performs float addition on src and dst and places result in dst.
    pub fn xmm_add_rr(&mut self, src: Reg, dst: WritableReg, size: OperandSize) {
        let op = match size {
            OperandSize::S32 => SseOpcode::Addss,
            OperandSize::S64 => SseOpcode::Addsd,
            OperandSize::S8 | OperandSize::S16 | OperandSize::S128 => unreachable!(),
        };

        self.emit(Inst::XmmRmRUnaligned {
            op,
            src1: Xmm::from(dst.to_reg()).into(),
            src2: Xmm::from(src).into(),
            dst: dst.map(Into::into),
        });
    }

    /// Performs float subtraction on src and dst and places result in dst.
    pub fn xmm_sub_rr(&mut self, src: Reg, dst: WritableReg, size: OperandSize) {
        let op = match size {
            OperandSize::S32 => SseOpcode::Subss,
            OperandSize::S64 => SseOpcode::Subsd,
            OperandSize::S8 | OperandSize::S16 | OperandSize::S128 => unreachable!(),
        };

        self.emit(Inst::XmmRmRUnaligned {
            op,
            src1: Xmm::from(dst.to_reg()).into(),
            src2: Xmm::from(src).into(),
            dst: dst.map(Into::into),
        });
    }

    /// Performs float multiplication on src and dst and places result in dst.
    pub fn xmm_mul_rr(&mut self, src: Reg, dst: WritableReg, size: OperandSize) {
        let op = match size {
            OperandSize::S32 => SseOpcode::Mulss,
            OperandSize::S64 => SseOpcode::Mulsd,
            OperandSize::S8 | OperandSize::S16 | OperandSize::S128 => unreachable!(),
        };

        self.emit(Inst::XmmRmRUnaligned {
            op,
            src1: Xmm::from(dst.to_reg()).into(),
            src2: Xmm::from(src).into(),
            dst: dst.map(Into::into),
        });
    }

    /// Performs float division on src and dst and places result in dst.
    pub fn xmm_div_rr(&mut self, src: Reg, dst: WritableReg, size: OperandSize) {
        let op = match size {
            OperandSize::S32 => SseOpcode::Divss,
            OperandSize::S64 => SseOpcode::Divsd,
            OperandSize::S8 | OperandSize::S16 | OperandSize::S128 => unreachable!(),
        };

        self.emit(Inst::XmmRmRUnaligned {
            op,
            src1: Xmm::from(dst.to_reg()).into(),
            src2: Xmm::from(src).into(),
            dst: dst.map(Into::into),
        });
    }

    /// Minimum for src and dst XMM registers with results put in dst.
    pub fn xmm_min_seq(&mut self, src: Reg, dst: WritableReg, size: OperandSize) {
        self.emit(Inst::XmmMinMaxSeq {
            size: size.into(),
            is_min: true,
            lhs: src.into(),
            rhs: dst.to_reg().into(),
            dst: dst.map(Into::into),
        });
    }

    /// Maximum for src and dst XMM registers with results put in dst.
    pub fn xmm_max_seq(&mut self, src: Reg, dst: WritableReg, size: OperandSize) {
        self.emit(Inst::XmmMinMaxSeq {
            size: size.into(),
            is_min: false,
            lhs: src.into(),
            rhs: dst.to_reg().into(),
            dst: dst.map(Into::into),
        });
    }

    /// Perform rounding operation on float register src and place results in
    /// float register dst.
    pub fn xmm_rounds_rr(
        &mut self,
        src: Reg,
        dst: WritableReg,
        mode: RoundingMode,
        size: OperandSize,
    ) {
        let op = match size {
            OperandSize::S32 => SseOpcode::Roundss,
            OperandSize::S64 => SseOpcode::Roundsd,
            OperandSize::S8 | OperandSize::S16 | OperandSize::S128 => unreachable!(),
        };

        let imm: u8 = match mode {
            RoundingMode::Nearest => 0x00,
            RoundingMode::Down => 0x01,
            RoundingMode::Up => 0x02,
            RoundingMode::Zero => 0x03,
        };

        self.emit(Inst::XmmUnaryRmRImm {
            op,
            src: XmmMemAligned::from(Xmm::from(src)),
            imm,
            dst: dst.map(Into::into),
        })
    }

    pub fn sqrt(&mut self, src: Reg, dst: WritableReg, size: OperandSize) {
        let op = match size {
            OperandSize::S32 => SseOpcode::Sqrtss,
            OperandSize::S64 => SseOpcode::Sqrtsd,
            OperandSize::S8 | OperandSize::S16 | OperandSize::S128 => unreachable!(),
        };

        self.emit(Inst::XmmRmR {
            op,
            src2: Xmm::from(src).into(),
            src1: dst.to_reg().into(),
            dst: dst.map(Into::into),
        })
    }

    /// Emit a call to an unknown location through a register.
    pub fn call_with_reg(&mut self, cc: CallingConvention, callee: Reg) {
        self.emit(Inst::CallUnknown {
            info: Box::new(CallInfo::empty(RegMem::reg(callee.into()), cc.into())),
        });
    }

    /// Emit a call to a locally defined function through an index.
    pub fn call_with_name(&mut self, cc: CallingConvention, name: UserExternalNameRef) {
        self.emit(Inst::CallKnown {
            info: Box::new(CallInfo::empty(ExternalName::user(name), cc.into())),
        });
    }

    /// Emit a call to a well-known libcall.
    pub fn call_with_lib(&mut self, cc: CallingConvention, lib: LibCall, dst: Reg) {
        let dest = ExternalName::LibCall(lib);

        // `use_colocated_libcalls` is never `true` from within Wasmtime,
        // so always require loading the libcall to a register and use
        // a `Far` relocation distance to ensure the right relocation when
        // emitting to binary.
        //
        // See [wasmtime::engine::Engine::check_compatible_with_shared_flag] and
        // [wasmtime_cranelift::obj::ModuleTextBuilder::append_func]
        self.emit(Inst::LoadExtName {
            dst: Writable::from_reg(dst.into()),
            name: Box::new(dest),
            offset: 0,
            distance: RelocDistance::Far,
        });
        self.call_with_reg(cc, dst);
    }

    /// Emits a conditional jump to the given label.
    pub fn jmp_if(&mut self, cc: impl Into<CC>, taken: MachLabel) {
        self.emit(Inst::WinchJmpIf {
            cc: cc.into(),
            taken,
        });
    }

    /// Performs an unconditional jump to the given label.
    pub fn jmp(&mut self, target: MachLabel) {
        self.emit(Inst::JmpKnown { dst: target });
    }

    /// Emits a jump table sequence.
    pub fn jmp_table(
        &mut self,
        targets: SmallVec<[MachLabel; 4]>,
        default: MachLabel,
        index: Reg,
        tmp1: Reg,
        tmp2: Reg,
    ) {
        self.emit(Inst::JmpTableSeq {
            idx: index.into(),
            tmp1: Writable::from_reg(tmp1.into()),
            tmp2: Writable::from_reg(tmp2.into()),
            default_target: default,
            targets: Box::new(targets.to_vec()),
        })
    }

    /// Emit a trap instruction.
    pub fn trap(&mut self, code: TrapCode) {
        self.emit(Inst::Ud2 { trap_code: code })
    }

    /// Conditional trap.
    pub fn trapif(&mut self, cc: impl Into<CC>, trap_code: TrapCode) {
        self.emit(Inst::TrapIf {
            cc: cc.into(),
            trap_code,
        });
    }

    /// Load effective address.
    pub fn lea(&mut self, addr: &Address, dst: WritableReg, size: OperandSize) {
        let addr = Self::to_synthetic_amode(
            addr,
            &mut self.pool,
            &mut self.constants,
            &mut self.buffer,
            MemFlags::trusted(),
        );
        self.emit(Inst::LoadEffectiveAddress {
            addr,
            dst: dst.map(Into::into),
            size: size.into(),
        });
    }

    pub fn adc_rr(&mut self, src: Reg, dst: WritableReg, size: OperandSize) {
        self.emit(Inst::AluRmiR {
            size: size.into(),
            op: AluRmiROpcode::Adc,
            src1: dst.to_reg().into(),
            src2: src.into(),
            dst: dst.map(Into::into),
        });
    }

    pub fn sbb_rr(&mut self, src: Reg, dst: WritableReg, size: OperandSize) {
        self.emit(Inst::AluRmiR {
            size: size.into(),
            op: AluRmiROpcode::Sbb,
            src1: dst.to_reg().into(),
            src2: src.into(),
            dst: dst.map(Into::into),
        });
    }

    pub fn mul_wide(
        &mut self,
        dst_lo: WritableReg,
        dst_hi: WritableReg,
        lhs: Reg,
        rhs: Reg,
        kind: MulWideKind,
        size: OperandSize,
    ) {
        self.emit(Inst::Mul {
            signed: kind == MulWideKind::Signed,
            size: size.into(),
            src1: lhs.into(),
            src2: rhs.into(),
            dst_lo: dst_lo.to_reg().into(),
            dst_hi: dst_hi.to_reg().into(),
        });
    }

    /// Shuffles bytes in `src` according to contents of `mask` and puts
    /// result in `dst`.
    pub fn xmm_vpshufb_rrm(&mut self, dst: WritableReg, src: Reg, mask: &Address) {
        let mask = Self::to_synthetic_amode(
            mask,
            &mut self.pool,
            &mut self.constants,
            &mut self.buffer,
            MemFlags::trusted(),
        );

        self.emit(Inst::XmmRmiRVex {
            op: args::AvxOpcode::Vpshufb,
            src1: src.into(),
            src2: XmmMemImm::unwrap_new(RegMemImm::Mem { addr: mask }),
            dst: dst.to_reg().into(),
        });
    }

    /// Shuffles bytes in `src` according to contents of `mask` and puts
    /// result in `dst`.
    pub fn xmm_vpshufb_rrr(&mut self, dst: WritableReg, src: Reg, mask: Reg) {
        self.emit(Inst::XmmRmiRVex {
            op: args::AvxOpcode::Vpshufb,
            src1: src.into(),
            src2: XmmMemImm::unwrap_new(RegMemImm::reg(mask.into())),
            dst: dst.to_reg().into(),
        })
    }

    /// Bitwise OR of `src1` and `src2`.
    pub fn vpor(&mut self, dst: WritableReg, src1: Reg, src2: Reg) {
        self.emit(Inst::XmmRmiRVex {
            op: args::AvxOpcode::Vpor,
            src1: src1.into(),
            src2: XmmMemImm::unwrap_new(src2.into()),
            dst: dst.to_reg().into(),
        })
    }

    /// Add unsigned integers with unsigned saturation.
    ///
    /// Adds the src operands but when an individual byte result is larger than
    /// an unsigned byte integer, 0xFF is written instead.
    pub fn xmm_vpaddusb_rrm(&mut self, dst: WritableReg, src1: Reg, src2: &Address) {
        let src2 = Self::to_synthetic_amode(
            src2,
            &mut self.pool,
            &mut self.constants,
            &mut self.buffer,
            MemFlags::trusted(),
        );

        self.emit(Inst::XmmRmiRVex {
            op: args::AvxOpcode::Vpaddusb,
            src1: src1.into(),
            src2: XmmMemImm::unwrap_new(RegMemImm::mem(src2)),
            dst: dst.to_reg().into(),
        })
    }

    /// Adds vectors of integers in `src1` and `src2` and puts the results in
    /// `dst`.
    pub fn xmm_vpadd_rrr(&mut self, src1: Reg, src2: Reg, dst: WritableReg, size: OperandSize) {
        let op = match size {
            OperandSize::S32 => AvxOpcode::Vpaddd,
            _ => unimplemented!(),
        };

        self.emit(Inst::XmmRmiRVex {
            op,
            src1: src1.into(),
            src2: src2.into(),
            dst: dst.to_reg().into(),
        });
    }

    pub fn fence(&mut self, kind: FenceKind) {
        self.emit(Inst::Fence { kind });
    }

    /// Extract a value from `src` into `dst` (zero extended) determined by `lane`.
    pub fn xmm_vpextr_rr(&mut self, dst: WritableReg, src: Reg, lane: u8, size: OperandSize) {
        self.emit(Inst::XmmToGprImmVex {
            op: Self::vpextr_opcode(size),
            src: src.into(),
            dst: dst.to_reg().into(),
            imm: lane,
        });
    }

    /// Copy value from `src2`, merge into `src1`, and put result in `dst` at
    /// the location specified in `count`.
    pub fn xmm_vpinsr_rrm(
        &mut self,
        dst: WritableReg,
        src1: Reg,
        src2: &Address,
        count: u8,
        size: OperandSize,
    ) {
        let src2 = Self::to_synthetic_amode(
            src2,
            &mut self.pool,
            &mut self.constants,
            &mut self.buffer,
            MemFlags::trusted(),
        );

        self.emit(Inst::XmmVexPinsr {
            op: Self::vpinsr_opcode(size),
            src1: src1.into(),
            src2: GprMem::unwrap_new(RegMem::mem(src2)),
            dst: dst.to_reg().into(),
            imm: count,
        });
    }

    /// Copy value from `src2`, merge into `src1`, and put result in `dst` at
    /// the location specified in `count`.
    pub fn xmm_vpinsr_rrr(
        &mut self,
        dst: WritableReg,
        src1: Reg,
        src2: Reg,
        count: u8,
        size: OperandSize,
    ) {
        self.emit(Inst::XmmVexPinsr {
            op: Self::vpinsr_opcode(size),
            src1: src1.into(),
            src2: src2.into(),
            dst: dst.to_reg().into(),
            imm: count,
        });
    }

    /// Copy a 32-bit float in `src2`, merge into `src1`, and put result in `dst`.
    pub fn xmm_vinsertps_rrm(&mut self, dst: WritableReg, src1: Reg, src2: &Address, imm: u8) {
        let src2 = Self::to_synthetic_amode(
            src2,
            &mut self.pool,
            &mut self.constants,
            &mut self.buffer,
            MemFlags::trusted(),
        );

        self.emit(Inst::XmmRmRImmVex {
            op: AvxOpcode::Vinsertps,
            src1: src1.into(),
            src2: XmmMem::unwrap_new(RegMem::mem(src2)),
            dst: dst.to_reg().into(),
            imm,
        });
    }

    /// Copy a 32-bit float in `src2`, merge into `src1`, and put result in `dst`.
    pub fn xmm_vinsertps_rrr(&mut self, dst: WritableReg, src1: Reg, src2: Reg, imm: u8) {
        self.emit(Inst::XmmRmRImmVex {
            op: AvxOpcode::Vinsertps,
            src1: src1.into(),
            src2: XmmMem::unwrap_new(RegMem::reg(src2.into())),
            dst: dst.to_reg().into(),
            imm,
        });
    }

    /// Moves lower 64-bit float from `src2` into lower 64-bits of `dst` and the
    /// upper 64-bits in `src1` into the upper 64-bits of `dst`.
    pub fn xmm_vmovsd_rrr(&mut self, dst: WritableReg, src1: Reg, src2: Reg) {
        self.emit(Inst::XmmRmiRVex {
            op: AvxOpcode::Vmovsd,
            src1: src1.into(),
            src2: XmmMemImm::unwrap_new(src2.into()),
            dst: dst.to_reg().into(),
        })
    }

    /// Moves 64-bit float from `src` into lower 64-bits of `dst`.
    pub fn xmm_vmovsd_rm(&mut self, dst: WritableReg, src: &Address) {
        let src = Self::to_synthetic_amode(
            src,
            &mut self.pool,
            &mut self.constants,
            &mut self.buffer,
            MemFlags::trusted(),
        );

        self.emit(Inst::XmmUnaryRmRVex {
            op: AvxOpcode::Vmovsd,
            src: XmmMem::unwrap_new(RegMem::mem(src)),
            dst: dst.to_reg().into(),
        })
    }

    /// Moves two 32-bit floats from `src2` to the upper 64-bits of `dst`.
    /// Copies two 32-bit floats from the lower 64-bits of `src1` to lower
    /// 64-bits of `dst`.
    pub fn xmm_vmovlhps_rrm(&mut self, dst: WritableReg, src1: Reg, src2: &Address) {
        let src2 = Self::to_synthetic_amode(
            src2,
            &mut self.pool,
            &mut self.constants,
            &mut self.buffer,
            MemFlags::trusted(),
        );

        self.emit(Inst::XmmRmiRVex {
            op: AvxOpcode::Vmovlhps,
            src1: src1.into(),
            src2: XmmMemImm::unwrap_new(RegMemImm::mem(src2)),
            dst: dst.to_reg().into(),
        });
    }

    /// Moves two 32-bit floats from the lower 64-bits of `src2` to the upper
    /// 64-bits of `dst`. Copies two 32-bit floats from the lower 64-bits of
    /// `src1` to lower 64-bits of `dst`.
    pub fn xmm_vmovlhps_rrr(&mut self, dst: WritableReg, src1: Reg, src2: Reg) {
        self.emit(Inst::XmmRmiRVex {
            op: AvxOpcode::Vmovlhps,
            src1: src1.into(),
            src2: XmmMemImm::unwrap_new(src2.into()),
            dst: dst.to_reg().into(),
        });
    }

    /// Move unaligned packed integer values from address `src` to `dst`.
    pub fn xmm_vmovdqu_mr(&mut self, src: &Address, dst: WritableReg, flags: MemFlags) {
        let src = Self::to_synthetic_amode(
            src,
            &mut self.pool,
            &mut self.constants,
            &mut self.buffer,
            flags,
        );
        self.emit(Inst::XmmUnaryRmRVex {
            op: AvxOpcode::Vmovdqu,
            src: XmmMem::unwrap_new(RegMem::mem(src)),
            dst: dst.map(Into::into),
        });
    }

    /// Move integer from `src` to xmm register `dst` using an AVX instruction.
    pub fn avx_gpr_to_xmm(&mut self, src: Reg, dst: WritableReg, size: OperandSize) {
        let op = match size {
            OperandSize::S32 => AvxOpcode::Vmovd,
            OperandSize::S64 => AvxOpcode::Vmovq,
            _ => unreachable!(),
        };

        self.emit(Inst::GprToXmmVex {
            op,
            src: src.into(),
            dst: dst.map(Into::into),
            src_size: size.into(),
        })
    }

    /// The `vpinsr` opcode to use.
    fn vpinsr_opcode(size: OperandSize) -> AvxOpcode {
        match size {
            OperandSize::S8 => AvxOpcode::Vpinsrb,
            OperandSize::S16 => AvxOpcode::Vpinsrw,
            OperandSize::S32 => AvxOpcode::Vpinsrd,
            OperandSize::S64 => AvxOpcode::Vpinsrq,
            _ => unimplemented!(),
        }
    }

    /// Perform an AVX opcode `op` involving registers `src1` and `src2`, writing the
    /// result to `dst`.
    pub fn xmm_vex_rr(&mut self, op: AvxOpcode, src1: Reg, src2: Reg, dst: WritableReg) {
        self.emit(Inst::XmmRmiRVex {
            op,
            src1: src1.into(),
            src2: src2.into(),
            dst: dst.map(Into::into),
        })
    }

    /// Perform an AVX opcode `op` involving register `src1` and an immediate `imm`, writing the
    /// result to `dst`.
    pub fn xmm_vex_ri(&mut self, op: AvxOpcode, src1: Reg, imm: u32, dst: WritableReg) {
        self.emit(Inst::XmmRmiRVex {
            op,
            src1: src1.into(),
            src2: XmmMemImm::unwrap_new(RegMemImm::imm(imm)),
            dst: dst.map(Into::into),
        })
    }

    pub fn xmm_vptest(&mut self, src1: Reg, src2: Reg) {
        self.emit(Inst::XmmCmpRmRVex {
            op: AvxOpcode::Vptest,
            src1: src1.into(),
            src2: src2.into(),
        })
    }

    /// The `vpextr` opcode to use.
    fn vpextr_opcode(size: OperandSize) -> AvxOpcode {
        match size {
            OperandSize::S8 => AvxOpcode::Vpextrb,
            OperandSize::S16 => AvxOpcode::Vpextrw,
            OperandSize::S32 => AvxOpcode::Vpextrd,
            OperandSize::S64 => AvxOpcode::Vpextrq,
            _ => unimplemented!(),
        }
    }

    /// Extract a value from `src` into `addr` determined by `lane`.
    pub(crate) fn xmm_vpextr_rm(
        &mut self,
        addr: &Address,
        src: Reg,
        lane: u8,
        size: OperandSize,
        flags: MemFlags,
    ) -> anyhow::Result<()> {
        assert!(addr.is_offset());
        let dst = Self::to_synthetic_amode(
            addr,
            &mut self.pool,
            &mut self.constants,
            &mut self.buffer,
            flags,
        );

        self.emit(Inst::XmmMovRMImmVex {
            op: Self::vpextr_opcode(size),
            src: src.into(),
            dst,
            imm: lane,
        });

        Ok(())
    }

    /// Converts vector of integers into vector of floating values.
    pub fn xmm_vcvt_rr(&mut self, src: Reg, dst: WritableReg, kind: VcvtKind) {
        let op = match kind {
            VcvtKind::I32ToF32 => AvxOpcode::Vcvtdq2ps,
            VcvtKind::I32ToF64 => AvxOpcode::Vcvtdq2pd,
            VcvtKind::F64ToF32 => AvxOpcode::Vcvtpd2ps,
            VcvtKind::F64ToI32 => AvxOpcode::Vcvttpd2dq,
            VcvtKind::F32ToF64 => AvxOpcode::Vcvtps2pd,
            VcvtKind::F32ToI32 => AvxOpcode::Vcvttps2dq,
        };

        self.emit(Inst::XmmUnaryRmRVex {
            op,
            src: src.into(),
            dst: dst.to_reg().into(),
        });
    }

    /// Shift vector data left by `imm`.
    pub fn xmm_vpsll_rr(&mut self, src: Reg, dst: WritableReg, imm: u32, size: OperandSize) {
        let op = match size {
            OperandSize::S32 => AvxOpcode::Vpslld,
            _ => unimplemented!(),
        };

        self.emit(Inst::XmmRmiRVex {
            op,
            src1: src.into(),
            src2: XmmMemImm::unwrap_new(RegMemImm::imm(imm)),
            dst: dst.to_reg().into(),
        });
    }

    /// Shift vector data right by `imm`.
    pub fn xmm_vpsrl_rr(&mut self, src: Reg, dst: WritableReg, imm: u32, size: OperandSize) {
        let op = match size {
            OperandSize::S32 => AvxOpcode::Vpsrld,
            OperandSize::S64 => AvxOpcode::Vpsrlq,
            _ => unimplemented!(),
        };

        self.emit(Inst::XmmRmiRVex {
            op,
            src1: src.into(),
            src2: XmmMemImm::unwrap_new(RegMemImm::imm(imm)),
            dst: dst.to_reg().into(),
        })
    }

    /// Subtract integers in vector `src1` from integers in vector `src2`.
    pub fn xmm_vpsub_rrr(&mut self, src1: Reg, src2: Reg, dst: WritableReg, size: OperandSize) {
        let op = match size {
            OperandSize::S32 => AvxOpcode::Vpsubd,
            OperandSize::S64 => AvxOpcode::Vpsubq,
            _ => unimplemented!(),
        };

        self.emit(Inst::XmmRmiRVex {
            op,
            src1: src1.into(),
            src2: src2.into(),
            dst: dst.to_reg().into(),
        })
    }

    fn vaddp_opcode(size: OperandSize) -> AvxOpcode {
        match size {
            OperandSize::S32 => AvxOpcode::Vaddps,
            OperandSize::S64 => AvxOpcode::Vaddpd,
            _ => unimplemented!(),
        }
    }

    /// Add floats in vector `src1` to floats in vector `src2`.
    pub fn xmm_vaddp_rrm(
        &mut self,
        src1: Reg,
        src2: &Address,
        dst: WritableReg,
        size: OperandSize,
    ) {
        let address = Self::to_synthetic_amode(
            src2,
            &mut self.pool,
            &mut self.constants,
            &mut self.buffer,
            MemFlags::trusted(),
        );

        self.emit(Inst::XmmRmiRVex {
            op: Self::vaddp_opcode(size),
            src1: src1.into(),
            src2: XmmMemImm::unwrap_new(RegMemImm::mem(address)),
            dst: dst.to_reg().into(),
        });
    }

    /// Add floats in vector `src1` to floats in vector `src2`.
    pub fn xmm_vaddp_rrr(&mut self, src1: Reg, src2: Reg, dst: WritableReg, size: OperandSize) {
        self.emit(Inst::XmmRmiRVex {
            op: Self::vaddp_opcode(size),
            src1: src1.into(),
            src2: src2.into(),
            dst: dst.to_reg().into(),
        })
    }

    fn vpcmpeq_opcode(size: OperandSize) -> AvxOpcode {
        match size {
            OperandSize::S8 => AvxOpcode::Vpcmpeqb,
            OperandSize::S16 => AvxOpcode::Vpcmpeqw,
            OperandSize::S32 => AvxOpcode::Vpcmpeqd,
            OperandSize::S64 => AvxOpcode::Vpcmpeqq,
            _ => unimplemented!(),
        }
    }

    /// Compare vector register `lhs` with a vector of integers in `rhs` for
    /// equality between packed integers and write the resulting vector into
    /// `dst`.
    pub fn xmm_vpcmpeq_rrm(
        &mut self,
        dst: WritableReg,
        lhs: Reg,
        address: &Address,
        size: OperandSize,
    ) {
        let address = Self::to_synthetic_amode(
            address,
            &mut self.pool,
            &mut self.constants,
            &mut self.buffer,
            MemFlags::trusted(),
        );

        self.emit(Inst::XmmRmiRVex {
            op: Self::vpcmpeq_opcode(size),
            src1: lhs.into(),
            src2: XmmMemImm::unwrap_new(RegMemImm::mem(address)),
            dst: dst.to_reg().into(),
        });
    }

    /// Compare vector registers `lhs` and `rhs` for equality between packed
    /// integers and write the resulting vector into `dst`.
    pub fn xmm_vpcmpeq_rrr(&mut self, dst: WritableReg, lhs: Reg, rhs: Reg, size: OperandSize) {
        self.emit(Inst::XmmRmiRVex {
            op: Self::vpcmpeq_opcode(size),
            src1: lhs.into(),
            src2: XmmMemImm::unwrap_new(rhs.into()),
            dst: dst.to_reg().into(),
        })
    }

    /// Performs a greater than comparison with vectors of signed integers in
    /// `lhs` and `rhs` and puts the results in `dst`.
    pub fn xmm_vpcmpgt_rrr(&mut self, dst: WritableReg, lhs: Reg, rhs: Reg, size: OperandSize) {
        let op = match size {
            OperandSize::S8 => AvxOpcode::Vpcmpgtb,
            OperandSize::S16 => AvxOpcode::Vpcmpgtw,
            OperandSize::S32 => AvxOpcode::Vpcmpgtd,
            OperandSize::S64 => AvxOpcode::Vpcmpgtq,
            _ => unimplemented!(),
        };

        self.emit(Inst::XmmRmiRVex {
            op,
            src1: lhs.into(),
            src2: XmmMemImm::unwrap_new(rhs.into()),
            dst: dst.to_reg().into(),
        })
    }

    /// Performs a max operation with vectors of signed integers in `lhs` and
    /// `rhs` and puts the results in `dst`.
    pub fn xmm_vpmaxs_rrr(&mut self, dst: WritableReg, lhs: Reg, rhs: Reg, size: OperandSize) {
        let op = match size {
            OperandSize::S8 => AvxOpcode::Vpmaxsb,
            OperandSize::S16 => AvxOpcode::Vpmaxsw,
            OperandSize::S32 => AvxOpcode::Vpmaxsd,
            _ => unimplemented!(),
        };

        self.emit(Inst::XmmRmiRVex {
            op,
            src1: lhs.into(),
            src2: XmmMemImm::unwrap_new(rhs.into()),
            dst: dst.to_reg().into(),
        })
    }

    /// Performs a max operation with vectors of unsigned integers in `lhs` and
    /// `rhs` and puts the results in `dst`.
    pub fn xmm_vpmaxu_rrr(&mut self, dst: WritableReg, lhs: Reg, rhs: Reg, size: OperandSize) {
        let op = match size {
            OperandSize::S8 => AvxOpcode::Vpmaxub,
            OperandSize::S16 => AvxOpcode::Vpmaxuw,
            OperandSize::S32 => AvxOpcode::Vpmaxud,
            _ => unimplemented!(),
        };

        self.emit(Inst::XmmRmiRVex {
            op,
            src1: lhs.into(),
            src2: XmmMemImm::unwrap_new(rhs.into()),
            dst: dst.to_reg().into(),
        })
    }

    /// Performs a min operation with vectors of signed integers in `lhs` and
    /// `rhs` and puts the results in `dst`.
    pub fn xmm_vpmins_rrr(&mut self, dst: WritableReg, lhs: Reg, rhs: Reg, size: OperandSize) {
        let op = match size {
            OperandSize::S8 => AvxOpcode::Vpminsb,
            OperandSize::S16 => AvxOpcode::Vpminsw,
            OperandSize::S32 => AvxOpcode::Vpminsd,
            _ => unimplemented!(),
        };

        self.emit(Inst::XmmRmiRVex {
            op,
            src1: lhs.into(),
            src2: XmmMemImm::unwrap_new(rhs.into()),
            dst: dst.to_reg().into(),
        })
    }

    /// Performs a min operation with vectors of unsigned integers in `lhs` and
    /// `rhs` and puts the results in `dst`.
    pub fn xmm_vpminu_rrr(&mut self, dst: WritableReg, lhs: Reg, rhs: Reg, size: OperandSize) {
        let op = match size {
            OperandSize::S8 => AvxOpcode::Vpminub,
            OperandSize::S16 => AvxOpcode::Vpminuw,
            OperandSize::S32 => AvxOpcode::Vpminud,
            _ => unimplemented!(),
        };

        self.emit(Inst::XmmRmiRVex {
            op,
            src1: lhs.into(),
            src2: XmmMemImm::unwrap_new(rhs.into()),
            dst: dst.to_reg().into(),
        })
    }

    /// Performs a comparison operation between vectors of floats in `lhs` and
    /// `rhs` and puts the results in `dst`.
    pub fn xmm_vcmpp_rrr(
        &mut self,
        dst: WritableReg,
        lhs: Reg,
        rhs: Reg,
        size: OperandSize,
        kind: VcmpKind,
    ) {
        let op = match size {
            OperandSize::S32 => AvxOpcode::Vcmpps,
            OperandSize::S64 => AvxOpcode::Vcmppd,
            _ => unimplemented!(),
        };

        self.emit(Inst::XmmRmRImmVex {
            op,
            src1: lhs.into(),
            src2: XmmMem::unwrap_new(rhs.into()),
            dst: dst.to_reg().into(),
            imm: match kind {
                VcmpKind::Eq => 0,
                VcmpKind::Lt => 1,
                VcmpKind::Le => 2,
                VcmpKind::Ne => 4,
            },
        });
    }

    /// Takes the lower lanes of vectors of floats in `src1` and `src2` and
    /// interleaves them in `dst`.
    pub fn xmm_vunpcklp_rrm(
        &mut self,
        src1: Reg,
        src2: &Address,
        dst: WritableReg,
        size: OperandSize,
    ) {
        let op = match size {
            OperandSize::S32 => AvxOpcode::Vunpcklps,
            _ => unimplemented!(),
        };

        let address = Self::to_synthetic_amode(
            src2,
            &mut self.pool,
            &mut self.constants,
            &mut self.buffer,
            MemFlags::trusted(),
        );

        self.emit(Inst::XmmRmiRVex {
            op,
            src1: src1.into(),
            src2: XmmMemImm::unwrap_new(RegMemImm::mem(address)),
            dst: dst.to_reg().into(),
        });
    }

    /// Performs a subtraction on two vectors of floats and puts the results in
    /// `dst`.
    pub fn xmm_vsub_rrm(&mut self, src1: Reg, src2: &Address, dst: WritableReg, size: OperandSize) {
        let op = match size {
            OperandSize::S64 => AvxOpcode::Vsubpd,
            _ => unimplemented!(),
        };

        let address = Self::to_synthetic_amode(
            src2,
            &mut self.pool,
            &mut self.constants,
            &mut self.buffer,
            MemFlags::trusted(),
        );

        self.emit(Inst::XmmRmiRVex {
            op,
            src1: src1.into(),
            src2: XmmMemImm::unwrap_new(RegMemImm::mem(address)),
            dst: dst.to_reg().into(),
        });
    }

    /// Performs a subtraction on two vectors of floats and puts the results in
    /// `dst`.
    pub fn xmm_vsub_rrr(&mut self, src1: Reg, src2: Reg, dst: WritableReg, size: OperandSize) {
        let op = match size {
            OperandSize::S32 => AvxOpcode::Vsubps,
            _ => unimplemented!(),
        };

        self.emit(Inst::XmmRmiRVex {
            op,
            src1: src1.into(),
            src2: src2.into(),
            dst: dst.to_reg().into(),
        });
    }

    /// Converts a vector of signed integers into a vector of narrower integers
    /// using saturation to handle overflow.
    pub fn xmm_vpackss_rrr(&mut self, src1: Reg, src2: Reg, dst: WritableReg, size: OperandSize) {
        let op = match size {
            OperandSize::S8 => AvxOpcode::Vpacksswb,
            OperandSize::S16 => AvxOpcode::Vpackssdw,
            _ => unimplemented!(),
        };

        self.emit(Inst::XmmRmiRVex {
            op,
            src1: src1.into(),
            src2: src2.into(),
            dst: dst.to_reg().into(),
        });
    }

    /// Converts a vector of unsigned integers into a vector of narrower
    /// integers using saturation to handle overflow.
    pub fn xmm_vpackus_rrr(&mut self, src1: Reg, src2: Reg, dst: WritableReg, size: OperandSize) {
        let op = match size {
            OperandSize::S8 => AvxOpcode::Vpackuswb,
            OperandSize::S16 => AvxOpcode::Vpackusdw,
            _ => unimplemented!(),
        };

        self.emit(Inst::XmmRmiRVex {
            op,
            src1: src1.into(),
            src2: src2.into(),
            dst: dst.to_reg().into(),
        });
    }

    /// Concatenates `src1` and `src2` and shifts right by `imm` and puts
    /// result in `dst`.
    pub fn xmm_vpalignr_rrr(&mut self, src1: Reg, src2: Reg, dst: WritableReg, imm: u8) {
        self.emit(Inst::XmmRmRImmVex {
            op: AvxOpcode::Vpalignr,
            src1: src1.into(),
            src2: src2.into(),
            dst: dst.to_reg().into(),
            imm,
        })
    }

    /// Unpacks and interleaves the higher lanes of vectors of integers in
    /// `src1` and `src2` and puts the results in `dst`.
    pub fn xmm_vpunpckh_rrr(&mut self, src1: Reg, src2: Reg, dst: WritableReg, size: OperandSize) {
        let op = match size {
            OperandSize::S8 => AvxOpcode::Vpunpckhbw,
            OperandSize::S16 => AvxOpcode::Vpunpckhwd,
            _ => unimplemented!(),
        };

        self.emit(Inst::XmmRmiRVex {
            op,
            src1: src1.into(),
            src2: src2.into(),
            dst: dst.to_reg().into(),
        });
    }

    /// Bitwise logical xor of vectors of floats in `src1` and `src2` and puts
    /// the results in `dst`.
    pub fn xmm_vxorp_rrr(&mut self, src1: Reg, src2: Reg, dst: WritableReg, size: OperandSize) {
        let op = match size {
            OperandSize::S32 => AvxOpcode::Vxorps,
            OperandSize::S64 => AvxOpcode::Vxorpd,
            _ => unimplemented!(),
        };

        self.emit(Inst::XmmRmiRVex {
            op,
            src1: src1.into(),
            src2: src2.into(),
            dst: dst.to_reg().into(),
        });
    }

    /// Unpacks and interleaves high order data of floats in `src1` and `src2`
    /// and puts the results in `dst`.
    pub fn xmm_vunpckhp_rrr(&mut self, src1: Reg, src2: Reg, dst: WritableReg, size: OperandSize) {
        let op = match size {
            OperandSize::S32 => AvxOpcode::Vunpckhps,
            _ => unimplemented!(),
        };

        self.emit(Inst::XmmRmiRVex {
            op,
            src1: src1.into(),
            src2: src2.into(),
            dst: dst.to_reg().into(),
        });
    }

    pub(crate) fn xmm_rm_rvex3(
        &mut self,
        op: Avx512Opcode,
        src1: Reg,
        src2: Reg,
        dst: WritableReg,
    ) {
        self.emit(Inst::XmmRmREvex3 {
            op,
            // `src1` reuses `dst`, and is ignored in emission
            src1: dst.to_reg().into(),
            src2: src1.into(),
            src3: src2.into(),
            dst: dst.map(Into::into),
        });
    }

    /// Creates a mask made up of the most significant bit of each byte of
    /// `src` and stores the result in `dst`.
    pub fn xmm_vpmovmsk_rr(
        &mut self,
        src: Reg,
        dst: WritableReg,
        src_size: OperandSize,
        dst_size: OperandSize,
    ) {
        let op = match src_size {
            OperandSize::S8 => AvxOpcode::Vpmovmskb,
            _ => unimplemented!(),
        };

        self.emit(Inst::XmmToGprVex {
            op,
            src: src.into(),
            dst: dst.to_reg().into(),
            dst_size: dst_size.into(),
        });
    }

    /// Creates a mask made up of the most significant bit of each byte of
    /// in `src` and stores the result in `dst`.
    pub fn xmm_vmovskp_rr(
        &mut self,
        src: Reg,
        dst: WritableReg,
        src_size: OperandSize,
        dst_size: OperandSize,
    ) {
        let op = match src_size {
            OperandSize::S32 => AvxOpcode::Vmovmskps,
            OperandSize::S64 => AvxOpcode::Vmovmskpd,
            _ => unimplemented!(),
        };

        self.emit(Inst::XmmToGprVex {
            op,
            src: src.into(),
            dst: dst.to_reg().into(),
            dst_size: dst_size.into(),
        })
    }

    /// Compute the absolute value of elements in vector `src` and put the
    /// results in `dst`.
    pub fn xmm_vpabs_rr(&mut self, src: Reg, dst: WritableReg, size: OperandSize) {
        let op = match size {
            OperandSize::S8 => AvxOpcode::Vpabsb,
            OperandSize::S16 => AvxOpcode::Vpabsw,
            OperandSize::S32 => AvxOpcode::Vpabsd,
            _ => unimplemented!(),
        };

        self.emit(Inst::XmmUnaryRmRVex {
            op,
            src: src.into(),
            dst: dst.to_reg().into(),
        });
    }

    /// Arithmetically (sign preserving) right shift on vector in `src` by
    /// `imm` with result written to `dst`.
    pub fn xmm_vpsra_rri(&mut self, src: Reg, dst: WritableReg, imm: u32, size: OperandSize) {
        let op = match size {
            OperandSize::S32 => AvxOpcode::Vpsrad,
            _ => unimplemented!(),
        };

        self.emit(Inst::XmmRmiRVex {
            op,
            src1: src.into(),
            src2: XmmMemImm::unwrap_new(RegMemImm::imm(imm)),
            dst: dst.to_reg().into(),
        });
    }

    fn vandp_opcode(size: OperandSize) -> AvxOpcode {
        match size {
            OperandSize::S32 => AvxOpcode::Vandps,
            OperandSize::S64 => AvxOpcode::Vandpd,
            _ => unimplemented!(),
        }
    }

    /// Perform an `and` operation on vectors of floats in `src1` and `src2`
    /// and put the results in `dst`.
    pub fn xmm_vandp_rrm(
        &mut self,
        src1: Reg,
        src2: &Address,
        dst: WritableReg,
        size: OperandSize,
    ) {
        let address = Self::to_synthetic_amode(
            src2,
            &mut self.pool,
            &mut self.constants,
            &mut self.buffer,
            MemFlags::trusted(),
        );

        self.emit(Inst::XmmRmiRVex {
            op: Self::vandp_opcode(size),
            src1: src1.into(),
            src2: XmmMemImm::unwrap_new(RegMemImm::mem(address)),
            dst: dst.to_reg().into(),
        });
    }

    /// Perform an `and` operation on vectors of floats in `src1` and `src2`
    /// and put the results in `dst`.
    pub fn xmm_vandp_rrr(&mut self, src1: Reg, src2: Reg, dst: WritableReg, size: OperandSize) {
        self.emit(Inst::XmmRmiRVex {
            op: Self::vandp_opcode(size),
            src1: src1.into(),
            src2: src2.into(),
            dst: dst.to_reg().into(),
        });
    }

    /// Perform a max operation across two vectors of floats and put the
    /// results in `dst`.
    pub fn xmm_vmaxp_rrr(&mut self, src1: Reg, src2: Reg, dst: WritableReg, size: OperandSize) {
        let op = match size {
            OperandSize::S32 => AvxOpcode::Vmaxps,
            OperandSize::S64 => AvxOpcode::Vmaxpd,
            _ => unimplemented!(),
        };

        self.emit(Inst::XmmRmiRVex {
            op,
            src1: src1.into(),
            src2: src2.into(),
            dst: dst.to_reg().into(),
        });
    }

    fn vminp_opcode(size: OperandSize) -> AvxOpcode {
        match size {
            OperandSize::S32 => AvxOpcode::Vminps,
            OperandSize::S64 => AvxOpcode::Vminpd,
            _ => unimplemented!(),
        }
    }

    // Perform a min operation across two vectors of floats and put the
    // results in `dst`.
    pub fn xmm_vminp_rrm(
        &mut self,
        src1: Reg,
        src2: &Address,
        dst: WritableReg,
        size: OperandSize,
    ) {
        let address = Self::to_synthetic_amode(
            src2,
            &mut self.pool,
            &mut self.constants,
            &mut self.buffer,
            MemFlags::trusted(),
        );

        self.emit(Inst::XmmRmiRVex {
            op: Self::vminp_opcode(size),
            src1: src1.into(),
            src2: XmmMemImm::unwrap_new(RegMemImm::mem(address)),
            dst: dst.to_reg().into(),
        });
    }

    // Perform a min operation across two vectors of floats and put the
    // results in `dst`.
    pub fn xmm_vminp_rrr(&mut self, src1: Reg, src2: Reg, dst: WritableReg, size: OperandSize) {
        self.emit(Inst::XmmRmiRVex {
            op: Self::vminp_opcode(size),
            src1: src1.into(),
            src2: src2.into(),
            dst: dst.to_reg().into(),
        });
    }

    // Round a vector of floats toward 0.
    pub fn xmm_vroundp_rri(
        &mut self,
        src: Reg,
        dst: WritableReg,
        mode: VroundMode,
        size: OperandSize,
    ) {
        let op = match size {
            OperandSize::S64 => AvxOpcode::Vroundpd,
            _ => unimplemented!(),
        };

        self.emit(Inst::XmmUnaryRmRImmVex {
            op,
            src: src.into(),
            dst: dst.to_reg().into(),
            imm: match mode {
                VroundMode::TowardZero => 3,
            },
        });
    }

    /// Shuffle of vectors of floats.
    pub fn xmm_vshufp_rrri(
        &mut self,
        src1: Reg,
        src2: Reg,
        dst: WritableReg,
        imm: u8,
        size: OperandSize,
    ) {
        let op = match size {
            OperandSize::S32 => AvxOpcode::Vshufps,
            _ => unimplemented!(),
        };

        self.emit(Inst::XmmRmRImmVex {
            op,
            src1: src1.into(),
            src2: src2.into(),
            dst: dst.to_reg().into(),
            imm,
        });
    }

    /// Each lane in `src1` is multiplied by the corresponding lane in `src2`
    /// producing intermediate 32-bit operands. Each intermediate 32-bit
    /// operand is truncated to 18 most significant bits. Rounding is performed
    /// by adding 1 to the least significant bit of the 18-bit intermediate
    /// result. The 16 bits immediately to the right of the most significant
    /// bit of each 18-bit intermediate result is placed in each lane of `dst`.
    pub fn xmm_vpmulhrs_rrr(&mut self, src1: Reg, src2: Reg, dst: WritableReg, size: OperandSize) {
        let op = match size {
            OperandSize::S16 => AvxOpcode::Vpmulhrsw,
            _ => unimplemented!(),
        };

        self.emit(Inst::XmmRmiRVex {
            op,
            src1: src1.into(),
            src2: src2.into(),
            dst: dst.to_reg().into(),
        });
    }
}

/// Captures the region in a MachBuffer where an add-with-immediate instruction would be emitted,
/// but the immediate is not yet known. Currently, this implementation expects a 32-bit immediate,
/// so 8 and 16 bit operand sizes are not supported.
pub(crate) struct PatchableAddToReg {
    /// The region to be patched in the [`MachBuffer`]. It must contain a valid add instruction
    /// sequence, accepting a 32-bit immediate.
    region: PatchRegion,

    /// The offset into the patchable region where the patchable constant begins.
    constant_offset: usize,
}

impl PatchableAddToReg {
    /// Create a new [`PatchableAddToReg`] by capturing a region in the output buffer where the
    /// add-with-immediate occurs. The [`MachBuffer`] will have and add-with-immediate instruction
    /// present in that region, though it will add `0` until the `::finalize` method is called.
    ///
    /// Currently this implementation expects to be able to patch a 32-bit immediate, which means
    /// that 8 and 16-bit addition cannot be supported.
    pub(crate) fn new(reg: Reg, size: OperandSize, buf: &mut MachBuffer<Inst>) -> Self {
        let open = buf.start_patchable();

        // Emit the opcode and register use for the add instruction.
        let start = buf.cur_offset();
        Self::add_inst_bytes(reg, size, buf);
        let constant_offset = usize::try_from(buf.cur_offset() - start).unwrap();

        // Emit a placeholder for the 32-bit immediate.
        buf.put4(0);

        let region = buf.end_patchable(open);

        Self {
            region,
            constant_offset,
        }
    }

    /// Generate the prefix of the add instruction (rex byte (depending on register use), opcode,
    /// and register reference).
    fn add_inst_bytes(reg: Reg, size: OperandSize, buf: &mut MachBuffer<Inst>) {
        match size {
            OperandSize::S32 | OperandSize::S64 => {}
            _ => {
                panic!(
                    "{}-bit addition is not supported, please see the comment on PatchableAddToReg::new",
                    size.num_bits(),
                )
            }
        }

        let enc_g = 0;

        debug_assert!(reg.is_int());
        let enc_e = u8::try_from(reg.hw_enc()).unwrap();

        RexFlags::from(args::OperandSize::from(size)).emit_two_op(buf, enc_g, enc_e);

        // the opcode for an add
        buf.put1(0x81);

        // the modrm byte
        buf.put1(encode_modrm(0b11, enc_g & 7, enc_e & 7));
    }

    /// Patch the [`MachBuffer`] with the known constant to be added to the register. The final
    /// value is passed in as an i32, but the instruction encoding is fixed when
    /// [`PatchableAddToReg::new`] is called.
    pub(crate) fn finalize(self, val: i32, buffer: &mut MachBuffer<Inst>) {
        let slice = self.region.patch(buffer);
        debug_assert_eq!(slice.len(), self.constant_offset + 4);
        slice[self.constant_offset..].copy_from_slice(val.to_le_bytes().as_slice());
    }
}
