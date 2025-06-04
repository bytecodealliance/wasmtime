use core::fmt;
use std::ops::BitOr;

#[derive(PartialEq, Debug)]
pub enum CustomOperation {
    Visit,
    Display,
    None,
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

#[derive(PartialEq)]
pub struct Custom(pub Vec<CustomOperation>);

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

impl From<Option<CustomOperation>> for Custom {
    fn from(flag: Option<CustomOperation>) -> Self {
        Custom(flag.into_iter().collect())
    }
}

impl From<Vec<CustomOperation>> for Custom {
    fn from(operations: Vec<CustomOperation>) -> Self {
        Custom(operations)
    }
}
