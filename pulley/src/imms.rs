//! Immediates.

/// A PC-relative offset.
///
/// This is relative to the start of this offset's containing instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PcRelOffset(i32);

#[cfg(feature = "arbitrary")]
impl<'a> arbitrary::Arbitrary<'a> for PcRelOffset {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(Self(u.arbitrary()?))
    }
}

impl From<i32> for PcRelOffset {
    #[inline]
    fn from(offset: i32) -> Self {
        PcRelOffset(offset)
    }
}

impl From<PcRelOffset> for i32 {
    #[inline]
    fn from(offset: PcRelOffset) -> Self {
        offset.0
    }
}
