/// Joins strings, just like `.join(" ")` but with [`core::fmt::Display`]
pub(crate) struct DisplayJoined<'a, S: AsRef<str>>(pub &'static str, pub &'a [S]);

impl<'a, S: AsRef<str>> core::fmt::Display for DisplayJoined<'a, S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut sep = false;
        for s in self.1 {
            if sep {
                f.write_str(self.0)?;
            }
            sep = true;
            f.write_str(s.as_ref())?;
        }
        Ok(())
    }
}

/// Joins strings, just like `.join(" ")` but with [`core::fmt::Display`]
pub(crate) struct DisplayJoinedVec<S: AsRef<str>>(pub &'static str, pub Vec<S>);

impl<S: AsRef<str>> core::fmt::Display for DisplayJoinedVec<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        DisplayJoined(self.0, &self.1).fmt(f)
    }
}

pub(crate) trait DisplayJoinedVecExt<S: AsRef<str>> {
    /// Joins strings, just like `.join(" ")` but with [`core::fmt::Display`]
    fn display_join(self, sep: &'static str) -> DisplayJoinedVec<S>;
}
impl<S: AsRef<str>> DisplayJoinedVecExt<S> for Vec<S> {
    fn display_join(self, sep: &'static str) -> DisplayJoinedVec<S> {
        DisplayJoinedVec(sep, self)
    }
}
