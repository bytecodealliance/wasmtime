//! ARM32 register descriptions.

use isa::registers::{RegBank, RegClass, RegClassData, RegInfo};

include!(concat!(env!("OUT_DIR"), "/registers-arm32.rs"));

#[cfg(test)]
mod tests {
    use super::INFO;
    use isa::RegUnit;

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
}
