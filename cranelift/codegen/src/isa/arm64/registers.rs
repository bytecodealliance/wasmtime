//! ARM64 register descriptions.

use crate::isa::registers::{RegBank, RegClass, RegClassData, RegInfo, RegUnit};

include!(concat!(env!("OUT_DIR"), "/registers-arm64.rs"));

#[cfg(test)]
mod tests {
    use super::INFO;
    use crate::isa::RegUnit;
    use alloc::string::{String, ToString};

    #[test]
    fn unit_encodings() {
        assert_eq!(INFO.parse_regunit("x0"), Some(0));
        assert_eq!(INFO.parse_regunit("x31"), Some(31));
        assert_eq!(INFO.parse_regunit("v0"), Some(32));
        assert_eq!(INFO.parse_regunit("v31"), Some(63));

        assert_eq!(INFO.parse_regunit("x32"), None);
        assert_eq!(INFO.parse_regunit("v32"), None);
    }

    #[test]
    fn unit_names() {
        fn uname(ru: RegUnit) -> String {
            INFO.display_regunit(ru).to_string()
        }

        assert_eq!(uname(0), "%x0");
        assert_eq!(uname(1), "%x1");
        assert_eq!(uname(31), "%x31");
        assert_eq!(uname(32), "%v0");
        assert_eq!(uname(33), "%v1");
        assert_eq!(uname(63), "%v31");
        assert_eq!(uname(64), "%nzcv");
        assert_eq!(uname(65), "%INVALID65");
    }
}
