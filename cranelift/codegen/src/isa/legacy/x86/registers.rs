//! x86 register descriptions.

use crate::isa::registers::{RegBank, RegClass, RegClassData, RegInfo, RegUnit};

include!(concat!(env!("OUT_DIR"), "/registers-x86.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    use crate::isa::RegUnit;
    use alloc::string::{String, ToString};

    #[test]
    fn unit_encodings() {
        fn gpr(unit: usize) -> Option<u16> {
            Some(GPR.unit(unit))
        }
        // The encoding of integer registers is not alphabetical.
        assert_eq!(INFO.parse_regunit("rax"), gpr(0));
        assert_eq!(INFO.parse_regunit("rbx"), gpr(3));
        assert_eq!(INFO.parse_regunit("rcx"), gpr(1));
        assert_eq!(INFO.parse_regunit("rdx"), gpr(2));
        assert_eq!(INFO.parse_regunit("rsi"), gpr(6));
        assert_eq!(INFO.parse_regunit("rdi"), gpr(7));
        assert_eq!(INFO.parse_regunit("rbp"), gpr(5));
        assert_eq!(INFO.parse_regunit("rsp"), gpr(4));
        assert_eq!(INFO.parse_regunit("r8"), gpr(8));
        assert_eq!(INFO.parse_regunit("r15"), gpr(15));

        fn fpr(unit: usize) -> Option<u16> {
            Some(FPR.unit(unit))
        }
        assert_eq!(INFO.parse_regunit("xmm0"), fpr(0));
        assert_eq!(INFO.parse_regunit("xmm15"), fpr(15));

        // FIXME(#1306) Add these tests back in when FPR32 is re-added.
        // fn fpr32(unit: usize) -> Option<u16> {
        //    Some(FPR32.unit(unit))
        // }
        // assert_eq!(INFO.parse_regunit("xmm0"), fpr32(0));
        // assert_eq!(INFO.parse_regunit("xmm31"), fpr32(31));
    }

    #[test]
    fn unit_names() {
        fn gpr(ru: RegUnit) -> String {
            INFO.display_regunit(GPR.first + ru).to_string()
        }
        assert_eq!(gpr(0), "%rax");
        assert_eq!(gpr(3), "%rbx");
        assert_eq!(gpr(1), "%rcx");
        assert_eq!(gpr(2), "%rdx");
        assert_eq!(gpr(6), "%rsi");
        assert_eq!(gpr(7), "%rdi");
        assert_eq!(gpr(5), "%rbp");
        assert_eq!(gpr(4), "%rsp");
        assert_eq!(gpr(8), "%r8");
        assert_eq!(gpr(15), "%r15");

        fn fpr(ru: RegUnit) -> String {
            INFO.display_regunit(FPR.first + ru).to_string()
        }
        assert_eq!(fpr(0), "%xmm0");
        assert_eq!(fpr(15), "%xmm15");

        // FIXME(#1306) Add these tests back in when FPR32 is re-added.
        // fn fpr32(ru: RegUnit) -> String {
        //    INFO.display_regunit(FPR32.first + ru).to_string()
        // }
        // assert_eq!(fpr32(0), "%xmm0");
        // assert_eq!(fpr32(31), "%xmm31");
    }

    #[test]
    fn regclasses() {
        assert_eq!(GPR.intersect_index(GPR), Some(GPR.into()));
        assert_eq!(GPR.intersect_index(ABCD), Some(ABCD.into()));
        assert_eq!(GPR.intersect_index(FPR), None);
        assert_eq!(ABCD.intersect_index(GPR), Some(ABCD.into()));
        assert_eq!(ABCD.intersect_index(ABCD), Some(ABCD.into()));
        assert_eq!(ABCD.intersect_index(FPR), None);
        assert_eq!(FPR.intersect_index(FPR), Some(FPR.into()));
        assert_eq!(FPR.intersect_index(GPR), None);
        assert_eq!(FPR.intersect_index(ABCD), None);
    }
}
