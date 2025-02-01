//! Manual impls of the `Arbitrary` trait for types throughout this crate.

use crate::{AsReg, Gpr, NonRspGpr, Registers, Simm32, Simm32PlusKnownOffset};
use arbitrary::{Arbitrary, Result, Unstructured};

impl Arbitrary<'_> for Simm32PlusKnownOffset {
    fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
        // For now, we don't generate offsets (TODO).
        Ok(Self {
            simm32: Simm32::arbitrary(u)?,
            offset: None,
        })
    }
}
impl<R: AsReg> Arbitrary<'_> for NonRspGpr<R> {
    fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
        use crate::reg::enc::*;
        let gpr = u.choose(&[
            RAX, RCX, RDX, RBX, RBP, RSI, RDI, R8, R9, R10, R11, R12, R13, R14, R15,
        ])?;
        Ok(Self::new(R::new(*gpr)))
    }
}
impl<'a, R: AsReg> Arbitrary<'a> for Gpr<R> {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        Ok(Self(R::new(u.int_in_range(0..=15)?)))
    }
}

/// Helper trait that's used to be the same as `Registers` except with an extra
/// `for<'a> Arbitrary<'a>` bound on all of the associated types.
pub trait RegistersArbitrary:
    Registers<ReadGpr: for<'a> Arbitrary<'a>, ReadWriteGpr: for<'a> Arbitrary<'a>>
{
}

impl<R> RegistersArbitrary for R
where
    R: Registers,
    R::ReadGpr: for<'a> Arbitrary<'a>,
    R::ReadWriteGpr: for<'a> Arbitrary<'a>,
{
}
