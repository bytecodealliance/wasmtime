//! RISC-V register descriptions.

use crate::isa::registers::{RegBank, RegClass, RegClassData, RegInfo, RegUnit};

include!(concat!(env!("OUT_DIR"), "/registers-riscv.rs"));

#[cfg(test)]
mod tests {
    use super::{FPR, GPR, INFO};
    use crate::isa::RegUnit;
    use alloc::string::{String, ToString};

    #[test]
    fn unit_encodings() {
        assert_eq!(INFO.parse_regunit("x0"), Some(0));
        assert_eq!(INFO.parse_regunit("x31"), Some(31));
        assert_eq!(INFO.parse_regunit("f0"), Some(32));
        assert_eq!(INFO.parse_regunit("f31"), Some(63));

        assert_eq!(INFO.parse_regunit("x32"), None);
        assert_eq!(INFO.parse_regunit("f32"), None);
    }

    #[test]
    fn unit_names() {
        fn uname(ru: RegUnit) -> String {
            INFO.display_regunit(ru).to_string()
        }

        assert_eq!(uname(0), "%x0");
        assert_eq!(uname(1), "%x1");
        assert_eq!(uname(31), "%x31");
        assert_eq!(uname(32), "%f0");
        assert_eq!(uname(33), "%f1");
        assert_eq!(uname(63), "%f31");
        assert_eq!(uname(64), "%INVALID64");
    }

    #[test]
    fn classes() {
        assert!(GPR.contains(GPR.unit(0)));
        assert!(GPR.contains(GPR.unit(31)));
        assert!(!FPR.contains(GPR.unit(0)));
        assert!(!FPR.contains(GPR.unit(31)));
        assert!(!GPR.contains(FPR.unit(0)));
        assert!(!GPR.contains(FPR.unit(31)));
        assert!(FPR.contains(FPR.unit(0)));
        assert!(FPR.contains(FPR.unit(31)));
    }
}
