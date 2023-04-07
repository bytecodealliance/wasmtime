use crate::isa::riscv64::lower::isle::generated_code::{
    VecAluOpRRR, VecAvl, VecLmul, VecMaskMode, VecSew, VecTailMode,
};
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

impl VecSew {
    pub fn from_type(ty: Type) -> Self {
        Self::from_bits(ty.lane_bits())
    }

    pub fn from_bits(bits: u32) -> Self {
        match bits {
            8 => VecSew::E8,
            16 => VecSew::E16,
            32 => VecSew::E32,
            64 => VecSew::E64,
            _ => panic!("Invalid number of bits for VecSew: {}", bits),
        }
    }

    pub fn bits(&self) -> u32 {
        match self {
            VecSew::E8 => 8,
            VecSew::E16 => 16,
            VecSew::E32 => 32,
            VecSew::E64 => 64,
        }
    }

    pub fn encode(&self) -> u32 {
        match self {
            VecSew::E8 => 0b000,
            VecSew::E16 => 0b001,
            VecSew::E32 => 0b010,
            VecSew::E64 => 0b011,
        }
    }
}

impl fmt::Display for VecSew {
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
    pub sew: VecSew,
    pub lmul: VecLmul,
    pub tail_mode: VecTailMode,
    pub mask_mode: VecMaskMode,
}

impl VType {
    // https://github.com/riscv/riscv-v-spec/blob/master/vtype-format.adoc
    pub fn encode(&self) -> u32 {
        let mut bits = 0;
        bits |= self.sew.encode();
        bits |= self.lmul.encode() << 3;
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
                sew: VecSew::from_type(ty),
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

impl VecAluOpRRR {
    pub fn opcode(&self) -> u32 {
        match self {
            VecAluOpRRR::Vadd => 0x57,
        }
    }
    pub fn funct3(&self) -> u32 {
        match self {
            VecAluOpRRR::Vadd => 0b000,
        }
    }
    pub fn funct6(&self) -> u32 {
        match self {
            VecAluOpRRR::Vadd => 0b000000,
        }
    }
}

impl fmt::Display for VecAluOpRRR {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            VecAluOpRRR::Vadd => write!(f, "vadd.vv"),
        }
    }
}
