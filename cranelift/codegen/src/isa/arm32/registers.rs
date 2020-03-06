//! ARM32 register descriptions.

use crate::isa::registers::{RegBank, RegClass, RegClassData, RegInfo, RegUnit};

include!(concat!(env!("OUT_DIR"), "/registers-arm32.rs"));

#[cfg(test)]
mod tests {
    use super::{D, GPR, INFO, S};
    use crate::isa::RegUnit;
    use alloc::string::{String, ToString};

    #[test]
    fn unit_encodings() {
        assert_eq!(INFO.parse_regunit("s0"), Some(0));
        assert_eq!(INFO.parse_regunit("s31"), Some(31));
        assert_eq!(INFO.parse_regunit("s32"), Some(32));
        assert_eq!(INFO.parse_regunit("r0"), Some(64));
        assert_eq!(INFO.parse_regunit("r15"), Some(79));
    }

    #[test]
    fn unit_names() {
        fn uname(ru: RegUnit) -> String {
            INFO.display_regunit(ru).to_string()
        }

        assert_eq!(uname(0), "%s0");
        assert_eq!(uname(1), "%s1");
        assert_eq!(uname(31), "%s31");
        assert_eq!(uname(64), "%r0");
    }

    #[test]
    fn overlaps() {
        // arm32 has the most interesting register geometries, so test `regs_overlap()` here.
        use crate::isa::regs_overlap;

        let r0 = GPR.unit(0);
        let r1 = GPR.unit(1);
        let r2 = GPR.unit(2);

        assert!(regs_overlap(GPR, r0, GPR, r0));
        assert!(regs_overlap(GPR, r2, GPR, r2));
        assert!(!regs_overlap(GPR, r0, GPR, r1));
        assert!(!regs_overlap(GPR, r1, GPR, r0));
        assert!(!regs_overlap(GPR, r2, GPR, r1));
        assert!(!regs_overlap(GPR, r1, GPR, r2));

        let s0 = S.unit(0);
        let s1 = S.unit(1);
        let s2 = S.unit(2);
        let s3 = S.unit(3);
        let d0 = D.unit(0);
        let d1 = D.unit(1);

        assert!(regs_overlap(S, s0, D, d0));
        assert!(regs_overlap(S, s1, D, d0));
        assert!(!regs_overlap(S, s0, D, d1));
        assert!(!regs_overlap(S, s1, D, d1));
        assert!(regs_overlap(S, s2, D, d1));
        assert!(regs_overlap(S, s3, D, d1));
        assert!(!regs_overlap(D, d1, S, s1));
        assert!(regs_overlap(D, d1, S, s2));
        assert!(!regs_overlap(D, d0, D, d1));
        assert!(regs_overlap(D, d1, D, d1));
    }
}
