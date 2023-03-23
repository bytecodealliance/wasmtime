use std::{backtrace, fmt::Debug};

use arbitrary::Arbitrary;

/// The control plane of chaos mode.
/// Please see the [crate-level documentation](crate).
///
/// **Clone liberally!** The chaos engine is reference counted.
#[derive(Debug, Clone)]
pub struct ControlPlane {
    data: Vec<bool>,
    is_noop: bool,
}

impl Arbitrary<'_> for ControlPlane {
    fn arbitrary<'a>(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(Self::new(u.arbitrary()?, false))
    }
}

impl ControlPlane {
    fn new(data: Vec<bool>, is_noop: bool) -> Self {
        Self {
            data: data,
            is_noop,
        }
    }

    /// TODO chaos: should be explained
    pub fn no_chaos() -> Self {
        Self::new(Vec::new(), false)
    }

    pub fn todo() -> Self {
        Self::new(Vec::new(), false)
    }
}

impl Default for ControlPlane {
    fn default() -> Self {
        Self::new(Vec::new(), true)
    }
}

impl ControlPlane {
    pub fn get_decision(&mut self) -> Option<bool> {
        if self.is_noop {
            //println!(
            //    "try to get a decision from a noop chaos engine at {} ",
            //    backtrace::Backtrace::force_capture()
            //);
            //None
            panic!("trying to get a decision from a noop chaos engine");
        } else {
            self.data.pop()
        }
    }
}
