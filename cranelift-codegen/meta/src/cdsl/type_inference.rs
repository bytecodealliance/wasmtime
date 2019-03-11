use crate::cdsl::typevar::TypeVar;

pub enum Constraint {
    WiderOrEq(TypeVar, TypeVar),
}
