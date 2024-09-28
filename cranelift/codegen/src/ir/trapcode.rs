//! Trap codes describing the reason for a trap.

use core::fmt::{self, Display, Formatter};
use core::num::NonZeroU8;
use core::str::FromStr;
#[cfg(feature = "enable-serde")]
use serde_derive::{Deserialize, Serialize};

/// A trap code describing the reason for a trap.
///
/// All trap instructions have an explicit trap code.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub struct TrapCode(NonZeroU8);

impl TrapCode {
    /// Number of reserved opcodes for Cranelift itself. This number of traps are
    /// defined below starting at the high end of the byte space (e.g. 255, 254,
    /// ...)
    const RESERVED: u8 = 5;
    const RESERVED_START: u8 = u8::MAX - Self::RESERVED + 1;

    /// Internal helper to create new reserved trap codes.
    const fn reserved(byte: u8) -> TrapCode {
        match NonZeroU8::new(byte + Self::RESERVED_START) {
            Some(nz) => TrapCode(nz),
            None => panic!("invalid reserved opcode"),
        }
    }

    /// The current stack space was exhausted.
    pub const STACK_OVERFLOW: TrapCode = TrapCode::reserved(0);
    /// An integer arithmetic operation caused an overflow.
    pub const INTEGER_OVERFLOW: TrapCode = TrapCode::reserved(1);
    /// A `heap_addr` instruction detected an out-of-bounds error.
    ///
    /// Note that not all out-of-bounds heap accesses are reported this way;
    /// some are detected by a segmentation fault on the heap unmapped or
    /// offset-guard pages.
    pub const HEAP_OUT_OF_BOUNDS: TrapCode = TrapCode::reserved(2);

    /// An integer division by zero.
    pub const INTEGER_DIVISION_BY_ZERO: TrapCode = TrapCode::reserved(3);

    /// Failed float-to-int conversion.
    pub const BAD_CONVERSION_TO_INTEGER: TrapCode = TrapCode::reserved(4);

    /// Create a user-defined trap code.
    pub const fn user(code: u8) -> TrapCode {
        assert!(
            code < Self::RESERVED_START,
            "user code collides with built-in trap code"
        );
        match NonZeroU8::new(code) {
            Some(nz) => TrapCode(nz),
            None => panic!("user trap code cannot be zero"),
        }
    }

    /// Returns the raw byte representing this trap.
    ///
    /// The returned byte is never zero and can be passed to `from_raw` later on
    /// to recreate the trap code.
    pub const fn as_raw(&self) -> u8 {
        self.0.get()
    }

    /// Creates a trap code from its raw byte, likely returned by
    /// [`TrapCode::as_raw`] previously.
    ///
    /// Returns `None` if the `byte` provided is zero.
    pub const fn from_raw(byte: u8) -> Option<TrapCode> {
        match NonZeroU8::new(byte) {
            Some(nz) => Some(TrapCode(nz)),
            None => None,
        }
    }

    /// Returns a slice of all traps except `TrapCode::User` traps
    pub const fn non_user_traps() -> &'static [TrapCode] {
        &[
            TrapCode::STACK_OVERFLOW,
            TrapCode::HEAP_OUT_OF_BOUNDS,
            TrapCode::INTEGER_OVERFLOW,
            TrapCode::INTEGER_DIVISION_BY_ZERO,
            TrapCode::BAD_CONVERSION_TO_INTEGER,
        ]
    }
}

impl Display for TrapCode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let identifier = match *self {
            Self::STACK_OVERFLOW => "stk_ovf",
            Self::HEAP_OUT_OF_BOUNDS => "heap_oob",
            Self::INTEGER_OVERFLOW => "int_ovf",
            Self::INTEGER_DIVISION_BY_ZERO => "int_divz",
            Self::BAD_CONVERSION_TO_INTEGER => "bad_toint",
            TrapCode(x) => return write!(f, "user{x}"),
        };
        f.write_str(identifier)
    }
}

impl FromStr for TrapCode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "stk_ovf" => Ok(Self::STACK_OVERFLOW),
            "heap_oob" => Ok(Self::HEAP_OUT_OF_BOUNDS),
            "int_ovf" => Ok(Self::INTEGER_OVERFLOW),
            "int_divz" => Ok(Self::INTEGER_DIVISION_BY_ZERO),
            "bad_toint" => Ok(Self::BAD_CONVERSION_TO_INTEGER),
            _ if s.starts_with("user") => s[4..].parse().map(TrapCode::user).map_err(|_| ()),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn display() {
        for r in TrapCode::non_user_traps() {
            let tc = *r;
            assert_eq!(tc.to_string().parse(), Ok(tc));
        }
        assert_eq!("bogus".parse::<TrapCode>(), Err(()));

        assert_eq!(TrapCode::user(17).to_string(), "user17");
        assert_eq!("user22".parse(), Ok(TrapCode::user(22)));
        assert_eq!("user".parse::<TrapCode>(), Err(()));
        assert_eq!("user-1".parse::<TrapCode>(), Err(()));
        assert_eq!("users".parse::<TrapCode>(), Err(()));
    }
}
