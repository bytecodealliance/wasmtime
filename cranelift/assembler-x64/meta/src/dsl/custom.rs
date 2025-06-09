use core::fmt;
use std::ops::BitOr;

#[derive(PartialEq, Debug)]
pub enum CustomOperation {
    Visit,
    Display,
    Encode,
}

impl fmt::Display for CustomOperation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl BitOr for CustomOperation {
    type Output = Custom;
    fn bitor(self, rhs: Self) -> Self::Output {
        assert_ne!(self, rhs, "duplicate custom operation: {self:?}");
        Custom(vec![self, rhs])
    }
}

impl BitOr<CustomOperation> for Custom {
    type Output = Custom;
    fn bitor(mut self, rhs: CustomOperation) -> Self::Output {
        assert!(
            !self.0.contains(&rhs),
            "duplicate custom operation: {rhs:?}"
        );
        self.0.push(rhs);
        self
    }
}

#[derive(PartialEq, Default)]
pub struct Custom(Vec<CustomOperation>);

impl Custom {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &CustomOperation> {
        self.0.iter()
    }

    pub fn contains(&self, operation: CustomOperation) -> bool {
        self.0.contains(&operation)
    }
}

impl fmt::Display for Custom {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" | ")
        )
    }
}

impl From<CustomOperation> for Custom {
    fn from(operation: CustomOperation) -> Self {
        Custom(vec![operation])
    }
}
