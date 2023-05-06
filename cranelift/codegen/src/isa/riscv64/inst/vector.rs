use crate::isa::riscv64::inst::AllocationConsumer;
use crate::isa::riscv64::inst::EmitState;
use crate::isa::riscv64::lower::isle::generated_code::{
    VecAMode, VecAluOpRRImm5, VecAluOpRRR, VecAvl, VecElementWidth, VecLmul, VecMaskMode,
    VecOpCategory, VecOpMasking, VecTailMode,
};
use crate::Reg;
use core::fmt;

use super::{Type, UImm5};

impl VecAvl {
    pub fn _static(size: u32) -> Self {
        VecAvl::Static {
            size: UImm5::maybe_from_u8(size as u8).expect("Invalid size for AVL"),
        }
    }

    pub fn is_static(&self) -> bool {
        match self {
            VecAvl::Static { .. } => true,
        }
    }

    pub fn unwrap_static(&self) -> UImm5 {
        match self {
            VecAvl::Static { size } => *size,
        }
    }
}

// TODO: Can we tell ISLE to derive this?
impl PartialEq for VecAvl {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (VecAvl::Static { size: lhs }, VecAvl::Static { size: rhs }) => lhs == rhs,
        }
    }
}

impl fmt::Display for VecAvl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            VecAvl::Static { size } => write!(f, "{}", size),
        }
    }
}

impl VecElementWidth {
    pub fn from_type(ty: Type) -> Self {
        Self::from_bits(ty.lane_bits())
    }

    pub fn from_bits(bits: u32) -> Self {
        match bits {
            8 => VecElementWidth::E8,
            16 => VecElementWidth::E16,
            32 => VecElementWidth::E32,
            64 => VecElementWidth::E64,
            _ => panic!("Invalid number of bits for VecElementWidth: {}", bits),
        }
    }

    pub fn bits(&self) -> u32 {
        match self {
            VecElementWidth::E8 => 8,
            VecElementWidth::E16 => 16,
            VecElementWidth::E32 => 32,
            VecElementWidth::E64 => 64,
        }
    }

    pub fn encode(&self) -> u32 {
        match self {
            VecElementWidth::E8 => 0b000,
            VecElementWidth::E16 => 0b001,
            VecElementWidth::E32 => 0b010,
            VecElementWidth::E64 => 0b011,
        }
    }
}

impl fmt::Display for VecElementWidth {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "e{}", self.bits())
    }
}

impl VecLmul {
    pub fn encode(&self) -> u32 {
        match self {
            VecLmul::LmulF8 => 0b101,
            VecLmul::LmulF4 => 0b110,
            VecLmul::LmulF2 => 0b111,
            VecLmul::Lmul1 => 0b000,
            VecLmul::Lmul2 => 0b001,
            VecLmul::Lmul4 => 0b010,
            VecLmul::Lmul8 => 0b011,
        }
    }
}

impl fmt::Display for VecLmul {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            VecLmul::LmulF8 => write!(f, "mf8"),
            VecLmul::LmulF4 => write!(f, "mf4"),
            VecLmul::LmulF2 => write!(f, "mf2"),
            VecLmul::Lmul1 => write!(f, "m1"),
            VecLmul::Lmul2 => write!(f, "m2"),
            VecLmul::Lmul4 => write!(f, "m4"),
            VecLmul::Lmul8 => write!(f, "m8"),
        }
    }
}

impl VecTailMode {
    pub fn encode(&self) -> u32 {
        match self {
            VecTailMode::Agnostic => 1,
            VecTailMode::Undisturbed => 0,
        }
    }
}

impl fmt::Display for VecTailMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            VecTailMode::Agnostic => write!(f, "ta"),
            VecTailMode::Undisturbed => write!(f, "tu"),
        }
    }
}

impl VecMaskMode {
    pub fn encode(&self) -> u32 {
        match self {
            VecMaskMode::Agnostic => 1,
            VecMaskMode::Undisturbed => 0,
        }
    }
}

impl fmt::Display for VecMaskMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            VecMaskMode::Agnostic => write!(f, "ma"),
            VecMaskMode::Undisturbed => write!(f, "mu"),
        }
    }
}

/// Vector Type (VType)
///
/// vtype provides the default type used to interpret the contents of the vector register file.
#[derive(Clone, Debug, PartialEq)]
pub struct VType {
    pub sew: VecElementWidth,
    pub lmul: VecLmul,
    pub tail_mode: VecTailMode,
    pub mask_mode: VecMaskMode,
}

impl VType {
    // https://github.com/riscv/riscv-v-spec/blob/master/vtype-format.adoc
    pub fn encode(&self) -> u32 {
        let mut bits = 0;
        bits |= self.lmul.encode();
        bits |= self.sew.encode() << 3;
        bits |= self.tail_mode.encode() << 6;
        bits |= self.mask_mode.encode() << 7;
        bits
    }
}

impl fmt::Display for VType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}, {}, {}, {}",
            self.sew, self.lmul, self.tail_mode, self.mask_mode
        )
    }
}

/// Vector State (VState)
///
/// VState represents the state of the vector unit that each instruction expects before execution.
/// Unlike VType or any of the other types here, VState is not a part of the RISC-V ISA. It is
/// used by our instruction emission code to ensure that the vector unit is in the correct state.
#[derive(Clone, Debug, PartialEq)]
pub struct VState {
    pub avl: VecAvl,
    pub vtype: VType,
}

impl VState {
    pub fn from_type(ty: Type) -> Self {
        VState {
            avl: VecAvl::_static(ty.lane_count()),
            vtype: VType {
                sew: VecElementWidth::from_type(ty),
                lmul: VecLmul::Lmul1,
                tail_mode: VecTailMode::Agnostic,
                mask_mode: VecMaskMode::Agnostic,
            },
        }
    }
}

impl fmt::Display for VState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "#avl={}, #vtype=({})", self.avl, self.vtype)
    }
}

impl VecOpCategory {
    pub fn encode(&self) -> u32 {
        // See: https://github.com/riscv/riscv-v-spec/blob/master/v-spec.adoc#101-vector-arithmetic-instruction-encoding
        match self {
            VecOpCategory::OPIVV => 0b000,
            VecOpCategory::OPFVV => 0b001,
            VecOpCategory::OPMVV => 0b010,
            VecOpCategory::OPIVI => 0b011,
            VecOpCategory::OPIVX => 0b100,
            VecOpCategory::OPFVF => 0b101,
            VecOpCategory::OPMVX => 0b110,
            VecOpCategory::OPCFG => 0b111,
        }
    }
}

impl VecOpMasking {
    pub fn encode(&self) -> u32 {
        match self {
            VecOpMasking::Enabled => 0,
            VecOpMasking::Disabled => 1,
        }
    }
}

impl VecAluOpRRR {
    pub fn opcode(&self) -> u32 {
        // Vector Opcode
        0x57
    }
    pub fn funct3(&self) -> u32 {
        match self {
            VecAluOpRRR::VaddVV
            | VecAluOpRRR::VsubVV
            | VecAluOpRRR::VandVV
            | VecAluOpRRR::VorVV
            | VecAluOpRRR::VxorVV => VecOpCategory::OPIVV,
            VecAluOpRRR::VmulVV | VecAluOpRRR::VmulhVV | VecAluOpRRR::VmulhuVV => {
                VecOpCategory::OPMVV
            }
        }
        .encode()
    }
    pub fn funct6(&self) -> u32 {
        // See: https://github.com/riscv/riscv-v-spec/blob/master/inst-table.adoc
        match self {
            VecAluOpRRR::VaddVV => 0b000000,
            VecAluOpRRR::VsubVV => 0b000010,
            VecAluOpRRR::VmulVV => 0b100101,
            VecAluOpRRR::VmulhVV => 0b100111,
            VecAluOpRRR::VmulhuVV => 0b100100,
            VecAluOpRRR::VandVV => 0b001001,
            VecAluOpRRR::VorVV => 0b001010,
            VecAluOpRRR::VxorVV => 0b001011,
        }
    }
}

impl fmt::Display for VecAluOpRRR {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut s = format!("{self:?}");
        s.make_ascii_lowercase();
        let (opcode, category) = s.split_at(s.len() - 2);
        f.write_str(&format!("{}.{}", opcode, category))
    }
}

impl VecAluOpRRImm5 {
    pub fn opcode(&self) -> u32 {
        // Vector Opcode
        0x57
    }
    pub fn funct3(&self) -> u32 {
        VecOpCategory::OPIVI.encode()
    }
    pub fn funct6(&self) -> u32 {
        // See: https://github.com/riscv/riscv-v-spec/blob/master/inst-table.adoc
        match self {
            VecAluOpRRImm5::VaddVI => 0b000000,
        }
    }
}

impl fmt::Display for VecAluOpRRImm5 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut s = format!("{self:?}");
        s.make_ascii_lowercase();
        let (opcode, category) = s.split_at(s.len() - 2);
        f.write_str(&format!("{}.{}", opcode, category))
    }
}

impl VecAMode {
    pub fn get_base_register(&self) -> Option<Reg> {
        match self {
            VecAMode::UnitStride { base, .. } => base.get_base_register(),
        }
    }

    pub fn get_allocatable_register(&self) -> Option<Reg> {
        match self {
            VecAMode::UnitStride { base, .. } => base.get_allocatable_register(),
        }
    }

    pub(crate) fn with_allocs(self, allocs: &mut AllocationConsumer<'_>) -> Self {
        match self {
            VecAMode::UnitStride { base } => VecAMode::UnitStride {
                base: base.with_allocs(allocs),
            },
        }
    }

    pub(crate) fn get_offset_with_state(&self, state: &EmitState) -> i64 {
        match self {
            VecAMode::UnitStride { base, .. } => base.get_offset_with_state(state),
        }
    }

    /// `mop` field, described in Table 7 of Section 7.2. Vector Load/Store Addressing Modes
    /// https://github.com/riscv/riscv-v-spec/blob/master/v-spec.adoc#72-vector-loadstore-addressing-modes
    pub fn mop(&self) -> u32 {
        match self {
            VecAMode::UnitStride { .. } => 0b00,
        }
    }

    /// `lumop` field, described in Table 9 of Section 7.2. Vector Load/Store Addressing Modes
    /// https://github.com/riscv/riscv-v-spec/blob/master/v-spec.adoc#72-vector-loadstore-addressing-modes
    pub fn lumop(&self) -> u32 {
        match self {
            VecAMode::UnitStride { .. } => 0b00000,
        }
    }

    /// `sumop` field, described in Table 10 of Section 7.2. Vector Load/Store Addressing Modes
    /// https://github.com/riscv/riscv-v-spec/blob/master/v-spec.adoc#72-vector-loadstore-addressing-modes
    pub fn sumop(&self) -> u32 {
        match self {
            VecAMode::UnitStride { .. } => 0b00000,
        }
    }

    /// The `nf[2:0]` field encodes the number of fields in each segment. For regular vector loads and
    /// stores, nf=0, indicating that a single value is moved between a vector register group and memory
    /// at each element position. Larger values in the nf field are used to access multiple contiguous
    /// fields within a segment as described in Section 7.8 Vector Load/Store Segment Instructions.
    ///
    /// https://github.com/riscv/riscv-v-spec/blob/master/v-spec.adoc#72-vector-loadstore-addressing-modes
    pub fn nf(&self) -> u32 {
        match self {
            VecAMode::UnitStride { .. } => 0b000,
        }
    }
}
