//! Trap codes describing the reason for a trap.

use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

/// A trap code describing the reason for a trap.
///
/// All trap instructions have an explicit trap code.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum TrapCode {
    /// The current stack space was exhausted.
    ///
    /// On some platforms, a stack overflow may also be indicated by a segmentation fault from the
    /// stack guard page.
    StackOverflow,

    /// A `heap_addr` instruction detected an out-of-bounds error.
    ///
    /// Some out-of-bounds heap accesses are detected by a segmentation fault on the heap guard
    /// pages.
    HeapOutOfBounds,

    /// Other bounds checking error.
    OutOfBounds,

    /// Indirect call to a null table entry.
    IndirectCallToNull,

    /// Signature mismatch on indirect call.
    BadSignature,

    /// An integer arithmetic operation caused an overflow.
    IntegerOverflow,

    /// An integer division by zero.
    IntegerDivisionByZero,

    /// Failed float-to-int conversion.
    BadConversionToInteger,

    /// A user-defined trap code.
    User(u16),
}

impl Display for TrapCode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        use self::TrapCode::*;
        let identifier = match *self {
            StackOverflow => "stk_ovf",
            HeapOutOfBounds => "heap_oob",
            OutOfBounds => "oob",
            IndirectCallToNull => "icall_null",
            BadSignature => "bad_sig",
            IntegerOverflow => "int_ovf",
            IntegerDivisionByZero => "int_divz",
            BadConversionToInteger => "bad_toint",
            User(x) => return write!(f, "user{}", x),
        };
        f.write_str(identifier)
    }
}

impl FromStr for TrapCode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use self::TrapCode::*;
        match s {
            "stk_ovf" => Ok(StackOverflow),
            "heap_oob" => Ok(HeapOutOfBounds),
            "oob" => Ok(OutOfBounds),
            "icall_null" => Ok(IndirectCallToNull),
            "bad_sig" => Ok(BadSignature),
            "int_ovf" => Ok(IntegerOverflow),
            "int_divz" => Ok(IntegerDivisionByZero),
            "bad_toint" => Ok(BadConversionToInteger),
            _ if s.starts_with("user") => s[4..].parse().map(User).map_err(|_| ()),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::string::ToString;

    // Everything but user-defined codes.
    const CODES: [TrapCode; 8] = [
        TrapCode::StackOverflow,
        TrapCode::HeapOutOfBounds,
        TrapCode::OutOfBounds,
        TrapCode::IndirectCallToNull,
        TrapCode::BadSignature,
        TrapCode::IntegerOverflow,
        TrapCode::IntegerDivisionByZero,
        TrapCode::BadConversionToInteger,
    ];

    #[test]
    fn display() {
        for r in &CODES {
            let tc = *r;
            assert_eq!(tc.to_string().parse(), Ok(tc));
        }
        assert_eq!("bogus".parse::<TrapCode>(), Err(()));

        assert_eq!(TrapCode::User(17).to_string(), "user17");
        assert_eq!("user22".parse(), Ok(TrapCode::User(22)));
        assert_eq!("user".parse::<TrapCode>(), Err(()));
        assert_eq!("user-1".parse::<TrapCode>(), Err(()));
        assert_eq!("users".parse::<TrapCode>(), Err(()));
    }
}
